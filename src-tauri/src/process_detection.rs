//! Detect whether the upstream-CLI is currently running, and whether VS Code
//! has the upstream extension active. Best-effort using the `sysinfo` crate;
//! detection failure is treated as "nothing detected" (we never block a swap
//! on this).

use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct RunningClaudeCode {
    pub cli_processes: u32,
    pub vscode_with_extension: Vec<String>,
}

pub fn detect() -> RunningClaudeCode {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut cli = 0u32;
    let mut vscode_workspaces = Vec::new();
    for (_pid, p) in sys.processes() {
        let name = p.name().to_string_lossy().to_lowercase();
        let cmd: Vec<String> = p
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let cmd_joined = cmd.join(" ").to_lowercase();

        // Upstream CLI: process name "claude" / "claude.exe", not the VS Code helper.
        if (name == "claude" || name == "claude.exe")
            && !cmd_joined.contains("electron")
            && !cmd_joined.contains("vscode")
        {
            cli += 1;
            continue;
        }

        // VS Code with the upstream extension loaded.
        if (name.contains("code") || name.contains("electron"))
            && cmd_joined.contains("anthropic.claude-code")
        {
            // Workspace folder typically appears as a positional argument.
            if let Some(folder) = cmd
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-') && std::path::Path::new(a.as_str()).exists())
            {
                if !vscode_workspaces.contains(folder) {
                    vscode_workspaces.push(folder.clone());
                }
            } else {
                vscode_workspaces.push("(unknown workspace)".to_string());
            }
        }
    }

    RunningClaudeCode {
        cli_processes: cli,
        vscode_with_extension: vscode_workspaces,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_does_not_panic_and_returns_struct() {
        let r = detect();
        let _ = r.cli_processes;
        let _ = r.vscode_with_extension.len();
    }
}
