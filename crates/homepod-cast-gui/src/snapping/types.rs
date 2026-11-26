use std::collections::HashMap;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::*;
pub const MAIN_WINDOW_SUBCLASS_ID: usize = 0x504F44; // "POD"
pub const EQUALIZER_WINDOW_SUBCLASS_ID: usize = 0x504F45; // "POE"
pub const MINI_PLAYER_WINDOW_SUBCLASS_ID: usize = 0x504F4D; // "POM"

// HWND storage for standalone viewports
pub(crate) static EQUALIZER_HWND: AtomicIsize = AtomicIsize::new(0);
pub(crate) static MINI_PLAYER_HWND: AtomicIsize = AtomicIsize::new(0);


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowKind {
    Main = 1,
    MiniPlayer = 2,
    Equalizer = 3,
    Visualizer = 4,
}

impl WindowKind {
    pub fn priority(&self) -> u32 {
        *self as u32
    }

    /// Whether this window consumes/displays real-time audio spectrum or VU meter.
    pub fn consumes_spectrum(&self) -> bool {
        match self {
            WindowKind::Main | WindowKind::MiniPlayer | WindowKind::Visualizer => true,
            WindowKind::Equalizer => false,
        }
    }

    /// Returns the embedded page in Main Window corresponding to this standalone window (if any).
    pub fn embedded_page(&self) -> Option<crate::gui::Page> {
        match self {
            WindowKind::MiniPlayer => Some(crate::gui::Page::Player),
            WindowKind::Equalizer => Some(crate::gui::Page::Equalizer),
            WindowKind::Main | WindowKind::Visualizer => None,
        }
    }

    /// 獨立視窗唯一的 Viewport ID 字串
    pub fn viewport_id_str(&self) -> Option<&'static str> {
        match self {
            WindowKind::Equalizer => Some("equalizer_window"),
            WindowKind::MiniPlayer => Some("mini_player_window"),
            _ => None,
        }
    }

    /// 視窗預設尺寸 [寬, 高]（迷你播放器與迷你等化器預設高度皆精準為主視窗 680px 的一半：340px）
    pub fn default_size(&self) -> [f32; 2] {
        match self {
            WindowKind::Equalizer => [460.0, 340.0],
            WindowKind::MiniPlayer => [460.0, 340.0],
            WindowKind::Main => [460.0, 680.0],
            WindowKind::Visualizer => [720.0, 680.0],
        }
    }

    /// 視窗最小邊界限制 [寬, 高]
    pub fn min_size(&self) -> [f32; 2] {
        match self {
            WindowKind::Equalizer => [460.0, 340.0],
            WindowKind::MiniPlayer => [460.0, 295.0],
            WindowKind::Main => [460.0, 500.0],
            WindowKind::Visualizer => [500.0, 400.0],
        }
    }

    /// 根據視窗標題比對此視窗是否吻合（涵蓋多國語言規格）
    pub fn matches_window_title(&self, title: &str) -> bool {
        match self {
            WindowKind::Main => title == "Win2Pod" || title == "Win2Pod Bridge",
            WindowKind::Visualizer => title.contains("Visualizer") || title.contains("視覺") || title.contains("Paint Pulse"),
            WindowKind::Equalizer => {
                crate::i18n::ALL_LANGUAGES.iter().any(|l| l.eq_window_title() == title || l.mini_equalizer_window_title() == title)
                    || title.contains("Equalizer")
                    || title.contains("等化器")
                    || title.contains("均衡器")
                    || title.contains("Égaliseur")
                    || title.contains("イコライザー")
            }
            WindowKind::MiniPlayer => {
                crate::i18n::ALL_LANGUAGES.iter().any(|l| l.mini_player_window_title() == title)
                    || title.contains("Mini Player")
                    || title.contains("迷你播放器")
                    || title.contains("Mini Lecteur")
                    || title.contains("ミニプレーヤー")
            }
        }
    }
}

pub fn set_equalizer_hwnd(hwnd: HWND) {
    EQUALIZER_HWND.store(hwnd as isize, Ordering::SeqCst);
    if !hwnd.is_null() {
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.register_window(WindowKind::Equalizer, hwnd);
        });
    } else {
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.unregister_window(WindowKind::Equalizer);
        });
    }
}

pub fn get_equalizer_hwnd() -> HWND {
    get_window_hwnd(WindowKind::Equalizer)
}

pub fn set_mini_player_hwnd(hwnd: HWND) {
    MINI_PLAYER_HWND.store(hwnd as isize, Ordering::SeqCst);
    crate::spectrum::update_spectrum_needed_state();
    if !hwnd.is_null() {
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.register_window(WindowKind::MiniPlayer, hwnd);
        });
    } else {
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.unregister_window(WindowKind::MiniPlayer);
        });
    }
}

