//! Genie S-Curve Effect (macOS 經典神燈精靈扭曲收縮效果)
//!
//! Based on Muelle's Genie mathematical model (S-curve funnel deformation & slide).
//! High-fidelity per-scanline (1px) subpixel anti-aliased fluid rasterization.
//! Fully decoupled visual overlay running on a background thread.
//! Does NOT touch main window HWND lifecycle or focus.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

static ABORT_FLAG: AtomicBool = AtomicBool::new(false);

/// Cached snapshot of the main window for the reverse genie restore animation
#[derive(Clone)]
pub struct CachedGenieSnapshot {
    pub pixels: Vec<u32>,
    pub width: i32,
    pub height: i32,
}

static LAST_SNAPSHOT: std::sync::Mutex<Option<CachedGenieSnapshot>> = std::sync::Mutex::new(None);
static SNAPSHOTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, CachedGenieSnapshot>>,
> = std::sync::OnceLock::new();

fn get_snapshots() -> &'static std::sync::Mutex<std::collections::HashMap<usize, CachedGenieSnapshot>> {
    SNAPSHOTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub static LAST_TASKBAR_BUTTON_X: AtomicI32 = AtomicI32::new(i32::MIN);
pub static LAST_TASKBAR_BUTTON_Y: AtomicI32 = AtomicI32::new(i32::MIN);

pub fn record_taskbar_button_coord(x: i32, y: i32) {
    if x > 0 && y > 0 {
        LAST_TASKBAR_BUTTON_X.store(x, Ordering::SeqCst);
        LAST_TASKBAR_BUTTON_Y.store(y, Ordering::SeqCst);
    }
}

/// Query if a cached snapshot is available for reverse genie restore
pub fn has_cached_genie_snapshot() -> bool {
    if let Ok(guard) = LAST_SNAPSHOT.lock() {
        guard.is_some()
    } else {
        false
    }
}

pub fn has_cached_snapshot_for_hwnd(hwnd_val: usize) -> bool {
    if let Ok(map) = get_snapshots().lock() {
        if map.contains_key(&hwnd_val) {
            return true;
        }
    }
    if let Ok(guard) = LAST_SNAPSHOT.lock() {
        guard.is_some()
    } else {
        false
    }
}

/// Captures a 1:1 window screenshot bitmap into memory for Genie effect
pub unsafe fn capture_window_snapshot(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> (usize, windows_sys::Win32::Foundation::RECT) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, GetDC, ReleaseDC, SelectObject, SRCCOPY,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut rect);
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 50 || h <= 50 {
        return (0, rect);
    }
    let screen_dc = GetDC(std::ptr::null_mut());
    let mem_dc = CreateCompatibleDC(screen_dc);
    let hbmp = CreateCompatibleBitmap(screen_dc, w, h);
    let old_bmp = SelectObject(mem_dc, hbmp);
    BitBlt(mem_dc, 0, 0, w, h, screen_dc, rect.left, rect.top, SRCCOPY);
    SelectObject(mem_dc, old_bmp);
    DeleteDC(mem_dc);
    ReleaseDC(std::ptr::null_mut(), screen_dc);
    (hbmp as usize, rect)
}

