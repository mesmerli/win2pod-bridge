//! Custom Win32 Drag Cursors (✋ Grab and ✊ Grabbing)
//!
//! Windows does not provide built-in OEM cursors for open hand / grab or closed fist / grabbing
//! (unlike macOS and the Web). `winit` on Windows normally maps both `Grab` and `Grabbing`
//! to the system four-way cross `IDC_SIZEALL` (✢).
//!
//! This module loads custom 32x32 hand icons embedded from PNG assets, converts them into
//! native Win32 `HCURSOR` handles via `CreateIconIndirect`, and provides hooks for the
//! unified window subclass procedure to display the custom cursors smoothly during hover
//! and window drag operations.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleBitmap, DeleteObject, GetDC, ReleaseDC, SetBitmapBits,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, SetCursor, HCURSOR, ICONINFO,
};

const RAW_GRAB_PNG: &[u8] = include_bytes!("../assets/cursor_grab.png");
const RAW_GRABBING_PNG: &[u8] = include_bytes!("../assets/cursor_grabbing.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CustomCursorState {
    None = 0,
    Grab = 1,
    Grabbing = 2,
}

impl From<u8> for CustomCursorState {
    fn from(val: u8) -> Self {
        match val {
            1 => CustomCursorState::Grab,
            2 => CustomCursorState::Grabbing,
            _ => CustomCursorState::None,
        }
    }
}

static GRAB_CURSOR: OnceLock<Option<isize>> = OnceLock::new();
static GRABBING_CURSOR: OnceLock<Option<isize>> = OnceLock::new();

// Window-specific hover/dragging states (indexed by WindowKind: 1 = Main, 2 = MiniPlayer, 3 = Equalizer, 4 = Visualizer)
static WINDOW_CURSOR_STATE: [AtomicU8; 5] = [
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
];

static WINDOW_MOVING: [AtomicBool; 5] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];

/// Create a native Win32 HCURSOR from PNG bytes with specified hotspot.
fn create_cursor_from_png(png_bytes: &[u8], hotspot_x: u32, hotspot_y: u32) -> Option<isize> {
    let img = image::load_from_memory(png_bytes).ok()?;
    let rgba = img.to_rgba8();
    let width = rgba.width() as i32;
    let height = rgba.height() as i32;

    let mut bgra = rgba.into_raw();
    bgra.chunks_exact_mut(4).for_each(|chunk| chunk.swap(0, 2));

    unsafe {
        let hdc_screen = GetDC(std::ptr::null_mut());
        if hdc_screen.is_null() {
            return None;
        }
        let hbm_color = CreateCompatibleBitmap(hdc_screen, width, height);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);
        if hbm_color.is_null() {
            return None;
        }

        if SetBitmapBits(hbm_color, bgra.len() as u32, bgra.as_ptr() as *const _) == 0 {
            DeleteObject(hbm_color);
            return None;
        }

        // Mask bitmap according to Windows API specifications
        let mask_len = ((((width + 15) >> 4) << 1) * height) as usize;
        let mask_bits = vec![0xff_u8; mask_len];
        let hbm_mask = CreateBitmap(width, height, 1, 1, mask_bits.as_ptr() as *const _);
        if hbm_mask.is_null() {
            DeleteObject(hbm_color);
            return None;
        }

        let icon_info = ICONINFO {
            fIcon: 0, // 0 for cursor, 1 for icon
            xHotspot: hotspot_x,
            yHotspot: hotspot_y,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let hcursor = CreateIconIndirect(&icon_info as *const _);
        DeleteObject(hbm_color);
        DeleteObject(hbm_mask);

        if hcursor.is_null() {
            None
        } else {
            Some(hcursor as isize)
        }
    }
}

pub fn get_grab_cursor() -> Option<HCURSOR> {
    let val = GRAB_CURSOR
        .get_or_init(|| create_cursor_from_png(RAW_GRAB_PNG, 14, 14))
        .as_ref()?;
    Some(*val as HCURSOR)
}

pub fn get_grabbing_cursor() -> Option<HCURSOR> {
    let val = GRABBING_CURSOR
        .get_or_init(|| create_cursor_from_png(RAW_GRABBING_PNG, 14, 14))
        .as_ref()?;
    Some(*val as HCURSOR)
}

/// Set the custom cursor state for a given window header.
pub fn set_header_cursor_state(kind: crate::snapping::WindowKind, state: CustomCursorState) {
    let idx = (kind as usize).min(4);
    let old_state = WINDOW_CURSOR_STATE[idx].swap(state as u8, Ordering::Relaxed);
    if old_state != state as u8 {
        apply_cursor_if_active(kind);
    }
}

/// Mark whether a window is currently in a Win32 modal moving / dragging loop.
pub fn set_window_moving(kind: crate::snapping::WindowKind, is_moving: bool) {
    let idx = (kind as usize).min(4);
    WINDOW_MOVING[idx].store(is_moving, Ordering::Relaxed);
    if is_moving {
        if let Some(hcursor) = get_grabbing_cursor() {
            unsafe {
                SetCursor(hcursor);
            }
        }
    }
}

/// Immediately apply the current custom cursor for this window if one is active.
pub fn apply_cursor_if_active(kind: crate::snapping::WindowKind) {
    if let Some(hcursor) = current_custom_cursor(kind) {
        unsafe {
            SetCursor(hcursor);
        }
    }
}

/// Get the active custom cursor for a specific window kind, if any.
pub fn current_custom_cursor(kind: crate::snapping::WindowKind) -> Option<HCURSOR> {
    let idx = (kind as usize).min(4);
    if WINDOW_MOVING[idx].load(Ordering::Relaxed) {
        return get_grabbing_cursor();
    }

    match CustomCursorState::from(WINDOW_CURSOR_STATE[idx].load(Ordering::Relaxed)) {
        CustomCursorState::Grabbing => get_grabbing_cursor(),
        CustomCursorState::Grab => get_grab_cursor(),
        CustomCursorState::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_cursor_creation() {
        let grab = get_grab_cursor();
        assert!(grab.is_some(), "Grab cursor should be created successfully");
        assert!(!grab.unwrap().is_null(), "Grab HCURSOR should be non-null");

        let grabbing = get_grabbing_cursor();
        assert!(grabbing.is_some(), "Grabbing cursor should be created successfully");
        assert!(!grabbing.unwrap().is_null(), "Grabbing HCURSOR should be non-null");
    }

    #[test]
    fn test_custom_cursor_state_transitions() {
        let kind = crate::snapping::WindowKind::MiniPlayer;
        set_header_cursor_state(kind, CustomCursorState::None);
        set_window_moving(kind, false);
        assert!(current_custom_cursor(kind).is_none());

        set_header_cursor_state(kind, CustomCursorState::Grab);
        assert_eq!(current_custom_cursor(kind), get_grab_cursor());

        set_header_cursor_state(kind, CustomCursorState::Grabbing);
        assert_eq!(current_custom_cursor(kind), get_grabbing_cursor());

        // When moving, window moving overrides state
        set_header_cursor_state(kind, CustomCursorState::None);
        set_window_moving(kind, true);
        assert_eq!(current_custom_cursor(kind), get_grabbing_cursor());

        set_window_moving(kind, false);
        assert!(current_custom_cursor(kind).is_none());
    }
}
