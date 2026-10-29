//! Microsoft Store In-App Purchase and Trial License Management.
//!
//! Store ID: 9N9ZC9MB9SB3
//! Deep link: ms-windows-store://pdp/?productid=9N9ZC9MB9SB3

pub const STORE_PRODUCT_ID: &str = "9N9ZC9MB9SB3";
pub const STORE_DEEP_LINK: &str = "ms-windows-store://pdp/?productid=9N9ZC9MB9SB3";

extern "system" {
    fn GetCurrentPackageFamilyName(
        packagefamilyname_length: *mut u32,
        packagefamilyname: *mut u16,
    ) -> u32;
}

/// Check whether the current process is running inside an MSIX / AppX package
pub fn is_packaged_app() -> bool {
    let mut len = 0u32;
    let res = unsafe { GetCurrentPackageFamilyName(&mut len, std::ptr::null_mut()) };
    const APPMODEL_ERROR_NO_PACKAGE: u32 = 15700;
    res != APPMODEL_ERROR_NO_PACKAGE
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LicenseTier {
    /// Full perpetual purchased license
    Licensed,
    /// Active trial period with remaining days
    Trial { days_remaining: u32 },
    /// Trial has expired! Premium features locked.
    TrialExpired,
    /// Standalone / unpackaged development mode
    Unpackaged,
}

impl Default for LicenseTier {
    fn default() -> Self {
        Self::Unpackaged
    }
}

impl LicenseTier {
    /// Check whether the premium feature ("Auto-mute PC speakers on connect") is allowed.
    pub fn is_auto_mute_allowed(&self) -> bool {
        match self {
            LicenseTier::Licensed => true,
            LicenseTier::Trial { .. } => true,
            LicenseTier::Unpackaged => true,
            LicenseTier::TrialExpired => false,
        }
    }
}

/// Open the official Microsoft Store product purchase page for Win2Pod Bridge
pub fn open_store_product_page() {
    tracing::info!("Opening Microsoft Store purchase page: {STORE_DEEP_LINK}");
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let uri: Vec<u16> = format!("{STORE_DEEP_LINK}\0").encode_utf16().collect();
        ShellExecuteW(
            0 as _,
            verb.as_ptr(),
            uri.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
}

/// Query Windows.Services.Store for the current app license.
pub async fn check_store_license() -> LicenseTier {
    // 1. Developer override via environment variable for instant testing:
    //    WIN2POD_FORCE_LICENSE=trial_expired / trial / licensed
    if let Ok(force) = std::env::var("WIN2POD_FORCE_LICENSE") {
        match force.to_lowercase().as_str() {
            "expired" | "trial_expired" | "1" => {
                tracing::info!("License check: Overridden to TrialExpired via WIN2POD_FORCE_LICENSE");
                return LicenseTier::TrialExpired;
            }
            "trial" => {
                tracing::info!("License check: Overridden to Trial (7 days) via WIN2POD_FORCE_LICENSE");
                return LicenseTier::Trial { days_remaining: 7 };
            }
            "licensed" | "full" => {
                tracing::info!("License check: Overridden to Licensed via WIN2POD_FORCE_LICENSE");
                return LicenseTier::Licensed;
            }
            "unpackaged" => {
                tracing::info!("License check: Overridden to Unpackaged via WIN2POD_FORCE_LICENSE");
                return LicenseTier::Unpackaged;
            }
            _ => {}
        }
    }

    // 2. If running as a standalone unpackaged executable (outside MSIX package),
    //    it is not managed by Windows Store DRM.
    if !is_packaged_app() {
        tracing::info!("Process is running unpackaged (standalone win2pod.exe)");
        return LicenseTier::Unpackaged;
    }

    // 3. Query Windows.Services.Store WinRT API for packaged MSIX app
    #[cfg(windows)]
    {
        match windows::Services::Store::StoreContext::GetDefault() {
            Ok(context) => match context.GetAppLicenseAsync() {
                Ok(op) => match op.await {
                    Ok(license) => {
                        let is_active = license.IsActive().unwrap_or(false);
                        let is_trial = license.IsTrial().unwrap_or(false);

                        tracing::info!(
                            "Store license retrieved: IsActive={}, IsTrial={}",
                            is_active,
                            is_trial
                        );

                        if !is_active {
                            return LicenseTier::TrialExpired;
                        }

                        if is_trial {
                            if let Ok(remaining) = license.TrialTimeRemaining() {
                                let duration_100ns = remaining.Duration;
                                if duration_100ns <= 0 {
                                    tracing::warn!("Store trial has expired (duration <= 0)");
                                    return LicenseTier::TrialExpired;
                                }
                                // 1 second = 10,000,000 units
                                // 1 day = 864,000,000,000 units
                                let days = (duration_100ns / 864_000_000_000) as u32;
                                let days = days.max(1);
                                tracing::info!("Store trial active: {days} days remaining");
                                return LicenseTier::Trial { days_remaining: days };
                            }
                            return LicenseTier::TrialExpired;
                        }

                        LicenseTier::Licensed
                    }
                    Err(e) => {
                        tracing::debug!("GetAppLicenseAsync await returned error (likely unpackaged): {e}");
                        LicenseTier::Unpackaged
                    }
                },
                Err(e) => {
                    tracing::debug!("StoreContext::GetAppLicenseAsync call failed: {e}");
                    LicenseTier::Unpackaged
                }
            },
            Err(e) => {
                tracing::debug!("StoreContext::GetDefault failed (unpackaged mode): {e}");
                LicenseTier::Unpackaged
            }
        }
    }

    #[cfg(not(windows))]
    LicenseTier::Unpackaged
}
