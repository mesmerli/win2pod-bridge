//! PolicyConfig COM Client for setting Windows Default Audio Endpoints.

use std::ffi::c_void;
use windows::core::{GUID, HRESULT, PCWSTR};
use windows::Win32::Media::Audio::{eConsole, eCommunications, eMultimedia, ERole};

const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
const IID_IPOLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

#[repr(C)]
struct IPolicyConfigVtbl {
    pub query_interface: usize,
    pub add_ref: usize,
    pub release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub get_mix_format: usize,
    pub get_device_format: usize,
    pub reset_device_format: usize,
    pub set_device_format: usize,
    pub get_processing_period: usize,
    pub set_processing_period: usize,
    pub get_share_mode: usize,
    pub set_share_mode: usize,
    pub get_property_value: usize,
    pub set_property_value: usize,
    pub set_default_endpoint: unsafe extern "system" fn(
        this: *mut c_void,
        wsz_device_id: PCWSTR,
        role: ERole,
    ) -> HRESULT,
}

#[repr(C)]
struct IPolicyConfig {
    lp_vtbl: *const IPolicyConfigVtbl,
}

#[link(name = "ole32")]
extern "system" {
    fn CoCreateInstance(
        rclsid: *const GUID,
        punkouter: *mut c_void,
        dwclscontext: u32,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT;
}

/// Validates that a target audio endpoint is currently active. If it has been disconnected
/// or unplugged, finds and returns the best available active playback endpoint (preferring non-standin).
fn resolve_active_render_device(target_id: &str) -> String {
    if target_id.is_empty() || target_id == "default" {
        return target_id.to_string();
    }
    if let Ok(enumerator) = wasapi::DeviceEnumerator::new() {
        if let Ok(dev) = enumerator.get_device(target_id) {
            if dev.get_state().ok() == Some(wasapi::DeviceState::Active) {
                return target_id.to_string();
            }
        }
        // target_id is disconnected or not active. Find an active fallback render device.
        if let Ok(col) = enumerator.get_device_collection(&wasapi::Direction::Render) {
            if let Ok(count) = col.get_nbr_devices() {
                let standin_id = crate::controller::get_standin_device_id();
                // 1st Priority: active non-standin device
                for i in 0..count {
                    if let Ok(dev) = col.get_device_at_index(i) {
                        if dev.get_state().ok() == Some(wasapi::DeviceState::Active) {
                            if let Ok(id) = dev.get_id() {
                                if Some(&id) != standin_id.as_ref() {
                                    tracing::warn!(
                                        "[PolicyConfig] Target device [{}] is not active (disconnected). Falling back to active device: [{}]",
                                        target_id, id
                                    );
                                    return id;
                                }
                            }
                        }
                    }
                }
                // 2nd Priority: any active device
                for i in 0..count {
                    if let Ok(dev) = col.get_device_at_index(i) {
                        if dev.get_state().ok() == Some(wasapi::DeviceState::Active) {
                            if let Ok(id) = dev.get_id() {
                                tracing::warn!(
                                    "[PolicyConfig] Target device [{}] is not active (disconnected). Falling back to active device: [{}]",
                                    target_id, id
                                );
                                return id;
                            }
                        }
                    }
                }
            }
        }
    }
    target_id.to_string()
}

/// Set the system default audio playback endpoint across all roles (Console, Multimedia, Communications).
/// Automatically validates endpoint liveness and falls back to an active device if the target is offline.
pub fn set_default_render_device(device_id: &str) -> anyhow::Result<()> {
    let raw_id = device_id.to_string();
    std::thread::spawn(move || {
        let _com = wasapi::initialize_mta();
        let id = resolve_active_render_device(&raw_id);
        unsafe {
            let mut raw_ptr: *mut c_void = std::ptr::null_mut();
            // CLSCTX_ALL = 0x17
            let hr = CoCreateInstance(
                &CLSID_POLICY_CONFIG_CLIENT,
                std::ptr::null_mut(),
                0x17,
                &IID_IPOLICY_CONFIG,
                &mut raw_ptr,
            );
            if hr.is_err() || raw_ptr.is_null() {
                anyhow::bail!("Failed to create IPolicyConfig COM instance: {:?}", hr);
            }

            let policy = &*(raw_ptr as *const IPolicyConfig);
            let vtbl = &*policy.lp_vtbl;

            let w_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
            let pcwstr = PCWSTR(w_id.as_ptr());

            for role in [eConsole, eMultimedia, eCommunications] {
                let res = (vtbl.set_default_endpoint)(raw_ptr, pcwstr, role);
                if res.is_err() {
                    tracing::warn!("SetDefaultEndpoint failed for role {:?}: {:?}", role, res);
                }
            }
            (vtbl.release)(raw_ptr);
            tracing::info!("[PolicyConfig] Successfully updated Windows default audio endpoint to [{}]", id);
            Ok(())
        }
    })
    .join()
    .unwrap_or_else(|_| anyhow::bail!("set_default_render_device worker thread panicked"))
}
