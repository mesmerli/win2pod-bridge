//! Genie S-Curve Effect (macOS 經典神燈精靈扭曲收縮效果)
//!
//! Based on Muelle's Genie mathematical model (S-curve funnel deformation & slide).
//! High-fidelity per-scanline (1px) subpixel anti-aliased fluid rasterization.
//! Fully decoupled visual overlay running on a background thread.
//! Does NOT touch main window HWND lifecycle or focus.

use std::sync::atomic::{AtomicBool, Ordering};

static ABORT_FLAG: AtomicBool = AtomicBool::new(false);

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

/// Trigger the macOS-style S-curve Genie effect using window screenshot bitmap
pub fn play_genie_s_curve_effect(
    hbmp_handle: usize,
    orig_hwnd: usize,
    start_x: i32,
    start_y: i32,
    start_w: i32,
    start_h: i32,
) {
    #[cfg(test)]
    {
        let _ = (hbmp_handle, orig_hwnd, start_x, start_y, start_w, start_h);
        return;
    }

    #[cfg(not(test))]
    {
        abort_genie_effect();

        std::thread::Builder::new()
            .name("genie-scurve-fx".into())
            .spawn(move || {
                run_genie_s_curve_animation(hbmp_handle, orig_hwnd, start_x, start_y, start_w, start_h);
            })
            .ok();
    }
}

#[cfg(not(test))]
fn run_genie_s_curve_animation(
    hbmp_handle: usize,
    orig_hwnd_val: usize,
    start_x: i32,
    start_y: i32,
    start_w: i32,
    start_h: i32,
) {
    use std::time::Instant;
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDIBits, SelectObject,
        AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
        DIB_RGB_COLORS, HBITMAP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow,
        FindWindowExW, FindWindowW, GetSystemMetrics, GetWindowLongW, GetWindowRect,
        RegisterClassExW, SetWindowLongW, SetWindowPos, ShowWindow,
        UpdateLayeredWindow,
        GWL_EXSTYLE, HWND_NOTOPMOST, SM_CXSCREEN, SM_CYSCREEN,
        SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
        ULW_ALPHA, WNDCLASSEXW, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    ABORT_FLAG.store(false, Ordering::SeqCst);

    let hide_orig_window = |hwnd_val: usize| {
        if hwnd_val != 0 {
            let hwnd = hwnd_val as HWND;
            unsafe {
                if windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0 {
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
                }
            }
        }
    };

    let mut orig_hidden = false;

    let hbmp_src: HBITMAP = hbmp_handle as HBITMAP;
    if hbmp_src.is_null() || start_w <= 10 || start_h <= 10 {
        hide_orig_window(orig_hwnd_val);
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

        // 2. Locate precise target tray coordinate
        let (tray_x, tray_y) = {
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
                let shell_tray = FindWindowW(SHELL_TRAY.as_ptr(), std::ptr::null());
                let mut pt = (0, 0);
                let mut found = false;
                if !shell_tray.is_null() {
                    let tray_notify = FindWindowExW(shell_tray, std::ptr::null_mut(), TRAY_NOTIFY.as_ptr(), std::ptr::null());
                    let target = if !tray_notify.is_null() { tray_notify } else { shell_tray };
                    let mut rc: RECT = std::mem::zeroed();
                    if GetWindowRect(target, &mut rc) != 0 {
                        // Notice: TrayNotifyWnd contains battery/wifi on the left and app icons on the right.
                        // Targeting the right side (near clock) instead of center hits the app tray icons accurately.
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
        };

        let genie = Genie {
            window: Edges {
                left: start_x as f32,
                top: start_y as f32,
                right: (start_x + start_w) as f32,
                bottom: (start_y + start_h) as f32,
            },
            slot: Edges {
                left: (tray_x - 14) as f32,
                top: (tray_y - 14) as f32,
                right: (tray_x + 14) as f32,
                bottom: (tray_y + 14) as f32,
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
            return;
        }

        let old_rnd = SelectObject(hdc_render, hbmp_render);
        // Note: Do NOT ShowWindow here yet! Wait until the 0-th frame is drawn and UpdateLayeredWindow is called.
        // This ensures the animation window immediately matches the original window without a blank/black frame flicker.

        let pixels = std::slice::from_raw_parts_mut(pixel_ptr as *mut u32, (win_w * win_h) as usize);

        let duration_ms = 700.0; // 700ms relaxed & elegant macOS Genie duration
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

        // Cleanup resources
        SelectObject(hdc_render, old_rnd);
        DeleteDC(hdc_render);
        DeleteObject(hbmp_render);
        DestroyWindow(hwnd);

        // Defer working set trimming until animation has completed
        crate::tray::trim_working_set();
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
}
