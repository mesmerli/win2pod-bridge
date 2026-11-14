//! Windows Bluetooth Device Enumerator for AirPods / Beats.
//!
//! Uses Win32 Bluetooth enumeration APIs from `bthprops.cpl` to query paired
//! AirPods and Beats devices and their current connection status.

use std::ffi::c_void;

#[repr(C)]
struct SYSTEMTIME {
    w_year: u16,
    w_month: u16,
    w_day_of_week: u16,
    w_day: u16,
    w_hour: u16,
    w_minute: u16,
    w_second: u16,
    w_milliseconds: u16,
}

#[repr(C)]
struct BLUETOOTH_DEVICE_INFO {
    dw_size: u32,
    address: u64,
    ul_class_of_device: u32,
    f_connected: i32,
    f_remembered: i32,
    f_authenticated: i32,
    st_last_seen: SYSTEMTIME,
    st_last_used: SYSTEMTIME,
    sz_name: [u16; 248],
}

#[repr(C)]
struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
    dw_size: u32,
    f_return_authenticated: i32,
    f_return_remembered: i32,
    f_return_unknown: i32,
    f_return_connected: i32,
    f_issue_inquiry: i32,
    c_timeout_multiplier: u8,
    _pad: [u8; 3],
    h_radio: *mut c_void,
}

type FnBluetoothFindFirstDevice = unsafe extern "system" fn(
    *const BLUETOOTH_DEVICE_SEARCH_PARAMS,
    *mut BLUETOOTH_DEVICE_INFO,
) -> *mut c_void;

type FnBluetoothFindNextDevice = unsafe extern "system" fn(
    *mut c_void,
    *mut BLUETOOTH_DEVICE_INFO,
) -> i32;

type FnBluetoothFindDeviceClose = unsafe extern "system" fn(*mut c_void) -> i32;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PairedAirPodsDevice {
    pub name: String,
    pub address: u64,
    pub is_connected: bool,
}

/// Enumerates all remembered/paired AirPods and Beats devices from the Windows Bluetooth stack.
pub fn enumerate_paired_airpods() -> Vec<PairedAirPodsDevice> {
    let mut results = Vec::new();
    unsafe {
        let bth = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(b"bthprops.cpl\0".as_ptr());
        if bth.is_null() {
            return results;
        }

        let find_first: Option<FnBluetoothFindFirstDevice> = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(bth, b"BluetoothFindFirstDevice\0".as_ptr())
        );
        let find_next: Option<FnBluetoothFindNextDevice> = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(bth, b"BluetoothFindNextDevice\0".as_ptr())
        );
        let find_close: Option<FnBluetoothFindDeviceClose> = std::mem::transmute(
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(bth, b"BluetoothFindDeviceClose\0".as_ptr())
        );

        if let (Some(find_first), Some(find_next), Some(find_close)) = (find_first, find_next, find_close) {
            let mut search_params: BLUETOOTH_DEVICE_SEARCH_PARAMS = std::mem::zeroed();
            search_params.dw_size = std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32;
            search_params.f_return_remembered = 1;
            search_params.f_return_authenticated = 1;
            search_params.f_return_connected = 1;

            let mut dev_info: BLUETOOTH_DEVICE_INFO = std::mem::zeroed();
            dev_info.dw_size = std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32;

            let h_find = find_first(&search_params, &mut dev_info);
            if !h_find.is_null() {
                loop {
                    let name_len = dev_info.sz_name.iter().position(|&c| c == 0).unwrap_or(dev_info.sz_name.len());
                    let name = String::from_utf16_lossy(&dev_info.sz_name[..name_len]);
                    let n_lower = name.to_lowercase();

                    if n_lower.contains("airpods") || n_lower.contains("beats") || n_lower.contains("pods") {
                        results.push(PairedAirPodsDevice {
                            name,
                            address: dev_info.address,
                            is_connected: dev_info.f_connected != 0,
                        });
                    }

                    dev_info = std::mem::zeroed();
                    dev_info.dw_size = std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
                    if find_next(h_find, &mut dev_info) == 0 {
                        break;
                    }
                }
                find_close(h_find);
            }
        }
        windows_sys::Win32::Foundation::FreeLibrary(bth);
    }
    results
}
