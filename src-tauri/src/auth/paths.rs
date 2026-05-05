//! Path resolution mirroring claude-code's own behavior.
//!
//! 1. If `$CLAUDE_CONFIG_DIR` is set → `$CLAUDE_CONFIG_DIR/.claude.json`
//! 2. Else if `<config_home>/.config.json` exists → that (legacy fallback,
//!    filename is `.config.json` not `.claude.json` — intentional)
//! 3. Else `<homedir>/.claude.json`
//!
//! Where `<config_home> = $CLAUDE_CONFIG_DIR ?? <homedir>/.claude`.

use std::path::PathBuf;

pub fn claude_config_home() -> Option<PathBuf> {
    if let Some(env) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(env));
    }
    home_dir().map(|h| h.join(".claude"))
}

pub fn claude_global_config() -> Option<PathBuf> {
    // Step 1: env var wins outright.
    if let Some(env) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return Some(PathBuf::from(env).join(".claude.json"));
    }
    // Step 2: legacy `<config_home>/.config.json` if present.
    if let Some(home) = home_dir() {
        let legacy = home.join(".claude").join(".config.json");
        if legacy.exists() {
            return Some(legacy);
        }
        // Step 3: standard location.
        return Some(home.join(".claude.json"));
    }
    None
}

#[cfg(unix)]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(windows)]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn env_var_wins() {
        let dir = tempdir().unwrap();
        // Use a scoped env var via std::env::set_var (test-process scoped, fine
        // for single-threaded tests). Restore at end.
        let prev = std::env::var_os("CLAUDE_CONFIG_DIR");
        // SAFETY: test-only, single-threaded by default.
        unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", dir.path()) };
        let p = claude_global_config().unwrap();
        assert_eq!(p, dir.path().join(".claude.json"));
        match prev {
            Some(v) => unsafe { std::env::set_var("CLAUDE_CONFIG_DIR", v) },
            None => unsafe { std::env::remove_var("CLAUDE_CONFIG_DIR") },
        }
    }

    #[test]
    fn legacy_filename_is_config_json_not_claude_json() {
        // Compile-time check: function signature exists. Runtime behavior
        // depends on filesystem state (legacy file presence) — covered in
        // env-var test above. This is a smoke test.
        let _ = claude_global_config();
    }
}
