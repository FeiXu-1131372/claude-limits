use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, specta::Type)]
pub struct PricingEntry {
    pub prefix: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_5m_per_mtok: f64,
    pub cache_1h_per_mtok: f64,
    /// Optional 1M-context tier (Sonnet 4 only at time of writing). When
    /// the per-call input-side context exceeds `above_tokens`, every rate
    /// in this block replaces the base rate for that call.
    #[serde(default)]
    pub tier: Option<PricingTier>,
}

#[derive(Debug, Clone, Deserialize, Serialize, specta::Type)]
pub struct PricingTier {
    pub above_tokens: u64,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_5m_per_mtok: f64,
    pub cache_1h_per_mtok: f64,
}

#[derive(Debug, Deserialize)]
struct PricingFile {
    pricing: Vec<PricingEntry>,
}

pub struct PricingTable {
    entries: Vec<PricingEntry>,
}

impl PricingTable {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).context("read pricing.json")?;
        Self::parse(&raw)
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let f: PricingFile = serde_json::from_str(raw)?;
        let mut entries = f.pricing;
        entries.sort_by_key(|e| std::cmp::Reverse(e.prefix.len()));
        Ok(Self { entries })
    }

    pub fn bundled() -> Result<Self> {
        let raw = include_str!("../../pricing.json");
        Self::parse(raw)
    }

    pub fn entries(&self) -> &[PricingEntry] {
        &self.entries
    }

    pub fn lookup(&self, model: &str) -> Option<&PricingEntry> {
        let lower = model.to_ascii_lowercase();
        // Strip the "claude-" vendor prefix so that both full API model IDs
        // ("claude-sonnet-4-6-20260115") and bare family names ("sonnet-4-6")
        // resolve correctly via starts_with on the pricing prefix.
        let needle = lower.strip_prefix("claude-").unwrap_or(&lower);
        self.entries.iter().find(|e| needle.starts_with(e.prefix.as_str()))
    }

    pub fn cost_for(
        &self,
        model: &str,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_5m: u64,
        cache_1h: u64,
    ) -> f64 {
        let Some(e) = self.lookup(model) else {
            return 0.0;
        };
        let m = 1_000_000.0;

        // For 1M-context models, Anthropic charges every input-side and
        // output token at the higher tier rate when the prompt's context
        // size exceeds the threshold (it's not a per-token split — the
        // whole call shifts up). Total context = input + cache_read +
        // cache_creation; that's what Claude's tokenizer counted.
        let context_size = input + cache_read + cache_5m + cache_1h;
        let (input_rate, output_rate, cr_rate, c5m_rate, c1h_rate) = match &e.tier {
            Some(t) if context_size > t.above_tokens => (
                t.input_per_mtok,
                t.output_per_mtok,
                t.cache_read_per_mtok,
                t.cache_5m_per_mtok,
                t.cache_1h_per_mtok,
            ),
            _ => (
                e.input_per_mtok,
                e.output_per_mtok,
                e.cache_read_per_mtok,
                e.cache_5m_per_mtok,
                e.cache_1h_per_mtok,
            ),
        };

        (input as f64) / m * input_rate
            + (output as f64) / m * output_rate
            + (cache_read as f64) / m * cr_rate
            + (cache_5m as f64) / m * c5m_rate
            + (cache_1h as f64) / m * c1h_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> PricingTable {
        PricingTable::bundled().unwrap()
    }

    #[test]
    fn longest_prefix_wins() {
        let tbl = t();
        let opus47 = tbl.lookup("claude-opus-4-7-20260115").unwrap();
        assert_eq!(opus47.prefix, "opus-4-7");
        let opus_generic = tbl.lookup("opus-7-5-future").unwrap();
        assert_eq!(opus_generic.prefix, "opus");
    }

    #[test]
    fn every_current_family_is_priced() {
        let tbl = t();
        for m in [
            "opus-4-7",
            "opus-4-6",
            "opus-4-5",
            "opus-4-1",
            "opus-4",
            "sonnet-4-6",
            "sonnet-4-5",
            "sonnet-4",
            "haiku-4-5",
            "haiku-3-5",
        ] {
            assert!(tbl.lookup(m).is_some(), "missing pricing for {m}");
        }
    }

    #[test]
    fn unknown_model_is_zero_cost_not_panic() {
        let tbl = t();
        assert_eq!(
            tbl.cost_for("completely-unknown-model", 100, 200, 0, 0, 0),
            0.0
        );
    }

    #[test]
    fn cost_math_matches_expected() {
        let tbl = t();
        // 100k input — well below Sonnet 4's 200k tier — pays the base
        // $3/MTok rate, so 100k tokens = $0.30.
        let c = tbl.cost_for("sonnet-4-6", 100_000, 0, 0, 0, 0);
        assert!((c - 0.30).abs() < 0.001, "got {c}");
    }

    /// Sonnet 4 with a small prompt — context is 100k, well below the 200k
    /// 1M-context threshold, so the base rate applies.
    #[test]
    fn tier_does_not_apply_below_threshold() {
        let tbl = t();
        // 100k context (input + cache_read), 1M output for round numbers.
        let c = tbl.cost_for("sonnet-4-6", 100_000, 1_000_000, 0, 0, 0);
        // 0.1 × $3 (input) + 1 × $15 (output) = $0.30 + $15.00 = $15.30
        assert!((c - 15.30).abs() < 0.001, "got {c}");
    }

    /// Same call but the prompt's input-side context crosses 200k — every
    /// rate in this call jumps to the tier rate (whole-call bump, not a
    /// split). This is the exact accuracy gap vs the old flat-rate calc.
    #[test]
    fn tier_applies_when_context_exceeds_threshold() {
        let tbl = t();
        // 250k cache_read pushes total context above 200k. Tiny new input,
        // 1M output for round numbers.
        let c = tbl.cost_for("sonnet-4-6", 0, 1_000_000, 250_000, 0, 0);
        // 0.25 × $0.60 (cache_read tier) + 1 × $22.50 (output tier)
        //  = $0.15 + $22.50 = $22.65
        // (vs the OLD flat calc: 0.25 × $0.30 + 1 × $15 = $15.075)
        assert!((c - 22.65).abs() < 0.001, "got {c}");
    }

    /// Threshold check sums all input-side buckets — cache_creation also
    /// contributes to the per-call context size.
    #[test]
    fn tier_threshold_sums_all_input_side_tokens() {
        let tbl = t();
        // 80k input + 80k cache_read + 80k cache_5m = 240k context > 200k.
        let c = tbl.cost_for("sonnet-4-6", 80_000, 0, 80_000, 80_000, 0);
        // 0.08 × $6 + 0.08 × $0.60 + 0.08 × $7.50 = $0.48 + $0.048 + $0.60 = $1.128
        assert!((c - 1.128).abs() < 0.001, "got {c}");
    }

    /// Models without a `tier` block (Opus, Haiku) keep the flat rate
    /// regardless of context size.
    #[test]
    fn flat_models_ignore_threshold() {
        let tbl = t();
        let small = tbl.cost_for("opus-4-1", 100_000, 0, 0, 0, 0);
        let huge = tbl.cost_for("opus-4-1", 500_000, 0, 0, 0, 0);
        assert!((small - 1.5).abs() < 0.001);
        assert!((huge - 7.5).abs() < 0.001);
        // Linear scaling — no tier kink.
        assert!((huge / small - 5.0).abs() < 0.001);
    }
}
