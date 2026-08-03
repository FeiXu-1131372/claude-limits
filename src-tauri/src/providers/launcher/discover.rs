//! Finding a user-installed CLI from inside a GUI app.
//!
//! This exists because of a hard platform asymmetry in what `PATH` a windowed
//! process inherits.
//!
//! On Windows a GUI process inherits the full user + machine `PATH` from the
//! registry, so the npm global directory that carries `claude.cmd` is simply
//! there and a bare `which` works.
//!
//! On macOS it is not. An `.app` launched from Finder, Dock, Spotlight or
//! LaunchAgent is started by launchd, and launchd hands it the system default
//! `PATH=/usr/bin:/bin:/usr/sbin:/sbin` — measured, not assumed. The user's
//! shell `PATH` is built by `~/.zprofile` / `~/.zshrc`, which no GUI launch ever
//! sources. Every location Claude Code actually installs into is outside that
//! default: `~/.local/bin` (native installer), `/opt/homebrew/bin` (Homebrew on
//! Apple Silicon), `/usr/local/bin` (Homebrew on Intel, and plain Node), and
//! `~/.claude/local` (the `migrate-installer` layout).
//!
//! So `which("claude")` succeeded for the whole of development — where the app
//! is started from a terminal and inherits a real `PATH` — and failed for every
//! user of the shipped bundle. The launcher reported "could not find the
//! `claude` executable" and no session could ever start.
//!
//! Resolution is therefore layered, cheapest first:
//!
//! 1. The inherited `PATH`. Correct under `tauri dev` and whenever the app was
//!    started from a shell, and free.
//! 2. A fixed list of the locations these tools genuinely install into. Covers
//!    the overwhelming majority of real machines with a few `stat` calls.
//! 3. The login shell's own `PATH`, asked for directly. This is the only thing
//!    that can find a version-manager install (nvm, asdf, fnm) or a custom npm
//!    prefix, since those live under paths we cannot enumerate.
//!
//! Step 3 costs a subprocess, so it is reached only when the launch would
//! otherwise fail outright.

use std::path::{Path, PathBuf};

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Whether `p` is a file this process could actually exec.
///
/// The mode check is the point on unix: a *directory* named `claude`, or a
/// non-executable leftover, would otherwise be returned and then fail at spawn
/// time with a far less obvious error.
fn is_executable_file(p: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(p) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    // Windows has no exec bit; reaching a named `.cmd`/`.exe` is enough.
    #[cfg(not(unix))]
    {
        true
    }
}

/// First entry that is an executable file. Pure — the whole fallback layer is
/// testable against a temp dir without touching the process environment.
pub fn first_executable(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| is_executable_file(p)).cloned()
}

/// Every place a Claude Code install can legitimately live, in probe order.
///
/// Ordered by how current the install method is: the native installer first,
/// then the package managers, then the legacy local layout. A machine with two
/// installs gets the one the user most likely means.
#[cfg(target_os = "macos")]
pub fn claude_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = home() {
        // Native installer (`curl -fsSL claude.ai/install.sh`).
        v.push(h.join(".local/bin/claude"));
        // `claude migrate-installer` moves the npm install here.
        v.push(h.join(".claude/local/claude"));
        v.push(h.join(".bun/bin/claude"));
        v.push(h.join(".volta/bin/claude"));
        // A user-level npm prefix, the usual fix for EACCES on a global install.
        v.push(h.join(".npm-global/bin/claude"));
    }
    // Homebrew on Apple Silicon, then Intel — also where a pkg-installed Node
    // puts its global bins.
    v.push(PathBuf::from("/opt/homebrew/bin/claude"));
    v.push(PathBuf::from("/usr/local/bin/claude"));
    v
}

/// Windows GUI processes inherit the real `PATH`, so step 1 already succeeds
/// and there is nothing useful to hard-code.
#[cfg(not(target_os = "macos"))]
pub fn claude_candidates() -> Vec<PathBuf> {
    Vec::new()
}