pub fn get_mini_player_hwnd() -> HWND {
    get_window_hwnd(WindowKind::MiniPlayer)
}

pub fn get_window_hwnd(kind: WindowKind) -> HWND {
    match kind {
        WindowKind::Main => crate::tray::get_main_hwnd(),
        WindowKind::Visualizer => crate::visualizer::get_visualizer_hwnd(),
        WindowKind::Equalizer => {
            let hwnd = EQUALIZER_HWND.load(Ordering::SeqCst);
            if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
                hwnd as HWND
            } else {
                std::ptr::null_mut()
            }
        }
        WindowKind::MiniPlayer => {
            let hwnd = MINI_PLAYER_HWND.load(Ordering::SeqCst);
            if hwnd != 0 && unsafe { IsWindow(hwnd as HWND) != 0 } {
                hwnd as HWND
            } else {
                std::ptr::null_mut()
            }
        }
    }
}

/// Check if a window is currently active, valid, visible, and not minimized.
pub fn is_window_visible_and_active(kind: WindowKind) -> bool {
    if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
        return false;
    }
    if kind == WindowKind::Visualizer {
        return crate::visualizer::is_visualizer_visible();
    }
    let hwnd = get_window_hwnd(kind);
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) != 0
            && windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0
            && windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) == 0
    }
}

/// Dock side relationship of Slave window relative to Master Window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    None,
    Right,  // Slave is docked to the right edge of Master Window
    Left,   // Slave is docked to the left edge of Master Window
    Top,    // Slave is docked above Master Window
    Bottom, // Slave is docked below Master Window
}

/// Generalized Dock Bond (Slave -> Master edge)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockBond {
    pub is_bonded: bool,
    pub parent: WindowKind,
    pub side: DockSide,
    pub offset_x: i32,
    pub offset_y: i32,
}


pub const ALL_WINDOW_KINDS: [WindowKind; 4] = [
    WindowKind::Main,
    WindowKind::MiniPlayer,
    WindowKind::Equalizer,
    WindowKind::Visualizer,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRole {
    Standalone,
    ClusterMaster,
    ClusterSlave,
}

pub fn get_window_role(kind: WindowKind) -> WindowRole {
    with_dock_tree(|tree| {
        let is_bonded = tree.get(&kind).map(|b| b.is_bonded).unwrap_or(false);
        if is_bonded {
            WindowRole::ClusterSlave
        } else {
            let has_bonded_children = tree.values().any(|b| b.is_bonded && b.parent == kind);
            if has_bonded_children {
                WindowRole::ClusterMaster
            } else {
                WindowRole::Standalone
            }
        }
    })
}


// --- Window Title Query & HWND Discovery ---

pub unsafe fn get_window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        String::new()
    }
}

unsafe extern "system" fn enum_windows_find_by_kind_proc(
    hwnd: HWND,
    lparam: LPARAM,
) -> windows_sys::Win32::Foundation::BOOL {
    let pair = &mut *(lparam as *mut (WindowKind, HWND));
    let target_kind = pair.0;

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == windows_sys::Win32::System::Threading::GetCurrentProcessId() {
        if IsWindow(hwnd) != 0 {
            let title = get_window_title(hwnd);
            if target_kind.matches_window_title(&title) {
                pair.1 = hwnd;
                return 0; // stop enumeration
            }
        }
    }
    1 // continue enumeration
}

pub fn find_hwnd_by_kind(kind: WindowKind) -> HWND {
    let existing = get_window_hwnd(kind);
    if !existing.is_null() && unsafe { IsWindow(existing) != 0 } {
        return existing;
    }
    let mut pair = (kind, std::ptr::null_mut() as HWND);
    unsafe {
        EnumWindows(
            Some(enum_windows_find_by_kind_proc),
            &mut pair as *mut _ as LPARAM,
        );
    }
    if !pair.1.is_null() {
        match kind {
            WindowKind::Equalizer => set_equalizer_hwnd(pair.1),
            WindowKind::MiniPlayer => set_mini_player_hwnd(pair.1),
            _ => {}
        }
    }
    pair.1
}

pub fn find_equalizer_hwnd() -> HWND {
    find_hwnd_by_kind(WindowKind::Equalizer)
}

pub fn find_mini_player_hwnd() -> HWND {
    find_hwnd_by_kind(WindowKind::MiniPlayer)
}


pub(crate) static LAST_SPAWNER: Mutex<Option<HashMap<WindowKind, WindowKind>>> = Mutex::new(None);

pub fn set_window_spawner(target: WindowKind, spawner: WindowKind) {
    let mut guard = LAST_SPAWNER.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(target, spawner);
}

pub fn get_window_spawner(target: WindowKind) -> Option<WindowKind> {
    let guard = LAST_SPAWNER.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().and_then(|m| m.get(&target).copied())
}


