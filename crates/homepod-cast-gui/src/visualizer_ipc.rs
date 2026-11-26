//! Type-safe IPC Protocol for communication between Rust Visualizer Host and WebView2 Frontend.

use serde::{Deserialize, Serialize};

/// Strongly-typed outgoing messages sent from Rust to WebView2.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum HostToWebMessage<'a> {
    #[serde(rename = "spectrum")]
    Spectrum {
        data: &'a [u8],
    },
    #[serde(rename = "init_state")]
    InitState {
        lang: &'a str,
        pinned: bool,
        badge: &'a str,
        docked: bool,
    },
    #[serde(rename = "set_language")]
    SetLanguage {
        lang: &'a str,
    },
    #[serde(rename = "title_badge")]
    TitleBadge {
        badge: &'a str,
        docked: bool,
    },
    #[serde(rename = "vis_pin_changed")]
    VisPinChanged {
        pinned: bool,
        from_host: bool,
    },
    #[serde(rename = "vis_fullscreen_changed")]
    VisFullscreenChanged {
        fullscreen: bool,
    },
    #[serde(rename = "sync_opacity")]
    SyncOpacity {
        val: u32,
        show_hud: bool,
    },
}

/// Strongly-typed incoming commands sent from WebView2 to Rust Host.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action", content = "payload")]
pub enum WebToHostCommand {
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "close_cluster")]
    CloseCluster,
    #[serde(rename = "minimize")]
    Minimize,
    #[serde(rename = "minimize_solo")]
    MinimizeSolo,
    #[serde(rename = "start_drag")]
    StartDrag,
    #[serde(rename = "fullscreen")]
    ToggleFullscreen,
    #[serde(rename = "toggle_pin")]
    TogglePin,
    #[serde(rename = "ready")]
    VisReady,
    #[serde(rename = "show_main")]
    ShowMain,
    #[serde(rename = "wake_ghost")]
    WakeGhost,
    #[serde(rename = "wake_ghost_solo")]
    WakeGhostSolo,
    #[serde(rename = "toggle_opacity")]
    ToggleOpacity,
    #[serde(rename = "toggle_opacity_solo")]
    ToggleOpacitySolo,
    #[serde(rename = "set_opacity")]
    SetOpacity {
        pct: f32,
    },
    #[serde(rename = "set_opacity_solo")]
    SetOpacitySolo {
        pct: f32,
    },
    #[serde(rename = "opacity_menu_open")]
    OpacityMenuOpen {
        is_open: bool,
    },
    #[serde(rename = "set_hover_peek")]
    SetHoverPeek {
        enabled: bool,
    },
    #[serde(rename = "suppress_peek")]
    SuppressPeek {
        suppress: bool,
    },
    #[serde(rename = "vis_mouse_enter")]
    MouseEnter,
    #[serde(rename = "vis_mouse_leave")]
    MouseLeave,
}