/// Where the VS Code `code` CLI lives when the user has never run "Shell
/// Command: Install 'code' command in PATH".
///
/// The in-bundle path is the interesting one: it works even though nothing was
/// ever symlinked into `/usr/local/bin`, which is the state most VS Code
/// installs are actually in.
#[cfg(target_os = "macos")]
pub fn code_candidates() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/usr/local/bin/code"),
        PathBuf::from("/opt/homebrew/bin/code"),
    ];
    for app in [
        "/Applications/Visual Studio Code.app",
        "/Applications/Visual Studio Code - Insiders.app",
    ] {
        v.push(PathBuf::from(app).join("Contents/Resources/app/bin/code"));
    }
    if let Some(h) = home() {
        for app in [
            "Applications/Visual Studio Code.app",
            "Applications/Visual Studio Code - Insiders.app",
        ] {
            v.push(h.join(app).join("Contents/Resources/app/bin/code"));
        }
    }
    v
}

#[cfg(not(target_os = "macos"))]
pub fn code_candidates() -> Vec<PathBuf> {
    Vec::new()
}

/// Marks the `PATH` in the login shell's output so noise from the user's rc
/// files — version-manager banners, `fortune`, a stray `echo` — cannot be
/// mistaken for it.
const SENTINEL: &str = "__SWITCHBOARD_PATH__";

/// Pulls the marked `PATH` out of whatever the shell printed. Pure, so the
/// noisy-rc-file case is a test rather than a hope.
pub fn parse_shell_path(out: &str) -> Option<String> {
    let (_, rest) = out.split_once(SENTINEL)?;
    let (path, _) = rest.split_once(SENTINEL)?;
    let path = path.trim();
    (!path.is_empty()).then(|| path.to_string())
}

/// The `PATH` the user's login shell builds, which is the only place a
/// version-manager install can be discovered.
///
/// `-ilc` is deliberate. A *login* shell alone reads `~/.zprofile` but not
/// `~/.zshrc`, and zsh users overwhelmingly extend `PATH` in `.zshrc` — dropping
/// `-i` would miss the common case this layer exists to catch.
///
/// Cached because it cannot change for the life of the process, and because an
/// interactive shell start is the one genuinely slow step in this module. The
/// *`PATH`* is cached rather than the resolution, so a CLI installed after the
/// first probe is still found.
#[cfg(unix)]
fn login_shell_path() -> Option<String> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Option<String>> = OnceLock::new();
    CACHE.get_or_init(query_login_shell).clone()
}

#[cfg(not(unix))]
fn login_shell_path() -> Option<String> {
    None
}

/// Runs the shell on a worker thread and gives up after five seconds.
///
/// An interactive shell with no tty is the failure mode to defend against: an
/// rc file that waits on input, or a version manager that probes the network,
/// would otherwise wedge the launch button forever. On timeout the thread is
/// abandoned rather than joined — the shell is short-lived and will exit on its
/// own, and this path is only reached when the launch was already failing.
#[cfg(unix)]
fn query_login_shell() -> Option<String> {
    use std::sync::mpsc;
    use std::time::Duration;

    let shell = std::env::var("SHELL").ok().filter(|s| !s.is_empty())?;
    let script = format!("printf '{SENTINEL}%s{SENTINEL}' \"$PATH\"");

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let out = std::process::Command::new(&shell)
            .args(["-ilc", &script])
            // An rc file that reads stdin blocks forever otherwise.
            .stdin(std::process::Stdio::null())
            .output();
        let _ = tx.send(out);
    });

    let out = rx.recv_timeout(Duration::from_secs(5)).ok()?.ok()?;
    // Not `status.success()`: an interactive shell with no tty frequently exits
    // non-zero after having printed a perfectly good PATH.
    parse_shell_path(&String::from_utf8_lossy(&out.stdout))
}

