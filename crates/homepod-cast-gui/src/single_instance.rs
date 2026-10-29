//! Single instance support for HomePod-Cast GUI.
//! Ensures only one instance runs at a time. When launched again,
//! signals the first instance to show its flyout panel and exits immediately.

use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
};

const MUTEX_NAME: &str = "Local\\Win2Pod_SingleInstance_Mutex\0";
const EVENT_NAME: &str = "Local\\Win2Pod_ShowFlyout_Event\0";
const ASFW_ANY: u32 = 0xFFFFFFFF;
const INFINITE: u32 = 0xFFFFFFFF;
const EVENT_MODIFY_STATE: u32 = 0x0002;

extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *mut std::ffi::c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> isize;

    fn CreateEventW(
        lpEventAttributes: *mut std::ffi::c_void,
        bManualReset: i32,
        bInitialState: i32,
        lpName: *const u16,
    ) -> isize;

    fn OpenEventW(
        dwDesiredAccess: u32,
        bInheritHandle: i32,
        lpName: *const u16,
    ) -> isize;

    fn SetEvent(hEvent: isize) -> i32;

    fn WaitForSingleObject(hHandle: isize, dwMilliseconds: u32) -> u32;

    fn CloseHandle(hObject: isize) -> i32;
}

static mut SHOW_EVENT_HANDLE: isize = 0;

/// Check if another instance is running.
/// If another instance exists and `wake_existing` is true: signals it to open its panel, focuses its window, and returns false.
/// If another instance exists and `wake_existing` is false (e.g. background startup): exits quietly without showing the window.
/// If this is the first instance: sets up the IPC mechanism and returns true.
pub fn check_or_wake_existing(wake_existing: bool) -> anyhow::Result<bool> {
    let mutex_w: Vec<u16> = MUTEX_NAME.encode_utf16().collect();
    let event_w: Vec<u16> = EVENT_NAME.encode_utf16().collect();

    unsafe {
        // Create or open the single instance mutex
        let mutex = CreateMutexW(null_mut(), 1, mutex_w.as_ptr());
        if mutex == 0 {
            return Ok(true); // Fallback: continue if mutex creation failed
        }

        if GetLastError() == ERROR_ALREADY_EXISTS {
            if wake_existing {
                // Allow the target process to bring itself to the foreground
                AllowSetForegroundWindow(ASFW_ANY);

                // Another instance is already running! Signal it to show its window.
                let event = OpenEventW(EVENT_MODIFY_STATE, 0, event_w.as_ptr());
                if event != 0 {
                    SetEvent(event);
                    CloseHandle(event);
                }

                // Direct Win32 fallback to bring the existing window to the front
                let title: Vec<u16> = "Win2Pod Bridge\0".encode_utf16().collect();
                let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                if !hwnd.is_null() {
                    ShowWindow(hwnd, SW_SHOW);
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                }
            } else {
                tracing::info!("Another instance is already running; startup/background mode requested, exiting silently without waking existing window");
            }

            CloseHandle(mutex);
            return Ok(false);
        }

        // We are the first instance! Create the named event for subsequent instances to signal.
        let event = CreateEventW(null_mut(), 0, 0, event_w.as_ptr());
        SHOW_EVENT_HANDLE = event;
        // The mutex handle is kept open for the lifetime of this process
    }

    Ok(true)
}

/// Spawns a background thread listening for the wake signal from any subsequent launch attempts.
pub fn spawn_wake_listener(egui_ctx: eframe::egui::Context) {
    let event = unsafe { SHOW_EVENT_HANDLE };
    if event == 0 {
        return;
    }

    std::thread::Builder::new()
        .name("single_instance_listener".into())
        .spawn(move || unsafe {
            while WaitForSingleObject(event, INFINITE) == 0 {
                crate::tray::show_and_focus_main_window(&egui_ctx);
            }
            CloseHandle(event);
        })
        .expect("Failed to spawn single instance listener thread");
}
