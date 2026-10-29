//! Windows Registry Autostart module for Win2Pod Bridge.
//! Manages registration in HKCU\Software\Microsoft\Windows\CurrentVersion\Run
//! to allow the application to launch silently into the system tray on Windows startup.

use std::ptr::{null, null_mut};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0";
const VALUE_NAME: &str = "Win2Pod\0";

fn run_subkey_w() -> Vec<u16> {
    RUN_SUBKEY.encode_utf16().collect()
}

fn value_name_w() -> Vec<u16> {
    VALUE_NAME.encode_utf16().collect()
}

/// Checks if Win2Pod is registered in the Windows Run key for the current user.
pub fn is_autostart_enabled() -> bool {
    let subkey = run_subkey_w();
    let val_name = value_name_w();
    let mut hkey: HKEY = std::ptr::null_mut();

    unsafe {
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        if status != 0 || hkey.is_null() {
            return false;
        }

        let mut data_type: u32 = 0;
        let mut data_len: u32 = 0;
        let query_res = RegQueryValueExW(
            hkey,
            val_name.as_ptr(),
            null(),
            &mut data_type,
            null_mut(),
            &mut data_len,
        );

        RegCloseKey(hkey);
        query_res == 0 && data_len > 0
    }
}

/// Enables or disables Win2Pod autostart in the Windows Run registry key.
pub fn set_autostart(enabled: bool) -> anyhow::Result<()> {
    let subkey = run_subkey_w();
    let val_name = value_name_w();
    let mut hkey: HKEY = std::ptr::null_mut();

    unsafe {
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_WRITE,
            &mut hkey,
        );
        if status != 0 || hkey.is_null() {
            anyhow::bail!("Failed to open Run registry key: error code {status}");
        }

        let res = if enabled {
            // Get current executable path
            let current_exe = std::env::current_exe()?;
            let exe_str = current_exe.to_string_lossy();
            // Start minimized to system tray on Windows boot
            let cmd_line = format!("\"{exe_str}\" --startup\0");
            let cmd_line_w: Vec<u16> = cmd_line.encode_utf16().collect();
            let byte_len = (cmd_line_w.len() * 2) as u32;

            let set_res = RegSetValueExW(
                hkey,
                val_name.as_ptr(),
                0,
                REG_SZ,
                cmd_line_w.as_ptr() as *const u8,
                byte_len,
            );
            if set_res != 0 {
                Err(anyhow::anyhow!("RegSetValueExW failed with code {set_res}"))
            } else {
                tracing::info!("Registered autostart in Windows Run key: {cmd_line}");
                Ok(())
            }
        } else {
            // Remove registry value if exists
            let del_res = RegDeleteValueW(hkey, val_name.as_ptr());
            // 2 = ERROR_FILE_NOT_FOUND, which is also fine when disabling
            if del_res != 0 && del_res != 2 {
                Err(anyhow::anyhow!("RegDeleteValueW failed with code {del_res}"))
            } else {
                tracing::info!("Removed autostart from Windows Run key");
                Ok(())
            }
        };

        RegCloseKey(hkey);
        res
    }
}
