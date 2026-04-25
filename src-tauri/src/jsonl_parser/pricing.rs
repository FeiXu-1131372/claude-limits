use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PricingEntry {
    pub prefix: String,
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
        entries.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));
        Ok(Self { entries })
    }

    pub fn bundled() -> Result<Self> {
        let raw = include_str!("../../pricing.json");
        Self::parse(raw)
    }

    pub fn lookup(&self, model: &str) -> Option<&PricingEntry> {
        let needle = model.to_ascii_lowercase();
        self.entries.iter().find(|e| needle.contains(&e.prefix))
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
        (input as f64) / m * e.input_per_mtok
            + (output as f64) / m * e.output_per_mtok
            + (cache_read as f64) / m * e.cache_read_per_mtok
            + (cache_5m as f64) / m * e.cache_5m_per_mtok
            + (cache_1h as f64) / m * e.cache_1h_per_mtok
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
        let c = tbl.cost_for("sonnet-4-6", 1_000_000, 0, 0, 0, 0);
        assert!((c - 3.0).abs() < 0.001);
    }
}