/// Resolve `binary`, trying the inherited `PATH`, then `candidates`, then the
/// login shell's `PATH`.
pub fn find(binary: &str, candidates: &[PathBuf]) -> Option<PathBuf> {
    if let Ok(p) = which::which(binary) {
        return Some(p);
    }
    if let Some(p) = first_executable(candidates) {
        return Some(p);
    }
    let path = login_shell_path()?;
    which::which_in(binary, Some(path), std::env::current_dir().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    fn write_exec(dir: &Path, name: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join(name);
        std::fs::write(&p, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn missing_candidates_resolve_to_nothing() {
        assert_eq!(
            first_executable(&[PathBuf::from("/definitely/not/here/claude")]),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_first_executable_candidate_wins() {
        let dir = tempdir().unwrap();
        let second = write_exec(dir.path(), "second");
        let found = first_executable(&[
            dir.path().join("missing"),
            second.clone(),
            write_exec(dir.path(), "third"),
        ]);
        assert_eq!(found, Some(second), "probe order must be preserved");
    }

    /// A non-executable file would be returned and then fail at spawn with a
    /// much less obvious error than "not found".
    #[cfg(unix)]
    #[test]
    fn a_non_executable_file_is_not_a_candidate() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("claude");
        std::fs::write(&p, "not executable").unwrap();
        assert_eq!(first_executable(&[p]), None);
    }

    /// A directory named `claude` sits on `~/.claude` for every single user of
    /// this app, so this is the realistic collision, not a contrived one.
    #[cfg(unix)]
    #[test]
    fn a_directory_is_not_a_candidate() {
        let dir = tempdir().unwrap();
        let d = dir.path().join("claude");
        std::fs::create_dir(&d).unwrap();
        assert_eq!(first_executable(&[d]), None);
    }

    /// The regression this module exists for: none of the places Claude Code
    /// installs into are on the `PATH` launchd gives a bundled `.app`.
    #[cfg(target_os = "macos")]
    #[test]
    fn candidates_cover_the_installs_that_launchd_path_misses() {
        let c = claude_candidates();
        let launchd_path = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];
        for expected in [
            "/.local/bin/claude",
            "/.claude/local/claude",
            "/opt/homebrew/bin/claude",
            "/usr/local/bin/claude",
        ] {
            assert!(
                c.iter().any(|p| p.to_string_lossy().ends_with(expected)),
                "{expected} must be probed; got {c:?}"
            );
        }
        for p in &c {
            let parent = p.parent().unwrap().to_string_lossy().to_string();
            assert!(
                !launchd_path.contains(&parent.as_str()),
                "{p:?} is already on the inherited PATH — step 1 covers it"
            );
        }
    }

    /// Most VS Code installs never run the "Install 'code' command in PATH"
    /// command, so the in-bundle CLI is the one that actually has to be found.
    #[cfg(target_os = "macos")]
    #[test]
    fn code_is_probed_inside_the_application_bundle() {
        let c = code_candidates();
        assert!(
            c.contains(&PathBuf::from(
                "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"
            )),
            "got {c:?}"
        );
    }

    /// The end-to-end regression, run against the real machine rather than a
    /// fixture. Ignored by default because it needs Claude Code installed, and
    /// it is only meaningful when run the way the bug actually happened:
    ///
    /// ```sh
    /// env -i HOME="$HOME" SHELL="$SHELL" PATH=/usr/bin:/bin:/usr/sbin:/sbin \
    ///   cargo test -- --ignored resolves_claude_under_the_launchd_path
    /// ```
    ///
    /// That `PATH` is not invented — it was read out of the environment block
    /// of the shipped `.app` while it was running.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "needs Claude Code installed; run under a launchd-shaped PATH"]
    fn resolves_claude_under_the_launchd_path() {
        let found = super::find("claude", &claude_candidates())
            .expect("claude must resolve even when PATH holds nothing but the system dirs");
        assert!(is_executable_file(&found), "{found:?} is not executable");
    }

    #[test]
    fn shell_path_is_read_from_between_the_sentinels() {
        let out = format!("{SENTINEL}/opt/homebrew/bin:/usr/bin{SENTINEL}");
        assert_eq!(
            parse_shell_path(&out).as_deref(),
            Some("/opt/homebrew/bin:/usr/bin")
        );
    }

    /// Version-manager banners and motd noise print around our output. The
    /// sentinels are what make that survivable.
    #[test]
    fn rc_file_noise_around_the_path_is_ignored() {
        let out = format!(
            "Now using node v22.3.0\n{SENTINEL}/Users/x/.nvm/versions/node/v22.3.0/bin:/usr/bin{SENTINEL}\nhave a nice day\n"
        );
        assert_eq!(
            parse_shell_path(&out).as_deref(),
            Some("/Users/x/.nvm/versions/node/v22.3.0/bin:/usr/bin")
        );
    }

    #[test]
    fn output_without_sentinels_is_not_a_path() {
        assert_eq!(parse_shell_path("/usr/bin:/bin"), None);
        assert_eq!(parse_shell_path(""), None);
    }

    /// A shell that printed nothing between the markers must not be read as a
    /// valid empty `PATH`, which `which_in` would happily search.
    #[test]
    fn an_empty_path_between_sentinels_is_rejected() {
        assert_eq!(parse_shell_path(&format!("{SENTINEL}{SENTINEL}")), None);
        assert_eq!(parse_shell_path(&format!("{SENTINEL}  \n {SENTINEL}")), None);
    }
}