impl WebToHostCommand {
    /// Robust parsing that accepts both modern JSON envelopes (e.g. `{"action":"close"}`)
    /// and legacy raw string literals (e.g. `"close"`, `"minimize_solo"`, `"set_opacity:80"`).
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        // 1. Try parsing JSON object: {"action":"close"}
        if let Ok(cmd) = serde_json::from_str::<WebToHostCommand>(trimmed) {
            return Some(cmd);
        }
        // 2. Try parsing JSON quoted string: "close"
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            if let Ok(s) = serde_json::from_str::<String>(trimmed) {
                return Self::from_str_exact(&s);
            }
        }
        // 3. Fallback to exact match on unquoted raw string
        Self::from_str_exact(trimmed)
    }

    fn from_str_exact(s: &str) -> Option<Self> {
        if let Some(val) = s.strip_prefix("set_opacity_solo:") {
            if let Ok(pct) = val.trim().parse::<f32>() {
                return Some(WebToHostCommand::SetOpacitySolo { pct });
            }
        }
        if let Some(val) = s.strip_prefix("set_opacity:") {
            if let Ok(pct) = val.trim().parse::<f32>() {
                return Some(WebToHostCommand::SetOpacity { pct });
            }
        }
        if let Some(val) = s.strip_prefix("opacity_menu_open:") {
            let v = val.trim();
            let is_open = v == "1" || v == "true";
            return Some(WebToHostCommand::OpacityMenuOpen { is_open });
        }
        if let Some(val) = s.strip_prefix("set_hover_peek:") {
            let v = val.trim();
            let enabled = v == "1" || v == "true";
            return Some(WebToHostCommand::SetHoverPeek { enabled });
        }
        if let Some(val) = s.strip_prefix("suppress_peek:") {
            let v = val.trim();
            let suppress = v == "1" || v == "true";
            return Some(WebToHostCommand::SuppressPeek { suppress });
        }

        match s {
            "close" => Some(WebToHostCommand::Close),
            "close_cluster" => Some(WebToHostCommand::CloseCluster),
            "minimize" => Some(WebToHostCommand::Minimize),
            "minimize_solo" => Some(WebToHostCommand::MinimizeSolo),
            "start_drag" => Some(WebToHostCommand::StartDrag),
            "fullscreen" => Some(WebToHostCommand::ToggleFullscreen),
            "toggle_pin" => Some(WebToHostCommand::TogglePin),
            "vis_ready" | "ready" => Some(WebToHostCommand::VisReady),
            "show_main" => Some(WebToHostCommand::ShowMain),
            "wake_ghost" => Some(WebToHostCommand::WakeGhost),
            "wake_ghost_solo" => Some(WebToHostCommand::WakeGhostSolo),
            "toggle_opacity" => Some(WebToHostCommand::ToggleOpacity),
            "toggle_opacity_solo" => Some(WebToHostCommand::ToggleOpacitySolo),
            "vis_mouse_enter" => Some(WebToHostCommand::MouseEnter),
            "vis_mouse_leave" => Some(WebToHostCommand::MouseLeave),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_to_host_command_parsing() {
        assert_eq!(WebToHostCommand::parse("close"), Some(WebToHostCommand::Close));
        assert_eq!(WebToHostCommand::parse("close_cluster"), Some(WebToHostCommand::CloseCluster));
        assert_eq!(WebToHostCommand::parse("minimize"), Some(WebToHostCommand::Minimize));
        assert_eq!(WebToHostCommand::parse("minimize_solo"), Some(WebToHostCommand::MinimizeSolo));
        assert_eq!(WebToHostCommand::parse(r#"{"action":"fullscreen"}"#), Some(WebToHostCommand::ToggleFullscreen));
        assert_eq!(WebToHostCommand::parse(r#""start_drag""#), Some(WebToHostCommand::StartDrag));
        assert_eq!(WebToHostCommand::parse("set_opacity:85.5"), Some(WebToHostCommand::SetOpacity { pct: 85.5 }));
        assert_eq!(WebToHostCommand::parse("set_opacity_solo:40"), Some(WebToHostCommand::SetOpacitySolo { pct: 40.0 }));
        assert_eq!(WebToHostCommand::parse("vis_mouse_enter"), Some(WebToHostCommand::MouseEnter));
        assert_eq!(WebToHostCommand::parse("vis_mouse_leave"), Some(WebToHostCommand::MouseLeave));
        assert_eq!(WebToHostCommand::parse("unknown_command"), None);
    }

    #[test]
    fn test_host_to_web_message_serialization() {
        let spectrum = [10u8, 50u8, 200u8];
        let msg = HostToWebMessage::Spectrum { data: &spectrum };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"spectrum""#));
        assert!(json.contains(r#""data":[10,50,200]"#));

        let init = HostToWebMessage::InitState {
            lang: "zh-TW",
            pinned: true,
            badge: "📎",
            docked: true,
        };
        let init_json = serde_json::to_string(&init).unwrap();
        assert!(init_json.contains(r#""type":"init_state""#));
        assert!(init_json.contains(r#""lang":"zh-TW""#));
    }
}