/// Disables DWM transitions (minimize/restore animations) for this window so our custom
/// Genie animation has zero interference or flash with Windows DWM. (Follows Muelle `calm_windows`).
pub unsafe fn disable_window_dwm_transitions(hwnd: windows_sys::Win32::Foundation::HWND) {
    if !hwnd.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 {
        let disabled: i32 = 1;
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            windows_sys::Win32::Graphics::Dwm::DWMWA_TRANSITIONS_FORCEDISABLED as u32,
            &disabled as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

/// Immediately abort any running genie animation (e.g. when hotkey is pressed).
pub fn abort_genie_effect() {
    ABORT_FLAG.store(true, Ordering::SeqCst);
}

/// Screen rectangle by its edges (matches Muelle's core geometry model).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Genie {
    pub window: Edges,
    pub slot: Edges,
}

const BEND_END: f32 = 0.5;
const SLIDE_START: f32 = 0.15;

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Genie {
    #[must_use]
    pub fn bend(t: f32) -> f32 {
        smoothstep(t / BEND_END)
    }

    #[must_use]
    pub fn slide(t: f32) -> f32 {
        let x = ((t - SLIDE_START) / (1.0 - SLIDE_START)).clamp(0.0, 1.0);
        x * x
    }

    /// Calculate exact left and right boundary curves at screen height `y`.
    /// Follows Muelle's cosine S-curve formula for organic funnel aesthetics.
    pub fn sides(&self, y: f32, bend: f32) -> (f32, f32) {
        let span = self.slot.top - self.window.top;
        let along = if span > 0.0 {
            ((y - self.window.top) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let curve = (1.0 - (std::f32::consts::PI * along).cos()) / 2.0 * bend;
        (
            self.window.left + (self.slot.left - self.window.left) * curve,
            self.window.right + (self.slot.right - self.window.right) * curve,
        )
    }
}

/// Destination target for the macOS-style S-curve Genie effect
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenieTarget {
    /// Minimize to Taskbar (工作列): keeps Taskbar button, animates into taskbar button rect, SW_MINIMIZE
    Taskbar,
    /// Hide/Close to Tray (工作匣/系統匣): removes Taskbar button, animates into notification area tray icon
    Tray,
}

/// Trigger the macOS-style S-curve Genie effect using window screenshot bitmap
pub fn play_genie_s_curve_effect(
    hbmp_handle: usize,
    orig_hwnd: usize,
    start_x: i32,
    start_y: i32,
    start_w: i32,
    start_h: i32,
    target: GenieTarget,
) -> bool {
    play_genie_s_curve_effect_with_finish(
        hbmp_handle,
        orig_hwnd,
        start_x,
        start_y,
        start_w,
        start_h,
        target,
        || {},
    )
}

pub fn play_genie_s_curve_effect_with_finish<F>(
    hbmp_handle: usize,
    orig_hwnd: usize,
    start_x: i32,
    start_y: i32,
    start_w: i32,
    start_h: i32,
    target: GenieTarget,
    on_finish: F,
) -> bool
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(test)]
    {
        let _ = (hbmp_handle, orig_hwnd, start_x, start_y, start_w, start_h, target);
        on_finish();
        return true;
    }

    #[cfg(not(test))]
    {
        abort_genie_effect();

        std::thread::Builder::new()
            .name("genie-scurve-fx".into())
            .spawn(move || {
                run_genie_s_curve_animation(hbmp_handle, orig_hwnd, start_x, start_y, start_w, start_h, target, on_finish);
            })
            .ok();
        true
    }
}

#[cfg(not(test))]
fn run_genie_s_curve_animation<F>(
    hbmp_handle: usize,
    orig_hwnd_val: usize,
    start_x: i32,
    start_y: i32,
    start_w: i32,
    start_h: i32,
    target: GenieTarget,
    on_finish: F,
) where
    F: FnOnce() + Send + 'static, {
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDIBits, SelectObject,
        AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
        DIB_RGB_COLORS, HBITMAP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow,
        GetWindowLongW,
        RegisterClassExW, SetWindowLongW, SetWindowPos, ShowWindow,
        UpdateLayeredWindow,
        GWL_EXSTYLE, HWND_NOTOPMOST,
        SW_MINIMIZE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
        ULW_ALPHA, WNDCLASSEXW, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    ABORT_FLAG.store(false, Ordering::SeqCst);

    let hide_orig_window = |hwnd_val: usize| {
        if hwnd_val != 0 {
            let hwnd = hwnd_val as HWND;
            unsafe {
                if windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 {
                    if target == GenieTarget::Tray {
                        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
                        let new_ex = (ex | WS_EX_TOOLWINDOW as i32) & !(WS_EX_APPWINDOW as i32);
                        SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex);
                        crate::snapping::delete_taskbar_tab_pub(hwnd);
                        SetWindowPos(
                            hwnd,
                            HWND_NOTOPMOST,
                            -32000,
                            -32000,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    } else {
                        // Minimize to Taskbar:
                        // Win32 原生安全最小化：關閉 DWM 動畫後直接在原螢幕坐標處執行最小化！
                        // 讓 Win32 核心原生記錄正確的 (start_x, start_y, start_w, start_h) 為常態還原坐標，
                        // 絕不將視窗手動 SetWindowPos 移到離屏 (-32000, -32000)，從根本上徹底根除 DWM 縮圖毀損！
                        disable_window_dwm_transitions(hwnd);
                        ShowWindow(hwnd, SW_MINIMIZE);
                    }
                }
            }
        }
    };

    let mut orig_hidden = false;

    let hbmp_src: HBITMAP = hbmp_handle as HBITMAP;
    if hbmp_src.is_null() || start_w <= 10 || start_h <= 10 {
        hide_orig_window(orig_hwnd_val);
        on_finish();
        return;
    }

    unsafe {
        // 1. Extract source pixels from screenshot bitmap into memory buffer
        let src_w = start_w as usize;
        let src_h = start_h as usize;
        let mut src_pixels = vec![0u32; src_w * src_h];

        let screen_dc = windows_sys::Win32::Graphics::Gdi::GetDC(std::ptr::null_mut());
        let hdc_read = CreateCompatibleDC(screen_dc);

        let mut read_bmi: BITMAPINFO = std::mem::zeroed();
        read_bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        read_bmi.bmiHeader.biWidth = start_w;
        read_bmi.bmiHeader.biHeight = -start_h; // top-down
        read_bmi.bmiHeader.biPlanes = 1;
        read_bmi.bmiHeader.biBitCount = 32;
        read_bmi.bmiHeader.biCompression = BI_RGB;

        GetDIBits(
            hdc_read,
            hbmp_src,
            0,
            start_h as u32,
            src_pixels.as_mut_ptr() as *mut _,
            &mut read_bmi,
            DIB_RGB_COLORS,
        );
        DeleteDC(hdc_read);
        DeleteObject(hbmp_src); // Free original HBITMAP immediately

        // Cache snapshot for reverse genie restore
        let snap_obj = CachedGenieSnapshot {
            pixels: src_pixels.clone(),
            width: start_w,
            height: start_h,
        };
        if let Ok(mut map) = get_snapshots().lock() {
            map.insert(orig_hwnd_val, snap_obj.clone());
        }
        if let Ok(mut guard) = LAST_SNAPSHOT.lock() {
            *guard = Some(snap_obj);
        }

        // 2. Locate precise target coordinate (Tray or Taskbar)
        let (slot_x, slot_y) = match target {
            GenieTarget::Tray => locate_tray_coord(),
            GenieTarget::Taskbar => locate_taskbar_coord(orig_hwnd_val as HWND, start_x + start_w / 2),
        };

        let genie = Genie {
            window: Edges {
                left: start_x as f32,
                top: start_y as f32,
                right: (start_x + start_w) as f32,
                bottom: (start_y + start_h) as f32,
            },
            slot: Edges {
                left: (slot_x - 14) as f32,
                top: (slot_y - 14) as f32,
                right: (slot_x + 14) as f32,
                bottom: (slot_y + 14) as f32,
            },
        };

        // Bounding box covering start window and tray slot
        let min_x = (genie.window.left.min(genie.slot.left) - 40.0).floor() as i32;
        let max_x = (genie.window.right.max(genie.slot.right) + 40.0).ceil() as i32;
        let min_y = (genie.window.top.min(genie.slot.top) - 40.0).floor() as i32;
        let max_y = (genie.window.bottom.max(genie.slot.bottom) + 40.0).ceil() as i32;

        let win_w = (max_x - min_x).max(100);
        let win_h = (max_y - min_y).max(100);

        const CLASS_NAME: [u16; 17] = [
            b'G' as u16, b'e' as u16, b'n' as u16, b'i' as u16, b'e' as u16, b'S' as u16,
            b'C' as u16, b'u' as u16, b'r' as u16, b'v' as u16, b'e' as u16, b'C' as u16,
            b'l' as u16, b'a' as u16, b's' as u16, b'3' as u16, 0,
        ];

        let hinstance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(DefWindowProcW);
        wc.hInstance = hinstance;
        wc.lpszClassName = CLASS_NAME.as_ptr();
        RegisterClassExW(&wc);

        let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
        let hwnd: HWND = CreateWindowExW(
            ex_style,
            CLASS_NAME.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            min_x,
            min_y,
            win_w,
            win_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), screen_dc);
            on_finish();
            return;
        }

        // Render buffer DIBSection
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = win_w;
        bmi.bmiHeader.biHeight = -win_h; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut pixel_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hdc_render = CreateCompatibleDC(screen_dc);

        let hbmp_render: HBITMAP = CreateDIBSection(
            hdc_render,
            &bmi,
            DIB_RGB_COLORS,
            &mut pixel_ptr,
            std::ptr::null_mut(),
            0,
        );
        windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), screen_dc);

        if hbmp_render.is_null() || pixel_ptr.is_null() {
            DeleteDC(hdc_render);
            DestroyWindow(hwnd);
            on_finish();
            return;
        }

        let old_rnd = SelectObject(hdc_render, hbmp_render);
        // Note: Do NOT ShowWindow here yet! Wait until the 0-th frame is drawn and UpdateLayeredWindow is called.
        // This ensures the animation window immediately matches the original window without a blank/black frame flicker.

        let pixels = std::slice::from_raw_parts_mut(pixel_ptr as *mut u32, (win_w * win_h) as usize);

        let duration_ms = 240.0; // Option A: 240ms Muelle fast genie minimize duration
        let start_time = Instant::now();

        let win_size = SIZE { cx: win_w, cy: win_h };
        let pt_zero = POINT { x: 0, y: 0 };
        let pt_win = POINT { x: min_x, y: min_y };

        let orig_height = (genie.window.bottom - genie.window.top).max(1.0);

        while !ABORT_FLAG.load(Ordering::SeqCst) {
            let elapsed = start_time.elapsed().as_secs_f32() * 1000.0;
            if elapsed >= duration_ms {
                break;
            }

            let t = (elapsed / duration_ms).clamp(0.0, 1.0);
            let bend = Genie::bend(t);
            let shift = Genie::slide(t) * (genie.slot.bottom - genie.window.top).max(0.0);

            let screen_top = genie.window.top + shift;
            let screen_bottom = (genie.window.bottom + shift).min(genie.slot.bottom);

            if screen_top >= genie.slot.bottom {
                break; // Fully inside tray slot
            }

            // Clear destination canvas to fully transparent black
            pixels.fill(0);

            let start_local_y = ((screen_top - min_y as f32).floor() as i32).clamp(0, win_h - 1);
            let end_local_y = ((screen_bottom - min_y as f32).ceil() as i32).clamp(0, win_h - 1);

            let alpha_master = if t > 0.88 {
                ((1.0 - (t - 0.88) / 0.12) * 255.0).clamp(0.0, 255.0) as u32
            } else {
                255u32
            };

            // High-fidelity per-scanline (1px) subpixel anti-aliased fluid rasterization
            for local_y in start_local_y..=end_local_y {
                let screen_y = min_y as f32 + local_y as f32 + 0.5;
                if screen_y < screen_top || screen_y > screen_bottom {
                    continue;
                }

                let (screen_left, screen_right) = genie.sides(screen_y, bend);
                let span_w = screen_right - screen_left;
                if span_w <= 0.5 {
                    continue;
                }

                let local_left = screen_left - min_x as f32;
                let local_right = screen_right - min_x as f32;

                let ix_start = (local_left.floor() as i32).clamp(0, win_w - 1);
                let ix_end = (local_right.ceil() as i32).clamp(0, win_w - 1);

                let v = ((screen_y - screen_top) / orig_height).clamp(0.0, 0.9999);
                let src_y = (v * (src_h as f32)) as usize;
                let src_row_offset = src_y * src_w;
                let dst_row_offset = (local_y as usize) * (win_w as usize);

                for ix in ix_start..=ix_end {
                    let fx = ix as f32 + 0.5;
                    let u = (fx - local_left) / span_w;
                    if u < 0.0 || u > 1.0 {
                        continue;
                    }

                    // Subpixel edge anti-aliasing (smooth border like macOS CoreAnimation)
                    let mut coverage = 1.0f32;
                    if fx < local_left + 1.0 {
                        coverage = (fx + 0.5 - local_left).clamp(0.0, 1.0);
                    } else if fx > local_right - 1.0 {
                        coverage = (local_right - (fx - 0.5)).clamp(0.0, 1.0);
                    }
                    if coverage <= 0.01 {
                        continue;
                    }

                    let src_x = ((u * (src_w as f32)).clamp(0.0, (src_w - 1) as f32)) as usize;
                    let orig_pixel = src_pixels[src_row_offset + src_x];

                    let r = (orig_pixel >> 16) & 0xFF;
                    let g = (orig_pixel >> 8) & 0xFF;
                    let b = orig_pixel & 0xFF;

                    let a = ((coverage * (alpha_master as f32)).clamp(0.0, 255.0)) as u32;
                    if a == 0 {
                        continue;
                    }

                    // Premultiplied alpha calculation for UpdateLayeredWindow
                    let pr = (r * a / 255).min(255);
                    let pg = (g * a / 255).min(255);
                    let pb = (b * a / 255).min(255);

                    pixels[dst_row_offset + (ix as usize)] = (a << 24) | (pr << 16) | (pg << 8) | pb;
                }
            }

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            UpdateLayeredWindow(
                hwnd,
                std::ptr::null_mut(),
                &pt_win,
                &win_size,
                hdc_render,
                &pt_zero,
                0,
                &blend,
                ULW_ALPHA,
            );

            // Seamless handover: On the 0-th frame, the layered window has drawn the exact 1:1 window screenshot
            // and is positioned identically above the original window. Make it visible and immediately hide the
            // original window to eliminate any frame gap, blank flicker, or border recalculation hitch!
            if !orig_hidden {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                hide_orig_window(orig_hwnd_val);
                orig_hidden = true;
            }

            std::thread::sleep(std::time::Duration::from_millis(11));
        }

        // Safety fallback: ensure original window is hidden even if loop broke early (e.g. abort)
        if !orig_hidden {
            hide_orig_window(orig_hwnd_val);
        }

        if target == GenieTarget::Taskbar {
            let orig_h = orig_hwnd_val as HWND;
            if !orig_h.is_null() && windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(orig_h) != 0 {
                if windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(orig_h) == 0 {
                    disable_window_dwm_transitions(orig_h);
                    ShowWindow(orig_h, SW_MINIMIZE);
                }
            }
        } else {
            // Defer working set trimming until animation has completed
            crate::tray::schedule_trim_working_set();
        }

        // Cleanup resources: 母體完成離屏最小化後再銷毀神燈動畫分身視窗，確保視覺無縫閉環
        SelectObject(hdc_render, old_rnd);
        DeleteDC(hdc_render);
        DeleteObject(hbmp_render);
        DestroyWindow(hwnd);

        on_finish();
    }
}

