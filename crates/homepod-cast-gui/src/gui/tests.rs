use std::sync::{Arc, RwLock};
use eframe::egui;
use crate::controller::ControllerState;
use super::*;

#[cfg(test)]
impl FlyoutApp {
    pub fn new_mock() -> Self {
        let (cmd_tx, _) = async_channel::bounded(1);
        let shared_state = Arc::new(RwLock::new(ControllerState::default()));
        Self {
            cmd_tx,
            shared_state,
            state: ControllerState::default(),
            volume_slider: 1.0,
            is_dragging_volume: false,
            last_applied_height: 680.0,
            last_applied_width: 460.0,
            selected_device_ids: std::collections::HashSet::new(),
            current_page: Page::Main,
            last_page: Page::Main,
            page_history: Vec::new(),
            page_transition: None,
            settings_tab: SettingsTab::default(),
            meter_level: 0.0,
            last_meter_update: std::time::Instant::now(),
            startup_time: std::time::Instant::now(),
            startup_trimmed: false,
            frame_count: 0,
            last_hidden: false,
            spectrum_vis: SpectrumVisualizer::default(),
            eq_profile: EqProfile::Normal,
            selected_preset: None,
            receiver_selected_preset: None,
            display_eq_gains: [0.0; 10],
            display_receiver_eq_gains: [0.0; 10],
            album_art_texture: None,
            last_album_art_bytes: None,
            receiver_artwork_texture: None,
            last_receiver_artwork_len: 0,
            balance: 0.0,
            is_mono: false,
            is_always_on_top: false,
            equalizer_window_open: false,
            mini_player_window_open: false,
            mini_player_needs_initial_dock: false,
            equalizer_needs_initial_dock: false,
            seek_drag_secs: None,
            app_icon_texture: None,
            pending_modular_workstation_frames: 0,
            track_transition: None,
            prev_track_title: String::new(),
            prev_track_artist: String::new(),
            prev_track_album: String::new(),
            prev_album_art_texture: None,
            next_track_intent: None,
            workstation_onboarding_dismissed: crate::controller::is_workstation_discovered(),
            mascot_pos: egui::pos2(80.0, 38.0),
            mascot_activity: 0,
            mascot_platform_idx: 1,
            mascot_dir_x: 1.0,
            mascot_dir_y: 1.0,
            mascot_vel_y: 0.0,
            mascot_action_timer: 0.0,
            mascot_squash: 1.0,
            mascot_bounce: 0.0,
            mascot_last_update: std::time::Instant::now(),
            mascot_window: crate::snapping::WindowKind::Main,
            mascot_spectrum_rect: None,
            mascot_spectrum_privilege: false,
            show_supporter_mascot_modal: false,
            reset_about_scroll: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flyoutapp_navigation_stack() {
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.can_go_back());

        // Main -> Settings
        app.navigate_to(Page::Settings);
        assert_eq!(app.current_page, Page::Settings);
        assert!(app.can_go_back());

        // Settings -> Equalizer (e.g. adjust receiver EQ)
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.current_page, Page::Equalizer);

        // Go back -> Settings (should return to Settings, resolving user issue!)
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Settings);

        // Settings -> About
        app.navigate_to(Page::About);
        assert_eq!(app.current_page, Page::About);

