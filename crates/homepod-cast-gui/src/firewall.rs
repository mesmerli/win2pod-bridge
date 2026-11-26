//! Automatic Windows Firewall rule helper for standalone/portable editions.
//!
//! When running inside an MSIX package, Windows App Container automatically
//! manages firewall capabilities, so this module does nothing.
//!
//! When running as a standalone/portable executable, this module checks whether
//! an inbound firewall rule for `win2pod-engine.exe` exists. If not, it prompts
//! for one-click UAC elevation to add the rule silently without requiring the
//! user to manually type commands into a terminal.

use std::path::Path;

/// Check and ensure an inbound firewall rule exists for `win2pod-engine.exe`.
pub fn ensure_firewall_rule_for_engine(engine_path: &Path) {
    if crate::license::is_packaged_app() {
        tracing::debug!("Running in packaged MSIX container: skipping manual firewall rule configuration");
        return;
    }

    if is_engine_firewall_rule_active(engine_path) {
        tracing::debug!("Inbound firewall rule for win2pod-engine already configured");
        return;
    }

    tracing::info!("Inbound firewall rule missing for {:?}, requesting one-click UAC elevation...", engine_path);
    request_firewall_elevation(engine_path);
}

/// Query Windows Defender Firewall via `netsh` to check if a rule named "Win2Pod Engine" is already active.
fn is_engine_firewall_rule_active(engine_path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = std::process::Command::new("netsh")
            .args(["advfirewall", "firewall", "show", "rule", "name=Win2Pod Engine"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let p_str = engine_path.to_string_lossy();
                // Ensure the rule is for Win2Pod Engine and points to an engine executable
                if text.contains("Win2Pod Engine") && (text.contains(&*p_str) || text.contains("win2pod-engine.exe")) {
                    return true;
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = engine_path;

    false
}

/// Request UAC elevation to add the inbound firewall rule silently.
fn request_firewall_elevation(engine_path: &Path) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;

        let verb: Vec<u16> = "runas\0".encode_utf16().collect();
        let file: Vec<u16> = "netsh.exe\0".encode_utf16().collect();
        let param_str = format!(
            "advfirewall firewall add rule name=\"Win2Pod Engine\" dir=in action=allow program=\"{}\" enable=yes profile=any\0",
            engine_path.display()
        );
        let params: Vec<u16> = param_str.encode_utf16().collect();

        let res = ShellExecuteW(
            0 as _,
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            0, // SW_HIDE: run completely hidden
        );

        // ShellExecute returns HINSTANCE > 32 on success
        if (res as usize) > 32 {
            tracing::info!("Firewall elevation requested successfully");
        } else {
            tracing::warn!("User declined UAC or firewall elevation failed (code: {})", res as usize);
        }
    }
    #[cfg(not(windows))]
    let _ = engine_path;
}