#[cfg(not(test))]
fn is_taskbar_left_aligned() -> bool {
    unsafe {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ,
        };
        const SUBKEY: &[u16] = &[
            b'S' as u16, b'o' as u16, b'f' as u16, b't' as u16, b'w' as u16, b'a' as u16,
            b'r' as u16, b'e' as u16, b'\\' as u16, b'M' as u16, b'i' as u16, b'c' as u16,
            b'r' as u16, b'o' as u16, b's' as u16, b'o' as u16, b'f' as u16, b't' as u16,
            b'\\' as u16, b'W' as u16, b'i' as u16, b'n' as u16, b'd' as u16, b'o' as u16,
            b'w' as u16, b's' as u16, b'\\' as u16, b'C' as u16, b'u' as u16, b'r' as u16,
            b'r' as u16, b'e' as u16, b'n' as u16, b't' as u16, b'V' as u16, b'e' as u16,
            b'r' as u16, b's' as u16, b'i' as u16, b'o' as u16, b'n' as u16, b'\\' as u16,
            b'E' as u16, b'x' as u16, b'p' as u16, b'l' as u16, b'o' as u16, b'r' as u16,
            b'e' as u16, b'r' as u16, b'\\' as u16, b'A' as u16, b'd' as u16, b'v' as u16,
            b'a' as u16, b'n' as u16, b'c' as u16, b'e' as u16, b'd' as u16, 0,
        ];
        const VAL_NAME: &[u16] = &[
            b'T' as u16, b'a' as u16, b's' as u16, b'k' as u16, b'b' as u16, b'a' as u16,
            b'r' as u16, b'A' as u16, b'l' as u16, 0,
        ];

        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, SUBKEY.as_ptr(), 0, KEY_READ, &mut hkey) == 0 {
            let mut val: u32 = 0;
            let mut val_len: u32 = std::mem::size_of::<u32>() as u32;
            let mut val_type: u32 = 0;
            let status = RegQueryValueExW(
                hkey,
                VAL_NAME.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                &mut val as *mut u32 as *mut u8,
                &mut val_len,
            );
            RegCloseKey(hkey);
            if status == 0 {
                return val == 0;
            }
        }
        false
    }
}