        // Go back -> Settings
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Settings);

        // Go back -> Main
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.can_go_back());

        // Cannot go back further
        assert!(!app.go_back());
    }

    #[test]
    fn test_flyoutapp_navigation_reset() {
        let mut app = FlyoutApp::new_mock();
        app.navigate_to(Page::Settings);
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.page_history.len(), 2);

        app.reset_to(Page::Main);
        assert_eq!(app.current_page, Page::Main);
        assert!(app.page_history.is_empty());
        assert!(!app.can_go_back());
    }

    #[test]
    fn test_flyoutapp_equalizer_state_sync() {
        let _lock = crate::snapping::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        use crate::snapping::WindowKind;
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.equalizer_window_open);
        // Initially inactive
        assert!(!app.is_equalizer_active());

        // Embedded page activation
        app.navigate_to(Page::Equalizer);
        assert!(app.is_equalizer_active());

        app.go_back();
        assert_eq!(app.current_page, Page::Main);
        assert!(!app.is_equalizer_active());

        // Standalone window activation (e.g. from Mini Player)
        app.toggle_equalizer_window(WindowKind::Main);
        assert!(app.equalizer_window_open);
        // Even when on Main page, Equalizer is active (green button sync!)
        assert!(app.is_equalizer_active());

        // Closing standalone window
        app.toggle_equalizer_window(WindowKind::Main);
        assert!(!app.equalizer_window_open);
        assert!(!app.is_equalizer_active());

        // Receiver profile navigation test
        app.eq_profile = EqProfile::Receiver;
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.eq_profile, EqProfile::Receiver);
    }

    #[test]
    fn test_universal_popout_behavior() {
        let _lock = crate::snapping::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::snapping::reset_all();
        use crate::snapping::WindowKind;

        // 1. Verify bidirectional mapping
        assert_eq!(Page::Player.popout_window_kind(), Some(WindowKind::MiniPlayer));
        assert_eq!(Page::Equalizer.popout_window_kind(), Some(WindowKind::Equalizer));
        assert_eq!(Page::Main.popout_window_kind(), None);
        assert_eq!(WindowKind::MiniPlayer.embedded_page(), Some(Page::Player));
        assert_eq!(WindowKind::Equalizer.embedded_page(), Some(Page::Equalizer));

        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);

        // 2. Player Page -> Popout MiniPlayer
        app.navigate_to(Page::Player);
        assert_eq!(app.current_page, Page::Player);
        assert!(!app.is_window_open(WindowKind::MiniPlayer));

        // First click: Pop-out! Standalone opens, main window detaches and goes back to previous page!
        app.handle_popout_toggle(WindowKind::MiniPlayer, WindowKind::Main);
        assert!(app.is_window_open(WindowKind::MiniPlayer));
        assert_eq!(app.current_page, Page::Main);

        // Second click: When green (already open), clicking again closes the standalone window!
        app.handle_popout_toggle(WindowKind::MiniPlayer, WindowKind::Main);
        assert!(!app.is_window_open(WindowKind::MiniPlayer));

        // 3. Equalizer Page -> Popout Equalizer
        app.navigate_to(Page::Equalizer);
        assert_eq!(app.current_page, Page::Equalizer);
        assert!(!app.is_window_open(WindowKind::Equalizer));

        // First click: Pop-out! Standalone opens, main window detaches and goes back!
        app.handle_popout_toggle(WindowKind::Equalizer, WindowKind::Main);
        assert!(app.is_window_open(WindowKind::Equalizer));
        assert_eq!(app.current_page, Page::Main);

        // Second click: When green (already open), clicking again closes the standalone window!
        app.handle_popout_toggle(WindowKind::Equalizer, WindowKind::Main);
        assert!(!app.is_window_open(WindowKind::Equalizer));
    }

    #[test]
    fn test_universal_page_navigation_plan_b() {
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);

        // 1. Player Navigation with Popout Open
        app.mini_player_window_open = true;
        // Button turns green (is_page_active == true)
        assert!(app.is_page_active(Page::Player));
        assert!(app.is_player_active());

        // Under Plan B: clicking nav button navigates to Player page; DOES NOT close popout!
        app.navigate_toggle_page(Page::Player);
        assert_eq!(app.current_page, Page::Player);
        assert!(app.mini_player_window_open);

        // Clicking again while on Player page goes back to Main; popout STILL open!
        app.navigate_toggle_page(Page::Player);
        assert_eq!(app.current_page, Page::Main);
        assert!(app.mini_player_window_open);

        // 2. Equalizer Navigation with Popout Open (Now 100% consistent with Player!)
        app.equalizer_window_open = true;
        // Button turns green (is_page_active == true)
        assert!(app.is_page_active(Page::Equalizer));
        assert!(app.is_equalizer_active());

        // Under Plan B: clicking nav button navigates to Equalizer page; DOES NOT close popout!
        app.navigate_toggle_page(Page::Equalizer);
        assert_eq!(app.current_page, Page::Equalizer);
        assert!(app.equalizer_window_open);

        // Clicking again while on Equalizer page goes back to Main; popout STILL open!
        app.navigate_toggle_page(Page::Equalizer);
        assert_eq!(app.current_page, Page::Main);
        assert!(app.equalizer_window_open);
    }

    #[test]
    fn test_page_transition_push_and_pop() {
        let mut app = FlyoutApp::new_mock();
        assert_eq!(app.current_page, Page::Main);
        assert!(app.page_transition.is_none());

        // 1. Push transition (Main -> Settings)
        app.navigate_to(Page::Settings);
        assert_eq!(app.current_page, Page::Settings);
        let trans = app.page_transition.expect("Push transition must be active");
        assert_eq!(trans.direction, TransitionDirection::Push);
        assert_eq!(trans.from, Page::Main);
        assert_eq!(trans.to, Page::Settings);
        assert_eq!(trans.duration, std::time::Duration::from_millis(220));

        // 2. Pop transition (Settings -> Main)
        assert!(app.go_back());
        assert_eq!(app.current_page, Page::Main);
        let trans = app.page_transition.expect("Pop transition must be active");
        assert_eq!(trans.direction, TransitionDirection::Pop);
        assert_eq!(trans.from, Page::Settings);
        assert_eq!(trans.to, Page::Main);

        // 3. Reset transition
        app.navigate_to(Page::Equalizer);
        app.reset_to(Page::Main);
        assert_eq!(app.current_page, Page::Main);
        let trans = app.page_transition.expect("Reset transition must trigger Pop");
        assert_eq!(trans.direction, TransitionDirection::Pop);
        assert_eq!(trans.from, Page::Equalizer);
        assert_eq!(trans.to, Page::Main);
    }

    #[test]
    fn test_sliding_tab_bar_and_eq_curve_spline() {
        // Test catmull_rom interpolation points
        let p0 = egui::pos2(0.0, 0.0);
        let p1 = egui::pos2(10.0, 10.0);
        let p2 = egui::pos2(20.0, 10.0);
        let p3 = egui::pos2(30.0, 0.0);

        let start = catmull_rom(p0, p1, p2, p3, 0.0);
        assert!((start.x - 10.0).abs() < 1e-4);
        assert!((start.y - 10.0).abs() < 1e-4);

        let end = catmull_rom(p0, p1, p2, p3, 1.0);
        assert!((end.x - 20.0).abs() < 1e-4);
        assert!((end.y - 10.0).abs() < 1e-4);

        let mid = catmull_rom(p0, p1, p2, p3, 0.5);
        assert!((mid.x - 15.0).abs() < 1e-4);

        // Test display_eq_gains initialization
        let app = FlyoutApp::new_mock();
        assert_eq!(app.display_eq_gains, [0.0; 10]);
        assert_eq!(app.display_receiver_eq_gains, [0.0; 10]);
    }

    #[test]
    fn test_track_transition_and_cover_crossfade() {
        let mut app = FlyoutApp::new_mock();
        let ctx = egui::Context::default();
        let current_track_arc = crate::media_control::get_current_media_track();

        // 1. Initial track setup (no transition on initial load)
        {
            let mut track = current_track_arc.write().unwrap();
            track.title = "Song 1".to_string();
            track.artist = "Artist A".to_string();
            track.album = "Album X".to_string();
        }
        app.sync_media_track(&ctx);
        assert_eq!(app.prev_track_title, "Song 1");
        assert!(app.track_transition.is_none());

        // 2. Next track change triggers forward transition
        {
            let mut track = current_track_arc.write().unwrap();
            track.title = "Song 2".to_string();
            track.artist = "Artist B".to_string();
        }
        app.sync_media_track(&ctx);
        let trans = app.track_transition.as_ref().expect("Transition must be triggered");
        assert_eq!(trans.direction, TrackTransitionDirection::Next);
        assert_eq!(trans.from_title, "Song 1");
        assert_eq!(trans.to_title, "Song 2");
        assert_eq!(trans.duration, std::time::Duration::from_millis(320));

        // 3. Previous track intent triggers backward transition
        app.next_track_intent = Some(TrackTransitionDirection::Prev);
        {
            let mut track = current_track_arc.write().unwrap();
            track.title = "Song 3".to_string();
            track.artist = "Artist C".to_string();
        }
        app.sync_media_track(&ctx);
        let trans = app.track_transition.as_ref().expect("Transition must be triggered");
        assert_eq!(trans.direction, TrackTransitionDirection::Prev);
        assert_eq!(trans.from_title, "Song 2");
        assert_eq!(trans.to_title, "Song 3");
    }

    #[test]
    fn test_close_window_cluster_all() {
        let _lock = crate::snapping::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::snapping::reset_all();

        let mut app = FlyoutApp::new_mock();
        app.mini_player_window_open = true;
        app.equalizer_window_open = true;

        let mini_r = windows_sys::Win32::Foundation::RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = windows_sys::Win32::Foundation::RECT { left: 500, top: 350, right: 900, bottom: 600 };

        crate::snapping::set_dock_relation(
            crate::snapping::WindowKind::Equalizer,
            crate::snapping::WindowKind::MiniPlayer,
            &eq_r,
            &mini_r,
        );

        assert!(app.mini_player_window_open);
        assert!(app.equalizer_window_open);

        // Close entire cluster starting from MiniPlayer
        app.close_window_cluster(crate::snapping::WindowKind::MiniPlayer);

        assert!(!app.mini_player_window_open);
        assert!(!app.equalizer_window_open);
    }

    #[test]
    fn test_workstation_onboarding_state() {
        let mut app = FlyoutApp::new_mock();
        app.workstation_onboarding_dismissed = false;
        assert!(!app.workstation_onboarding_dismissed);

        // When discovered, state updates correctly
        app.workstation_onboarding_dismissed = true;
        assert!(app.workstation_onboarding_dismissed);
    }
}

