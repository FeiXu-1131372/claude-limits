//! Strict semver "is newer" comparison for the updater.
//!
//! We deliberately re-implement (rather than depend on the `semver` crate)
//! because the manifest only ever contains MAJOR.MINOR.PATCH for this app
//! — no prerelease, no build metadata — and pulling in 50KB of code for
//! one comparison would be silly.

pub fn is_newer(running: &str, candidate: &str) -> Result<bool, String> {
    let r = parse(running)?;
    let c = parse(candidate)?;
    Ok(c > r)
}

fn parse(s: &str) -> Result<(u32, u32, u32), String> {
    let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
    if parts.len() != 3 {
        return Err(format!("expected MAJOR.MINOR.PATCH, got {s:?}"));
    }
    let mut out = [0u32; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .parse::<u32>()
            .map_err(|e| format!("invalid version part {p:?}: {e}"))?;
    }
    Ok((out[0], out[1], out[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn detects_newer_patch() {
        assert_eq!(is_newer("0.1.0", "0.1.1"), Ok(true));
    }

    #[test]
    fn detects_newer_minor() {
        assert_eq!(is_newer("0.1.5", "0.2.0"), Ok(true));
    }

    #[test]
    fn detects_newer_major() {
        assert_eq!(is_newer("0.9.9", "1.0.0"), Ok(true));
    }

    #[test]
    fn rejects_same_version() {
        assert_eq!(is_newer("0.2.0", "0.2.0"), Ok(false));
    }

    #[test]
    fn rejects_older() {
        assert_eq!(is_newer("0.2.0", "0.1.9"), Ok(false));
    }

    #[test]
    fn handles_double_digit_minor() {
        // 0.10.0 > 0.9.0 — naive string compare would fail this
        assert_eq!(is_newer("0.9.0", "0.10.0"), Ok(true));
    }

    #[test]
    fn strips_leading_v() {
        assert_eq!(is_newer("v0.1.0", "v0.2.0"), Ok(true));
    }

    #[test]
    fn rejects_malformed_running() {
        assert!(is_newer("not-a-version", "0.2.0").is_err());
    }

    #[test]
    fn rejects_malformed_candidate() {
        assert!(is_newer("0.1.0", "0.2").is_err());
    }
}