#[cfg(not(test))]
fn locate_taskbar_coord(hwnd: windows_sys::Win32::Foundation::HWND, win_center_x: i32) -> (i32, i32) {
    unsafe {
        use windows_sys::Win32::Foundation::{POINT, RECT};
        use windows_sys::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowExW, FindWindowW, GetCursorPos, GetSystemMetrics, GetWindowRect,
            SM_CXSCREEN, SM_CYSCREEN,
        };

        // 1. Get monitor info for the window's monitor
        let monitor = if !hwnd.is_null() {
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
        } else {
            std::ptr::null_mut()
        };

        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let (mon_rect, work_rect) = if !monitor.is_null() && GetMonitorInfoW(monitor, &mut mi) != 0 {
            (mi.rcMonitor, mi.rcWork)
        } else {
            let sw = GetSystemMetrics(SM_CXSCREEN);
            let sh = GetSystemMetrics(SM_CYSCREEN);
            let mon = RECT { left: 0, top: 0, right: sw, bottom: sh };
            let work = RECT { left: 0, top: 0, right: sw, bottom: (sh - 48).max(0) };
            (mon, work)
        };

        // Taskbar Y is typically the center of the taskbar bar
        let taskbar_y = if work_rect.bottom < mon_rect.bottom {
            (work_rect.bottom + mon_rect.bottom) / 2
        } else if work_rect.top > mon_rect.top {
            (mon_rect.top + work_rect.top) / 2
        } else {
            mon_rect.bottom - 24
        };

        // 2. Check if current cursor is on this taskbar (e.g. user clicked restore/minimize on taskbar)
        let mut cur_pt: POINT = std::mem::zeroed();
        if GetCursorPos(&mut cur_pt) != 0 {
            let on_taskbar = (cur_pt.y >= work_rect.bottom && cur_pt.y <= mon_rect.bottom)
                || (work_rect.top > mon_rect.top && cur_pt.y <= work_rect.top);
            if on_taskbar && cur_pt.x >= mon_rect.left && cur_pt.x <= mon_rect.right {
                record_taskbar_button_coord(cur_pt.x, taskbar_y);
                return (cur_pt.x, taskbar_y);
            }
        }

        // 3. Check previously recorded taskbar button coordinate
        let rx = LAST_TASKBAR_BUTTON_X.load(Ordering::SeqCst);
        let ry = LAST_TASKBAR_BUTTON_Y.load(Ordering::SeqCst);
        if rx >= mon_rect.left && rx <= mon_rect.right && ry != i32::MIN {
            return (rx, taskbar_y);
        }

        // 4. Check if legacy Win10 task list (MSTaskListWClass) is present
        const SHELL_TRAY: [u16; 14] = [
            b'S' as u16, b'h' as u16, b'e' as u16, b'l' as u16, b'l' as u16, b'_' as u16,
            b'T' as u16, b'r' as u16, b'a' as u16, b'y' as u16, b'W' as u16, b'n' as u16,
            b'd' as u16, 0,
        ];
        const REBAR_CLASS: [u16; 14] = [
            b'R' as u16, b'e' as u16, b'B' as u16, b'a' as u16, b'r' as u16, b'W' as u16,
            b'i' as u16, b'n' as u16, b'd' as u16, b'o' as u16, b'w' as u16, b'3' as u16,
            b'2' as u16, 0,
        ];
        const TASKLIST_CLASS: [u16; 17] = [
            b'M' as u16, b'S' as u16, b'T' as u16, b'a' as u16, b's' as u16, b'k' as u16,
            b'L' as u16, b'i' as u16, b's' as u16, b't' as u16, b'W' as u16, b'C' as u16,
            b'l' as u16, b'a' as u16, b's' as u16, b's' as u16, 0,
        ];

        let shell_tray = FindWindowW(SHELL_TRAY.as_ptr(), std::ptr::null());
        if !shell_tray.is_null() {
            let rebar = FindWindowExW(shell_tray, std::ptr::null_mut(), REBAR_CLASS.as_ptr(), std::ptr::null());
            let tasklist = if !rebar.is_null() {
                FindWindowExW(rebar, std::ptr::null_mut(), TASKLIST_CLASS.as_ptr(), std::ptr::null())
            } else {
                FindWindowExW(shell_tray, std::ptr::null_mut(), TASKLIST_CLASS.as_ptr(), std::ptr::null())
            };

            if !tasklist.is_null() {
                let mut rc_task: RECT = std::mem::zeroed();
                if GetWindowRect(tasklist, &mut rc_task) != 0 && rc_task.right > rc_task.left + 40 {
                    let min_x = rc_task.left + 24;
                    let max_x = (rc_task.right - 24).max(min_x);
                    let clamped_x = win_center_x.clamp(min_x, max_x);
                    return (clamped_x, taskbar_y);
                }
            }
        }

        // 5. Modern Windows 11 Taskbar alignment
        let is_left = is_taskbar_left_aligned();
        let clamped_x = if is_left {
            // Left-aligned: Start (~48px) + Search (~260px) + TaskView/Widgets (~80px) = ~390px.
            // App icons start around mon_rect.left + 480. Clamped to safe range avoiding search box.
            let min_x = mon_rect.left + 480;
            let max_x = (mon_rect.left + 900).min(mon_rect.right - 100).max(min_x);
            win_center_x.clamp(min_x, max_x)
        } else {
            // Center-aligned: Buttons clustered around center of monitor
            let center_x = (mon_rect.left + mon_rect.right) / 2;
            let min_x = (center_x - 160).max(mon_rect.left + 100);
            let max_x = (center_x + 160).min(mon_rect.right - 100).max(min_x);
            win_center_x.clamp(min_x, max_x)
        };

        (clamped_x, taskbar_y)
    }
}

#[cfg(test)]
fn locate_taskbar_coord(_hwnd: windows_sys::Win32::Foundation::HWND, win_center_x: i32) -> (i32, i32) {
    let rx = LAST_TASKBAR_BUTTON_X.load(Ordering::SeqCst);
    let ry = LAST_TASKBAR_BUTTON_Y.load(Ordering::SeqCst);
    if rx != i32::MIN && ry != i32::MIN {
        return (rx, ry);
    }
    (win_center_x.clamp(200, 800), 1056)
}

#[cfg(not(test))]
fn locate_tray_coord() -> (i32, i32) {
    let px = crate::tray::TRAY_ICON_RECT_X.load(Ordering::SeqCst);
    let py = crate::tray::TRAY_ICON_RECT_Y.load(Ordering::SeqCst);

    if px != i32::MIN && py != i32::MIN {
        (px, py)
    } else {
        // Win32 Taskbar Fallback
        const SHELL_TRAY: [u16; 14] = [
            b'S' as u16, b'h' as u16, b'e' as u16, b'l' as u16, b'l' as u16, b'_' as u16,
            b'T' as u16, b'r' as u16, b'a' as u16, b'y' as u16, b'W' as u16, b'n' as u16,
            b'd' as u16, 0,
        ];
        const TRAY_NOTIFY: [u16; 14] = [
            b'T' as u16, b'r' as u16, b'a' as u16, b'y' as u16, b'N' as u16, b'o' as u16,
            b't' as u16, b'i' as u16, b'f' as u16, b'y' as u16, b'W' as u16, b'n' as u16,
            b'd' as u16, 0,
        ];
        unsafe {
            use windows_sys::Win32::Foundation::RECT;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                FindWindowExW, FindWindowW, GetSystemMetrics, GetWindowRect,
                SM_CXSCREEN, SM_CYSCREEN,
            };
            let shell_tray = FindWindowW(SHELL_TRAY.as_ptr(), std::ptr::null());
            let mut pt = (0, 0);
            let mut found = false;
            if !shell_tray.is_null() {
                let tray_notify = FindWindowExW(shell_tray, std::ptr::null_mut(), TRAY_NOTIFY.as_ptr(), std::ptr::null());
                let target = if !tray_notify.is_null() { tray_notify } else { shell_tray };
                let mut rc: RECT = std::mem::zeroed();
                if GetWindowRect(target, &mut rc) != 0 {
                    let target_x = (rc.right - 90).max(rc.left + 20);
                    pt = (target_x, (rc.top + rc.bottom) / 2);
                    found = true;
                }
            }
            if !found {
                let sw = GetSystemMetrics(SM_CXSCREEN);
                let sh = GetSystemMetrics(SM_CYSCREEN);
                (sw - 80, sh - 25)
            } else {
                pt
            }
        }
    }
}

/// Trigger the reverse macOS-style S-curve Genie effect (expanding from tray slot or taskbar back into window)
pub fn play_reverse_genie_s_curve_effect<F>(
    target_x: i32,
    target_y: i32,
    orig_hwnd: usize,
    source: GenieTarget,
    on_finish: F,
) -> bool
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(test)]
    {
        let _ = (target_x, target_y, orig_hwnd, source);
        on_finish();
        return true;
    }

    #[cfg(not(test))]
    {
        abort_genie_effect();

        let snapshot = {
            if let Ok(map) = get_snapshots().lock() {
                map.get(&orig_hwnd).cloned()
            } else {
                None
            }
        }.or_else(|| {
            if let Ok(guard) = LAST_SNAPSHOT.lock() {
                guard.clone()
            } else {
                None
            }
        });

        let Some(snap) = snapshot else {
            on_finish();
            return false;
        };

        std::thread::Builder::new()
            .name("rev-genie-fx".into())
            .spawn(move || {
                run_reverse_genie_animation(snap, target_x, target_y, orig_hwnd, source, on_finish);
            })
            .ok();

        true
    }
}

#[cfg(not(test))]
fn run_reverse_genie_animation<F>(
    snap: CachedGenieSnapshot,
    target_x: i32,
    target_y: i32,
    orig_hwnd_val: usize,
    source: GenieTarget,
    on_finish: F,
) where
    F: FnOnce() + Send + 'static,
{
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
        AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
        DIB_RGB_COLORS, HBITMAP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow,
        RegisterClassExW, ShowWindow,
        UpdateLayeredWindow,
        SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    ABORT_FLAG.store(false, Ordering::SeqCst);

    let (slot_x, slot_y) = match source {
        GenieTarget::Tray => locate_tray_coord(),
        GenieTarget::Taskbar => locate_taskbar_coord(orig_hwnd_val as HWND, target_x + snap.width / 2),
    };

    let start_w = snap.width;
    let start_h = snap.height;
    let src_w = start_w as usize;
    let src_h = start_h as usize;
    let src_pixels = snap.pixels;

    let genie = Genie {
        window: Edges {
            left: target_x as f32,
            top: target_y as f32,
            right: (target_x + start_w) as f32,
            bottom: (target_y + start_h) as f32,
        },
        slot: Edges {
            left: (slot_x - 14) as f32,
            top: (slot_y - 14) as f32,
            right: (slot_x + 14) as f32,
            bottom: (slot_y + 14) as f32,
        },
    };

    let min_x = (genie.window.left.min(genie.slot.left) - 40.0).floor() as i32;
    let max_x = (genie.window.right.max(genie.slot.right) + 40.0).ceil() as i32;
    let min_y = (genie.window.top.min(genie.slot.top) - 40.0).floor() as i32;
    let max_y = (genie.window.bottom.max(genie.slot.bottom) + 40.0).ceil() as i32;

    let win_w = (max_x - min_x).max(100);
    let win_h = (max_y - min_y).max(100);

    const CLASS_NAME: [u16; 17] = [
        b'G' as u16, b'e' as u16, b'n' as u16, b'i' as u16, b'e' as u16, b'S' as u16,
        b'C' as u16, b'u' as u16, b'r' as u16, b'v' as u16, b'e' as u16, b'C' as u16,
        b'l' as u16, b'a' as u16, b's' as u16, b'3' as u16, 0,
    ];

    unsafe {
        let hinstance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(DefWindowProcW);
        wc.hInstance = hinstance;
        wc.lpszClassName = CLASS_NAME.as_ptr();
        RegisterClassExW(&wc);

        let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
        let hwnd: HWND = CreateWindowExW(
            ex_style,
            CLASS_NAME.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            min_x,
            min_y,
            win_w,
            win_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            on_finish();
            return;
        }

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = win_w;
        bmi.bmiHeader.biHeight = -win_h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let screen_dc = windows_sys::Win32::Graphics::Gdi::GetDC(std::ptr::null_mut());
        let hdc_render = CreateCompatibleDC(screen_dc);
        let mut pixel_ptr: *mut std::ffi::c_void = std::ptr::null_mut();

        let hbmp_render: HBITMAP = CreateDIBSection(
            hdc_render,
            &bmi,
            DIB_RGB_COLORS,
            &mut pixel_ptr,
            std::ptr::null_mut(),
            0,
        );
        windows_sys::Win32::Graphics::Gdi::ReleaseDC(std::ptr::null_mut(), screen_dc);

        if hbmp_render.is_null() || pixel_ptr.is_null() {
            DeleteDC(hdc_render);
            DestroyWindow(hwnd);
            on_finish();
            return;
        }

        let old_rnd = SelectObject(hdc_render, hbmp_render);
        let pixels = std::slice::from_raw_parts_mut(pixel_ptr as *mut u32, (win_w * win_h) as usize);

        let duration_ms = 220.0; // Option A: 220ms Muelle fast restore duration
        let start_time = Instant::now();

        let win_size = SIZE { cx: win_w, cy: win_h };
        let pt_zero = POINT { x: 0, y: 0 };
        let pt_win = POINT { x: min_x, y: min_y };

        let orig_height = (genie.window.bottom - genie.window.top).max(1.0);
        let mut first_frame = true;

        let render_frame = |t_norm: f32, pixels: &mut [u32]| -> bool {
            let progress = (1.0 - t_norm).clamp(0.0, 1.0);
            let bend = Genie::bend(progress);
            let shift = Genie::slide(progress) * (genie.slot.bottom - genie.window.top).max(0.0);

            let screen_top = genie.window.top + shift;
            let screen_bottom = (genie.window.bottom + shift).min(genie.slot.bottom);

            if screen_top >= genie.slot.bottom {
                return false;
            }

            pixels.fill(0);

            let start_local_y = ((screen_top - min_y as f32).floor() as i32).clamp(0, win_h - 1);
            let end_local_y = ((screen_bottom - min_y as f32).ceil() as i32).clamp(0, win_h - 1);

            let alpha_master = if t_norm < 0.12 {
                ((t_norm / 0.12) * 255.0).clamp(0.0, 255.0) as u32
            } else {
                255u32
            };

            for local_y in start_local_y..=end_local_y {
                let screen_y = min_y as f32 + local_y as f32 + 0.5;
                if screen_y < screen_top || screen_y > screen_bottom {
                    continue;
                }

                let (screen_left, screen_right) = genie.sides(screen_y, bend);
                let span_w = screen_right - screen_left;
                if span_w <= 0.5 {
                    continue;
                }

                let local_left = screen_left - min_x as f32;
                let local_right = screen_right - min_x as f32;

                let ix_start = (local_left.floor() as i32).clamp(0, win_w - 1);
                let ix_end = (local_right.ceil() as i32).clamp(0, win_w - 1);

                let v = ((screen_y - screen_top) / orig_height).clamp(0.0, 0.9999);
                let src_y = (v * (src_h as f32)) as usize;
                let src_row_offset = src_y * src_w;
                let dst_row_offset = (local_y as usize) * (win_w as usize);

                for ix in ix_start..=ix_end {
                    let fx = ix as f32 + 0.5;
                    let u = (fx - local_left) / span_w;
                    if u < 0.0 || u > 1.0 {
                        continue;
                    }

                    let mut coverage = 1.0f32;
                    if fx < local_left + 1.0 {
                        coverage = (fx + 0.5 - local_left).clamp(0.0, 1.0);
                    } else if fx > local_right - 1.0 {
                        coverage = (local_right - (fx - 0.5)).clamp(0.0, 1.0);
                    }
                    if coverage <= 0.01 {
                        continue;
                    }

                    let src_x = ((u * (src_w as f32)).clamp(0.0, (src_w - 1) as f32)) as usize;
                    let orig_pixel = src_pixels[src_row_offset + src_x];

                    let r = (orig_pixel >> 16) & 0xFF;
                    let g = (orig_pixel >> 8) & 0xFF;
                    let b = orig_pixel & 0xFF;

                    let a = ((coverage * (alpha_master as f32)).clamp(0.0, 255.0)) as u32;
                    if a == 0 {
                        continue;
                    }

                    let pr = (r * a / 255).min(255);
                    let pg = (g * a / 255).min(255);
                    let pb = (b * a / 255).min(255);

                    pixels[dst_row_offset + (ix as usize)] = (a << 24) | (pr << 16) | (pg << 8) | pb;
                }
            }
            true
        };

        while !ABORT_FLAG.load(Ordering::SeqCst) {
            let elapsed = start_time.elapsed().as_secs_f32() * 1000.0;
            if elapsed >= duration_ms {
                break;
            }

            let t_norm = (elapsed / duration_ms).clamp(0.0, 1.0);
            if !render_frame(t_norm, pixels) {
                std::thread::sleep(std::time::Duration::from_millis(8));
                continue;
            }

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            UpdateLayeredWindow(
                hwnd,
                std::ptr::null_mut(),
                &pt_win,
                &win_size,
                hdc_render,
                &pt_zero,
                0,
                &blend,
                ULW_ALPHA,
            );

            if first_frame {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                first_frame = false;
            }

            std::thread::sleep(std::time::Duration::from_millis(11));
        }

        // Guaranteed Final Frame: If not aborted, render the 100% full unbent frame
        if !ABORT_FLAG.load(Ordering::SeqCst) {
            if render_frame(1.0, pixels) {
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                UpdateLayeredWindow(
                    hwnd,
                    std::ptr::null_mut(),
                    &pt_win,
                    &win_size,
                    hdc_render,
                    &pt_zero,
                    0,
                    &blend,
                    ULW_ALPHA,
                );
            }
        }

        // Handover: Animation reached target position, display the real main window
        on_finish();

        // Immediately hide and destroy the layered animation window so there is zero overlap latency
        ShowWindow(hwnd, SW_HIDE);
        SelectObject(hdc_render, old_rnd);
        DeleteDC(hdc_render);
        DeleteObject(hbmp_render);
        DestroyWindow(hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_muelle_genie_math() {
        let genie = Genie {
            window: Edges {
                left: 100.0,
                top: 100.0,
                right: 500.0,
                bottom: 700.0,
            },
            slot: Edges {
                left: 900.0,
                top: 1000.0,
                right: 932.0,
                bottom: 1032.0,
            },
        };

        assert_eq!(Genie::bend(0.0), 0.0);
        assert_eq!(Genie::bend(0.5), 1.0);
        assert_eq!(Genie::slide(0.0), 0.0);
        assert_eq!(Genie::slide(1.0), 1.0);

        let (left, right) = genie.sides(400.0, 0.5);
        assert!(left < right);
    }

    #[test]
    fn test_abort_flag() {
        abort_genie_effect();
        assert!(ABORT_FLAG.load(Ordering::SeqCst));
    }

    #[test]
    fn test_reverse_genie_flow() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let executed = Arc::new(AtomicBool::new(false));
        let exec_clone = executed.clone();
        let res = play_reverse_genie_s_curve_effect(100, 100, 0, GenieTarget::Taskbar, move || {
            exec_clone.store(true, Ordering::SeqCst);
        });
        assert!(res);
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_taskbar_coord() {
        LAST_TASKBAR_BUTTON_X.store(i32::MIN, Ordering::SeqCst);
        LAST_TASKBAR_BUTTON_Y.store(i32::MIN, Ordering::SeqCst);

        let (x, y) = locate_taskbar_coord(std::ptr::null_mut(), 500);
        assert_eq!(x, 500);
        assert_eq!(y, 1056);

        let (x_min, _) = locate_taskbar_coord(std::ptr::null_mut(), 50);
        assert_eq!(x_min, 200);

        let (x_max, _) = locate_taskbar_coord(std::ptr::null_mut(), 1500);
        assert_eq!(x_max, 800);

        record_taskbar_button_coord(678, 1040);
        let (rec_x, rec_y) = locate_taskbar_coord(std::ptr::null_mut(), 50);
        assert_eq!(rec_x, 678);
        assert_eq!(rec_y, 1040);

        LAST_TASKBAR_BUTTON_X.store(i32::MIN, Ordering::SeqCst);
        LAST_TASKBAR_BUTTON_Y.store(i32::MIN, Ordering::SeqCst);
    }
}
