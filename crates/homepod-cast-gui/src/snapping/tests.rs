use std::sync::atomic::Ordering;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::RECT;
use super::*;

    pub static TEST_LOCK: Mutex<()> = Mutex::new(());

    pub fn reset_all() {
        with_dock_edges(|e| e.clear());
        with_dock_tree(|t| t.clear());
        clear_known_rects();
        clear_all_cluster_fsm();
        clear_all_opacity_fsm();
        set_window_opacity(WindowKind::Main, 1.0);
        set_window_opacity(WindowKind::MiniPlayer, 1.0);
        set_window_opacity(WindowKind::Equalizer, 1.0);
        set_window_opacity(WindowKind::Visualizer, 1.0);
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);
        set_window_pinned(WindowKind::Visualizer, false);
        set_last_custom_opacity(WindowKind::Main, 30);
        set_last_custom_opacity(WindowKind::MiniPlayer, 30);
        set_last_custom_opacity(WindowKind::Equalizer, 30);
        set_last_custom_opacity(WindowKind::Visualizer, 30);
        clear_ghost_exit_record();
        crate::visualizer::reset_hud_main_shown();
        crate::visualizer::restore_hud_pinned_states();
        crate::visualizer::set_visualizer_fullscreen_mock(false);
    }

    #[test]
    fn test_dock_side_and_bonded_pos() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();
        // Main window at (100, 100), size 400x500 -> right=500, bottom=600
        let main_rect = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        // Mini player docked to Top of Main at (100, -100), size 400x200 -> right=500, bottom=100
        let mini_rect = RECT { left: 100, top: -100, right: 500, bottom: 100 };

        assert!(are_rects_docked(&mini_rect, &main_rect, 4));
        let side = determine_dock_side(&mini_rect, &main_rect, 10);
        assert_eq!(side, DockSide::Top);

        let bond = DockBond {
            is_bonded: true,
            parent: WindowKind::Main,
            side: DockSide::Top,
            offset_x: 0,
            offset_y: -200,
        };

        // When main moves to (200, 300)
        let new_main_rect = RECT { left: 200, top: 300, right: 600, bottom: 800 };
        let (slave_x, slave_y) = calc_bonded_visualizer_pos(&bond, &mini_rect, &new_main_rect);
        assert_eq!(slave_x, 200);
        assert_eq!(slave_y, 100); // 300 - mini_h (200) = 100
    }

    #[test]
    fn test_dock_tree_hierarchy_and_cycle_prevention() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 100, top: -100, right: 500, bottom: 100 };
        let eq_r = RECT { left: 100, top: -600, right: 500, bottom: -100 };

        // MiniPlayer docks to Main
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        // Equalizer docks to MiniPlayer
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // Check descendant relationships
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::MiniPlayer));
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::Main));
        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::Main, WindowKind::MiniPlayer));
        assert!(!is_descendant(WindowKind::Main, WindowKind::Equalizer));

        // Breaking MiniPlayer detaches it from Main
        break_dock_relation(WindowKind::MiniPlayer);
        assert!(!is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        // Equalizer is still bonded to MiniPlayer
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::MiniPlayer));
        assert!(!is_descendant(WindowKind::Equalizer, WindowKind::Main));

        break_dock_relation(WindowKind::Equalizer);
    }

    #[test]
    fn test_symmetric_snapping_assignment() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 }; // directly docked on Right

        // When Main approaches MiniPlayer (Main is dragged, MiniPlayer is stationary):
        // Main (Lv 1) has higher priority than MiniPlayer (Lv 2).
        // Therefore, MiniPlayer becomes Slave, Main becomes Master!
        assert!(WindowKind::Main.priority() < WindowKind::MiniPlayer.priority());
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::Main, WindowKind::MiniPlayer));

        let bond = with_dock_tree(|tree| tree.get(&WindowKind::MiniPlayer).copied()).unwrap();
        assert_eq!(bond.parent, WindowKind::Main);
        assert_eq!(bond.side, DockSide::Right);

        // When MiniPlayer approaches Equalizer (MiniPlayer is dragged, Equalizer is stationary):
        // MiniPlayer (Lv 2) has higher priority than Equalizer (Lv 3).
        // Therefore, Equalizer becomes Slave, MiniPlayer becomes Master!
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 850 }; // docked to Bottom of MiniPlayer
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        assert!(is_descendant(WindowKind::Equalizer, WindowKind::MiniPlayer));
        assert!(is_descendant(WindowKind::Equalizer, WindowKind::Main));

        let eq_bond = with_dock_tree(|tree| tree.get(&WindowKind::Equalizer).copied()).unwrap();
        assert_eq!(eq_bond.parent, WindowKind::MiniPlayer);
        assert_eq!(eq_bond.side, DockSide::Bottom);

        break_dock_relation(WindowKind::Equalizer);
        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_dock_tree_graph_bfs_case2_main_vis_mini() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 0, top: 0, right: 400, bottom: 600 };
        let vis_r = RECT { left: 400, top: 0, right: 800, bottom: 600 };
        let mini_r = RECT { left: 800, top: 0, right: 1200, bottom: 350 };

        // Main docks to Visualizer, and Visualizer docks to MiniPlayer
        // Physical chain: Main (Lv 1) <-> Visualizer (Lv 4) <-> MiniPlayer (Lv 2)
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Visualizer, &mini_r, &vis_r);

        // All 3 should be in the same component rooted at Main!
        assert_eq!(get_component_root(WindowKind::Main), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        // Check BFS tree directed edges
        let tree = with_dock_tree(|t| t.clone());
        let vis_bond = tree.get(&WindowKind::Visualizer).expect("Visualizer should be bonded");
        assert_eq!(vis_bond.parent, WindowKind::Main);
        assert_eq!(vis_bond.side, DockSide::Right);

        let mini_bond = tree.get(&WindowKind::MiniPlayer).expect("MiniPlayer should be bonded");
        assert_eq!(mini_bond.parent, WindowKind::Visualizer);
        assert_eq!(mini_bond.side, DockSide::Right);

        // Check descendant relationships
        assert!(is_descendant(WindowKind::Visualizer, WindowKind::Main));
        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Visualizer));
        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::Main, WindowKind::MiniPlayer));
        assert!(!is_descendant(WindowKind::Main, WindowKind::Visualizer));

        // When MiniPlayer is pulled away / detaches:
        break_dock_relation(WindowKind::MiniPlayer);

        // Visualizer should STILL be bonded to Main! (They do not separate!)
        assert!(is_descendant(WindowKind::Visualizer, WindowKind::Main));
        assert!(!is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);

        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_subcluster_root_selection_and_button_roles() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // Subcluster without Main: MiniPlayer (Lv 2) <-> Equalizer (Lv 3)
        let mini_r = RECT { left: 100, top: 100, right: 500, bottom: 350 };
        let eq_r = RECT { left: 100, top: 350, right: 500, bottom: 850 };

        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // Root of component is MiniPlayer
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::MiniPlayer);

        let tree = with_dock_tree(|t| t.clone());
        // MiniPlayer has no parent -> is Root -> keeps caption buttons
        let mini_is_slave = tree.get(&WindowKind::MiniPlayer).map(|b| b.is_bonded).unwrap_or(false);
        assert!(!mini_is_slave);

        // Equalizer has parent MiniPlayer -> is Slave -> caption buttons hidden
        let eq_is_slave = tree.get(&WindowKind::Equalizer).map(|b| b.is_bonded).unwrap_or(false);
        assert!(eq_is_slave);

        break_dock_relation(WindowKind::Equalizer);
    }

    #[test]
    fn test_cluster_members_retrieval_and_bring_to_top() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Single standalone window
        let single = get_cluster_members(WindowKind::Main);
        assert_eq!(single, vec![WindowKind::Main]);

        // 2. Form cluster: Main <-> Visualizer <-> Equalizer
        let main_r = RECT { left: 0, top: 0, right: 400, bottom: 600 };
        let vis_r = RECT { left: 400, top: 0, right: 800, bottom: 600 };
        let eq_r = RECT { left: 800, top: 0, right: 1200, bottom: 500 };

        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::Visualizer, &eq_r, &vis_r);

        // Querying from any member of the cluster should return all 3 members!
        let from_main = get_cluster_members(WindowKind::Main);
        let from_vis = get_cluster_members(WindowKind::Visualizer);
        let from_eq = get_cluster_members(WindowKind::Equalizer);

        assert_eq!(from_main.len(), 3);
        assert_eq!(from_vis.len(), 3);
        assert_eq!(from_eq.len(), 3);

        for member in &[WindowKind::Main, WindowKind::Visualizer, WindowKind::Equalizer] {
            assert!(from_main.contains(member));
            assert!(from_vis.contains(member));
            assert!(from_eq.contains(member));
        }

        // Standalone window (MiniPlayer) is not in this cluster
        let from_mini = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(from_mini, vec![WindowKind::MiniPlayer]);

        break_dock_relation(WindowKind::Equalizer);
        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_spawn_priority_cascade_and_center_fallback() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Main window only: next window should spawn at Main's Right-Top (Slot 1)
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 700 };
        update_known_rect(WindowKind::Main, main_r);

        let (spawn1, anchor1) = find_best_spawn_rect(WindowKind::MiniPlayer, 300, 200, Some(WindowKind::Main));
        assert_eq!(anchor1, Some(WindowKind::Main));
        assert_eq!(spawn1.left, 500);
        assert_eq!(spawn1.top, 100);
        assert_eq!(spawn1.right, 800);
        assert_eq!(spawn1.bottom, 300);

        // 2. MiniPlayer is now placed at Main's Right-Top:
        // Next window (Equalizer) should spawn at Main's Right-Bottom (Slot 2, directly below MiniPlayer)
        let mini_r = spawn1;
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        let (spawn2, anchor2) = find_best_spawn_rect(WindowKind::Equalizer, 300, 350, Some(WindowKind::Main));
        assert_eq!(anchor2, Some(WindowKind::Main));
        assert_eq!(spawn2.left, 500);
        assert_eq!(spawn2.top, 300); // below MiniPlayer (MiniPlayer's bottom is 300)
        assert_eq!(spawn2.right, 800);
        assert_eq!(spawn2.bottom, 650);

        // 3. Right edge of Main is fully occupied by a tall Visualizer:
        // When spawning from Visualizer (Visualizer is the mother window):
        // Main: (100, 100, 500, 700), Visualizer: (500, 100, 1200, 700)
        reset_all();
        update_known_rect(WindowKind::Main, main_r);
        let vis_r = RECT { left: 500, top: 100, right: 1200, bottom: 700 };
        update_known_rect(WindowKind::Visualizer, vis_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);

        // Next window spawned from Visualizer should spawn at Visualizer's Right-Top
        let (spawn3, anchor3) = find_best_spawn_rect(WindowKind::MiniPlayer, 300, 200, Some(WindowKind::Visualizer));
        assert_eq!(anchor3, Some(WindowKind::Visualizer));
        assert_eq!(spawn3.left, 1200);
        assert_eq!(spawn3.top, 100);
        assert_eq!(spawn3.right, 1500);
        assert_eq!(spawn3.bottom, 300);

        // 4. Mother window Bottom edge (Priority 3):
        // Main: (100, 100, 500, 500).
        // Right is blocked by Right-Docked tall window down to work area bottom (500, 100, 800, 1080).
        // Left is blocked by screen edge (width 300 cannot fit in 0..100).
        // Window should dock at Mother window's Bottom edge (Slot 7)
        reset_all();
        let main_small = RECT { left: 100, top: 100, right: 500, bottom: 500 };
        update_known_rect(WindowKind::Main, main_small);
        let block_r = RECT { left: 500, top: 100, right: 800, bottom: 1080 };
        update_known_rect(WindowKind::Visualizer, block_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &block_r, &main_small);

        let (spawn_bottom, anchor_bottom) = find_best_spawn_rect(WindowKind::MiniPlayer, 300, 200, Some(WindowKind::Main));
        assert_eq!(anchor_bottom, Some(WindowKind::Main));
        assert_eq!(spawn_bottom.left, 100); // Main's left
        assert_eq!(spawn_bottom.top, 500);  // Main's bottom
        assert_eq!(spawn_bottom.right, 400);
        assert_eq!(spawn_bottom.bottom, 700);

        // 5. Screen overflow fallback:
        // Reset and position Visualizer at right 1200, so a window of width 800 and height 500
        // overflows all possible sides:
        // Main Right: blocked by vis_r (500..1200)
        // Main Left: 100 - 800 = -700 (outside work area)
        // Main Bottom: 700 + 500 = 1200 > 1080 (outside work area)
        // Secondary Vis Right: 1200 + 800 = 2000 > 1920 (outside work area)
        // Secondary Vis Bottom: 700 + 500 = 1200 > 1080 (outside work area)
        // -> Falls back to Screen Center!
        reset_all();
        update_known_rect(WindowKind::Main, main_r);
        let vis_r = RECT { left: 500, top: 100, right: 1200, bottom: 700 };
        update_known_rect(WindowKind::Visualizer, vis_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);

        let (spawn_overflow, anchor_overflow) = find_best_spawn_rect(WindowKind::Equalizer, 800, 500, Some(WindowKind::Main));
        assert_eq!(anchor_overflow, None); // Fallback: standalone in center!
        // Center of work area (1920 x 1080):
        // left = (1920 - 800) / 2 = 560
        // top = (1080 - 500) / 2 = 290
        assert_eq!(spawn_overflow.left, 560);
        assert_eq!(spawn_overflow.top, 290);
        assert_eq!(spawn_overflow.right, 1360);
        assert_eq!(spawn_overflow.bottom, 790);

        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_summon_and_dock_breaks_separated_dock_edges() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initially, Main and MiniPlayer are docked together
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 }; // docked on Right of Main
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        // 2. Visualizer is independent at (1400, 100, 1800, 600)
        let vis_r = RECT { left: 1400, top: 100, right: 1800, bottom: 600 };

        // 3. Visualizer summons Main window to dock to Visualizer (Main moves to left of Visualizer: 1000..1400)
        let new_main_r = RECT { left: 1000, top: 100, right: 1400, bottom: 600 };
        // MiniPlayer remains at (500, 100, 900, 350), which is > 24px away from (1000..1400)
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &new_main_r);

        // 4. Verification:
        // Main and Visualizer are now bonded in the same component
        assert!(is_descendant(WindowKind::Visualizer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);

        // MiniPlayer is separated from Main (distance > tolerance), so dock edge was severed!
        assert!(!is_descendant(WindowKind::MiniPlayer, WindowKind::Main));
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);

        let tree = with_dock_tree(|t| t.clone());
        let mini_bond = tree.get(&WindowKind::MiniPlayer).map(|b| b.is_bonded).unwrap_or(false);
        assert!(!mini_bond, "MiniPlayer must no longer be bonded to Main");

        break_dock_relation(WindowKind::Visualizer);
    }

    #[test]
    fn test_window_role_detection() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // Standalone initially
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);

        // Dock Equalizer to Main
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let eq_r = RECT { left: 500, top: 100, right: 960, bottom: 540 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);

        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);

        // Break dock relation
        break_dock_relation(WindowKind::Equalizer);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);
    }

    #[test]
    fn test_cluster_minimize_guard_and_role_handling() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        assert!(!is_cluster_minimizing());

        // Dock Visualizer and MiniPlayer to Main
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let vis_r = RECT { left: 500, top: 100, right: 1200, bottom: 600 };
        let mini_r = RECT { left: 100, top: 600, right: 500, bottom: 850 };
        set_dock_relation(WindowKind::Visualizer, WindowKind::Main, &vis_r, &main_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Visualizer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);

        // Verify cluster members retrieval
        let members = get_cluster_members(WindowKind::Visualizer);
        assert_eq!(members.len(), 3);
        assert_eq!(members[0], WindowKind::Main);

        break_dock_relation(WindowKind::Visualizer);
        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_root_close_reassembles_remaining_cluster_in_place() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Main at (100, 100, 500, 600)
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        // MiniPlayer at (500, 100, 900, 350) - docked to right of Main
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        // Equalizer at (500, 350, 900, 700) - docked below MiniPlayer (touching MiniPlayer)
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 700 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);

        // Both MiniPlayer and Equalizer dock to Main, while touching each other
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);

        // Before closing Main: Main is Master of the cluster
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::Main);

        // 2. User closes Root window (Main) -> hides to tray and reassembles remaining windows
        crate::tray::IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        handle_window_closed(WindowKind::Main);

        // 3. Verify in-place re-assembly:
        // Main is no longer in the cluster
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        // MiniPlayer (Level 2) is promoted to the new ClusterMaster!
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterMaster);
        // Equalizer (Level 3) is synthesized as ClusterSlave bonded to MiniPlayer!
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::MiniPlayer);

        break_dock_relation(WindowKind::MiniPlayer);
        break_dock_relation(WindowKind::Equalizer);
        crate::tray::IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_spring_overshoot_curve_and_magnetic_snap() {
        // 1. Boundary conditions: t=0 -> 0.0, t=1 -> 1.0
        let s0 = spring_overshoot_curve(0.0);
        let s1 = spring_overshoot_curve(1.0);
        assert!((s0 - 0.0).abs() < 1e-4, "Spring curve at t=0 must start at 0: got {s0}");
        assert!((s1 - 1.0).abs() < 1e-4, "Spring curve at t=1 must end at 1: got {s1}");

        // 2. Physical overshoot verification: in t in 0.35..0.55, curve should produce a tactile overshoot (> 1.0)
        let mut max_val = 0.0f32;
        let mut max_t = 0.0f32;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let val = spring_overshoot_curve(t);
            if val > max_val {
                max_val = val;
                max_t = t;
            }
        }
        assert!(max_val > 1.04 && max_val < 1.15, "Spring overshoot should peak between 4% and 15%: got {max_val}");
        assert!(max_t >= 0.35 && max_t <= 0.60, "Spring peak should occur in mid motion: got {max_t}");

        // 3. Smooth monotonic approach at the end: values between 0.9 and 1.0 converge to 1.0
        let s09 = spring_overshoot_curve(0.9);
        assert!((s09 - 1.0).abs() < 0.02, "Spring curve should settle close to 1.0 by t=0.9: got {s09}");
    }

    #[test]
    fn test_snap_rect_to_rect_magnetic() {
        // Target stationary rectangle B: [200, 200, 500, 500] (w=300, h=300)
        let r_b = RECT { left: 200, top: 200, right: 500, bottom: 500 };

        // Case 1: Moving A is near B's Left edge -> snaps cleanly to B's Left
        let mut r_a_left = RECT { left: 80, top: 210, right: 195, bottom: 410 };
        assert!(snap_rect_to_rect(&mut r_a_left, &r_b, 12));
        assert_eq!(r_a_left.right, 200);

        // Case 2: Moving A is near B's Right edge -> snaps cleanly to B's Right
        let mut r_a_right = RECT { left: 508, top: 210, right: 620, bottom: 410 };
        assert!(snap_rect_to_rect(&mut r_a_right, &r_b, 12));
        assert_eq!(r_a_right.left, 500);

        // Case 3: Moving A is near B's Top edge -> snaps cleanly to B's Top
        let mut r_a_top = RECT { left: 220, top: 90, right: 420, bottom: 196 };
        assert!(snap_rect_to_rect(&mut r_a_top, &r_b, 12));
        assert_eq!(r_a_top.bottom, 200);

        // Case 4: Moving A is near B's Bottom edge -> snaps cleanly to B's Bottom
        let mut r_a_bottom = RECT { left: 220, top: 506, right: 420, bottom: 610 };
        assert!(snap_rect_to_rect(&mut r_a_bottom, &r_b, 12));
        assert_eq!(r_a_bottom.top, 500);

        // Case 5: Far away -> No snap
        let mut r_a_far = RECT { left: 800, top: 800, right: 900, bottom: 900 };
        assert!(!snap_rect_to_rect(&mut r_a_far, &r_b, 12));
    }

    #[test]
    fn test_per_window_pinning_and_cluster_propagation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Reset all dock edges and pin states
        remove_all_edges_for(WindowKind::Main);
        remove_all_edges_for(WindowKind::MiniPlayer);
        remove_all_edges_for(WindowKind::Equalizer);
        remove_all_edges_for(WindowKind::Visualizer);
        unsafe { rebuild_dock_tree(); }

        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);
        set_window_pinned(WindowKind::Visualizer, false);

        // 2. Standalone autonomy: toggling MiniPlayer does NOT affect Main
        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer));

        unsafe { toggle_window_pin(WindowKind::MiniPlayer); }
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should now be pinned");
        assert!(!is_window_pinned(WindowKind::Main), "Main window must remain unpinned (no global linking)");

        // 3. Docking assimilation: When MiniPlayer docks to unpinned Main, it inherits Main's unpinned state
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should inherit anchor's unpinned state upon docking");

        // 4. Cluster consistency: Toggling on cluster member propagates to entire cluster
        unsafe { toggle_window_pin(WindowKind::MiniPlayer); }
        assert!(is_window_pinned(WindowKind::Main), "Main should be pinned with cluster");
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should be pinned with cluster");

        // 5. Detach retention: When MiniPlayer breaks away, it retains current pinned state
        break_dock_relation(WindowKind::MiniPlayer);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should retain pinned state upon detach");

        // And can now be independently unpinned without unpinning Main
        unsafe { toggle_window_pin(WindowKind::MiniPlayer); }
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer should be unpinned");
        assert!(is_window_pinned(WindowKind::Main), "Main window should still remain pinned");
    }

    #[test]
    fn test_anchor_precedence_assimilation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. 初始化重設所有視窗狀態與拓撲關係
        remove_all_edges_for(WindowKind::Main);
        remove_all_edges_for(WindowKind::MiniPlayer);
        remove_all_edges_for(WindowKind::Equalizer);
        remove_all_edges_for(WindowKind::Visualizer);
        unsafe { rebuild_dock_tree(); }

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        let vis_r = RECT { left: 900, top: 100, right: 1500, bottom: 600 };

        // 情境 A：錨點置頂 (Anchor=True)，拖入的新成員未置頂 (Moving=False) -> 新成員同化為 True
        set_window_pinned(WindowKind::Main, true);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must be assimilated to anchor's pinned=true");

        // 拆解分離
        break_dock_relation(WindowKind::MiniPlayer);
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer retains pinned=true after detach");

        // 情境 B：錨點未置頂 (Anchor=False)，拖入的新成員置頂 (Moving=True) -> 新成員同化為 False
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::Equalizer, true);
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);

        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::Equalizer), "Equalizer must be assimilated to anchor's unpinned=false");

        // 拆解分離
        break_dock_relation(WindowKind::Equalizer);

        // 情境 C：非 Main 視窗相互吸附時，目標錨點居主導地位
        // MiniPlayer (Anchor, pinned=true) <- Visualizer (Moving, pinned=false)
        set_window_pinned(WindowKind::MiniPlayer, true);
        set_window_pinned(WindowKind::Visualizer, false);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        assert!(is_window_pinned(WindowKind::MiniPlayer));
        assert!(is_window_pinned(WindowKind::Visualizer), "Visualizer must assimilate to MiniPlayer anchor pinned=true");

        // 情境 D：多視窗連通叢集吸附至錨點時，全體叢集成員同態一致
        // 現有叢集 MiniPlayer + Visualizer (皆為 true)，被吸附至 Main (Anchor, pinned=false)
        set_window_pinned(WindowKind::Main, false);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must be assimilated to Main's unpinned=false");
        assert!(!is_window_pinned(WindowKind::Visualizer), "Visualizer in cluster must also be assimilated to Main's unpinned=false");
    }

    #[test]
    fn test_window_opacity_and_hover_peek() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let orig_main_op = get_window_opacity(WindowKind::Main);
        let orig_mini_op = get_window_opacity(WindowKind::MiniPlayer);
        let orig_eq_op = get_window_opacity(WindowKind::Equalizer);
        let orig_vis_op = get_window_opacity(WindowKind::Visualizer);
        let orig_peek = is_hover_peek_enabled();

        // 1. Initial / default bounds
        set_window_opacity(WindowKind::Main, 0.80);
        assert!((get_window_opacity(WindowKind::Main) - 0.80).abs() < 0.01);

        set_window_opacity(WindowKind::MiniPlayer, 0.75);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 0.75).abs() < 0.01);

        // 2. Clamp verification: min 0.2, max 1.0
        set_window_opacity(WindowKind::Main, 0.05);
        assert!((get_window_opacity(WindowKind::Main) - 0.20).abs() < 0.01);

        set_window_opacity(WindowKind::MiniPlayer, 1.50);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 1.00).abs() < 0.01);

        // 3. Equalizer & Visualizer opacity
        set_window_opacity(WindowKind::Equalizer, 0.70);
        assert!((get_window_opacity(WindowKind::Equalizer) - 0.70).abs() < 0.01);

        set_window_opacity(WindowKind::Visualizer, 0.60);
        assert!((get_window_opacity(WindowKind::Visualizer) - 0.60).abs() < 0.01);

        // 4. Suppress peek checking via active tuning
        assert!(!is_peek_suppressed(WindowKind::MiniPlayer));
        trigger_opacity_tuning(WindowKind::MiniPlayer);
        assert!(is_peek_suppressed(WindowKind::MiniPlayer));

        // 5. Hover peek toggling
        set_hover_peek_enabled(true);
        assert!(is_hover_peek_enabled());
        set_hover_peek_enabled(false);
        assert!(!is_hover_peek_enabled());
        set_hover_peek_enabled(true);
        assert!(is_hover_peek_enabled());

        // 6. Cluster opacity synchronization test
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        set_cluster_opacity(WindowKind::MiniPlayer, 0.45);
        assert!((get_window_opacity(WindowKind::Main) - 0.45).abs() < 0.01);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 0.45).abs() < 0.01);
        assert!(is_peek_suppressed(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::MiniPlayer));
        assert!(is_opacity_hud_active(WindowKind::Main));
        assert!(is_opacity_hud_active(WindowKind::MiniPlayer));
        break_dock_relation(WindowKind::MiniPlayer);

        // 7. Click-through mode threshold verification (<= 40%)
        set_window_opacity(WindowKind::Main, 0.45);
        assert!(!is_click_through_active(WindowKind::Main));
        set_window_opacity(WindowKind::Main, 0.40);
        assert!(is_click_through_active(WindowKind::Main));
        set_window_opacity(WindowKind::Main, 0.35);
        assert!(is_click_through_active(WindowKind::Main));
        set_window_opacity(WindowKind::Main, 0.20);
        assert!(is_click_through_active(WindowKind::Main));

        // Restore original user settings so unit test NEVER pollutes user's config.json
        set_window_opacity(WindowKind::Main, orig_main_op);
        set_window_opacity(WindowKind::MiniPlayer, orig_mini_op);
        set_window_opacity(WindowKind::Equalizer, orig_eq_op);
        set_window_opacity(WindowKind::Visualizer, orig_vis_op);
        set_hover_peek_enabled(orig_peek);
    }

    #[test]
    fn test_opacity_fsm_lifecycle_and_transitions() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initial state is Normal
        assert_eq!(get_opacity_state(WindowKind::Main), OpacityState::Normal);
        assert!(!is_opacity_hud_active(WindowKind::Main));
        assert!(!is_peek_suppressed(WindowKind::Main));
        assert!(!is_opacity_menu_open(WindowKind::Main));

        // 2. Trigger active tuning
        trigger_opacity_tuning(WindowKind::Main);
        assert!(matches!(get_opacity_state(WindowKind::Main), OpacityState::Tuning(_)));
        assert!(is_opacity_hud_active(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::Main));

        // 3. Open menu
        set_opacity_menu_open(WindowKind::Main, true);
        assert_eq!(get_opacity_state(WindowKind::Main), OpacityState::MenuOpen);
        assert!(is_opacity_menu_open(WindowKind::Main));
        assert!(!is_opacity_hud_active(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::Main));

        // 4. Close menu -> transitions to Fading
        set_opacity_menu_open(WindowKind::Main, false);
        assert!(matches!(get_opacity_state(WindowKind::Main), OpacityState::Fading(_)));
        assert!(!is_opacity_menu_open(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::Main));

        // 5. Cluster tuning propagation
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        trigger_cluster_opacity_tuning(WindowKind::MiniPlayer);
        assert!(is_opacity_hud_active(WindowKind::Main));
        assert!(is_opacity_hud_active(WindowKind::MiniPlayer));
        assert!(is_peek_suppressed(WindowKind::Main));
        assert!(is_peek_suppressed(WindowKind::MiniPlayer));

        break_dock_relation(WindowKind::MiniPlayer);
        reset_all();
    }

    #[test]
    fn test_minimize_window_solo() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterSlave);

        // Solo minimize MiniPlayer: it should break dock relation and become standalone
        unsafe {
            minimize_window_solo(WindowKind::MiniPlayer);
        }

        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
    }

    #[test]
    fn test_reverse_spring_curve_and_staggered_collapse() {
        // Boundary values
        assert_eq!(reverse_spring_curve(0.0), 0.0);
        let end_val = reverse_spring_curve(1.0);
        assert!((end_val - 1.0).abs() < 1e-4);

        // Monotonic progression
        let mid1 = reverse_spring_curve(0.3);
        let mid2 = reverse_spring_curve(0.7);
        assert!(mid1 > 0.0 && mid1 < mid2 && mid2 < 1.0);

        // Staggered delay behavior
        assert_eq!(reverse_staggered_spring(0.04, 0.08), 0.0);
        assert!(reverse_staggered_spring(0.12, 0.08) > 0.0);
        assert!((reverse_staggered_spring(1.0, 0.08) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_cluster_taskbar_ownership_sync_simulation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Standalone initially
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);

        // 2. Form a sub-cluster of small windows (MiniPlayer + Equalizer)
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::MiniPlayer);

        // Safe call to sync (handles null HWND safely in mock/test)
        unsafe {
            sync_cluster_taskbar_ownership();
        }

        // 3. Main hidden in tray simulation: ensures no panic and proper state handling
        crate::tray::IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        unsafe {
            sync_cluster_taskbar_ownership();
        }
        assert!(crate::tray::is_window_hidden_to_tray());
        crate::tray::IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);

        // Form two separate clusters: Cluster A (Main) and Cluster B (MiniPlayer + Equalizer)
        let main_r = RECT { left: 0, top: 0, right: 460, bottom: 680 };
        update_known_rect(WindowKind::Main, main_r);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::ClusterMaster);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::ClusterSlave);

        // Both Cluster Masters (Main and MiniPlayer) have independent taskbar presence,
        // while Equalizer is grouped under MiniPlayer.
        unsafe {
            sync_cluster_taskbar_ownership();
        }

        break_dock_relation(WindowKind::Equalizer);
        assert_eq!(get_window_role(WindowKind::Equalizer), WindowRole::Standalone);
        assert_eq!(get_window_role(WindowKind::MiniPlayer), WindowRole::Standalone);
    }

    #[test]
    fn test_compute_cluster_docked_targets() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 建立 Main(root) -> MiniPlayer(Right) -> Equalizer(Bottom) 的三級串聯結構
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 }; // 460 x 680
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 }; // 460 x 340
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 }; // 460 x 340

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // 假設 Main 還原到了新坐標 (200, 150)
        let new_main_r = RECT { left: 200, top: 150, right: 660, bottom: 830 };
        let targets = compute_cluster_docked_targets(WindowKind::Main, &new_main_r);

        let main_target = targets.get(&WindowKind::Main).expect("Main target exists");
        assert_eq!(main_target.left, new_main_r.left);
        assert_eq!(main_target.top, new_main_r.top);
        assert_eq!(main_target.right, new_main_r.right);
        assert_eq!(main_target.bottom, new_main_r.bottom);

        let mini_target = targets.get(&WindowKind::MiniPlayer).expect("MiniPlayer target exists");
        assert_eq!(mini_target.left, 660);
        assert_eq!(mini_target.top, 150);

        let eq_target = targets.get(&WindowKind::Equalizer).expect("Equalizer target exists");
        assert_eq!(eq_target.left, 660);
        assert_eq!(eq_target.top, 150 + (mini_target.bottom - mini_target.top));
    }

    #[test]
    fn test_staggered_spring_expand_behavior() {
        // 1. Initial boundary
        assert_eq!(staggered_spring(0.0, 0.0), 0.0);
        assert_eq!(staggered_spring(0.04, 0.08), 0.0); // 未達延遲前保持靜止

        // 2. 局部超調（Overshoot）特性檢驗：物理彈簧在接近收斂前會產生微幅 > 1.0 的彈性手感
        let mut has_overshoot = false;
        for step in 1..=20 {
            let t = step as f32 / 20.0;
            let val = staggered_spring(t, 0.0);
            if val > 1.0 {
                has_overshoot = true;
                break;
            }
        }
        assert!(has_overshoot, "Physical spring curve must exhibit natural overshoot for haptic feedback");

        // 3. 終端收斂精確性
        let end_val = staggered_spring(1.0, 0.08);
        assert!((end_val - 1.0).abs() < 1e-3, "Spring must converge to 1.0 at termination");
    }

    #[test]
    fn test_expand_cluster_animated_lock() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();
        assert!(!is_cluster_restoring());
        set_cluster_state(WindowKind::Main, ClusterState::Restoring(std::time::Instant::now()));
        assert!(is_cluster_restoring());
        finish_cluster_restore(WindowKind::Main);
        assert!(!is_cluster_restoring());
    }

    #[test]
    fn test_collapsed_silent_hide_simulation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        let members = get_cluster_members(WindowKind::Main);
        assert_eq!(members.len(), 2);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        // 模擬在 HWND 為 null 的環境下安全調用 restore_cluster，無崩潰
        unsafe {
            restore_cluster(WindowKind::Main, std::ptr::null_mut());
        }
    }

    #[test]
    fn test_is_main_in_cluster_and_cluster_summon() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. MiniPlayer 與 Equalizer 組成小群組
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        assert!(!is_main_in_cluster(WindowKind::MiniPlayer));
        assert!(!is_main_in_cluster(WindowKind::Equalizer));

        // 2. 模擬主視窗吸附至該小群組（停靠在整組的左側）
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        assert!(is_main_in_cluster(WindowKind::MiniPlayer));
        assert!(is_main_in_cluster(WindowKind::Equalizer));
        assert!(is_main_in_cluster(WindowKind::Main));

        // 3. 再次調用 handle_home_summon_request 應安全聚焦主視窗
        unsafe {
            handle_home_summon_request(WindowKind::Equalizer);
        }
    }

    #[test]
    fn test_hud_pinned_state_backup_and_restoration() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Setup cluster: Main + MiniPlayer + Equalizer
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        // Initially unpinned
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);

        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer));
        assert!(!is_window_pinned(WindowKind::Equalizer));

        // 2. Simulate entering fullscreen HUD
        crate::visualizer::show_hud_cluster();

        assert!(is_window_pinned(WindowKind::Main), "Main must be pinned in HUD");
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must be pinned in HUD");
        assert!(is_window_pinned(WindowKind::Equalizer), "Equalizer must be pinned in HUD");

        // 3. Simulate dragging Main window during HUD:
        // assimilate_cluster_to_anchor must preserve pinned=true!
        unsafe {
            assimilate_cluster_to_anchor(WindowKind::Main, WindowKind::MiniPlayer);
        }
        assert!(is_window_pinned(WindowKind::Main), "Main must stay pinned after drag");
        assert!(is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must stay pinned after drag");
        assert!(is_window_pinned(WindowKind::Equalizer), "Equalizer must stay pinned after drag");

        // 4. Simulate hiding HUD
        crate::visualizer::hide_hud_cluster();

        assert!(!is_window_pinned(WindowKind::Main), "Main must restore to unpinned after hiding HUD");
        assert!(!is_window_pinned(WindowKind::MiniPlayer), "MiniPlayer must restore to unpinned after hiding HUD");
        assert!(!is_window_pinned(WindowKind::Equalizer), "Equalizer must restore to unpinned after hiding HUD");

        // 5. Simulate exiting fullscreen directly while HUD is open
        crate::visualizer::show_hud_cluster();
        assert!(is_window_pinned(WindowKind::Main));
        crate::visualizer::restore_hud_pinned_states();
        crate::visualizer::reset_hud_main_shown();
        assert!(!is_window_pinned(WindowKind::Main), "Main must restore to unpinned on fullscreen exit");
        assert!(!is_window_pinned(WindowKind::MiniPlayer));
        assert!(!is_window_pinned(WindowKind::Equalizer));
    }

    #[test]
    fn test_hud_binary_toggle_lifecycle() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);

        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);
        set_window_pinned(WindowKind::Equalizer, false);

        assert!(!crate::visualizer::is_hud_shown());

        // Press 1: Toggle ON (All 3 windows appear in HUD)
        crate::visualizer::toggle_main_window_hud();
        assert!(crate::visualizer::is_hud_shown(), "HUD must be active on 1st toggle");
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));
        assert!(is_window_pinned(WindowKind::Equalizer));

        // Press 2: Toggle OFF (All 3 windows disappear, pure canvas)
        crate::visualizer::toggle_main_window_hud();
        assert!(!crate::visualizer::is_hud_shown(), "HUD must be inactive on 2nd toggle");
        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer));
        assert!(!is_window_pinned(WindowKind::Equalizer));

        // Press 3: Toggle ON again (1 ⇄ 0 binary toggle loop)
        crate::visualizer::toggle_main_window_hud();
        assert!(crate::visualizer::is_hud_shown(), "HUD must be active on 3rd toggle");
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));
        assert!(is_window_pinned(WindowKind::Equalizer));

        // Press 4: Toggle OFF again
        crate::visualizer::toggle_main_window_hud();
        assert!(!crate::visualizer::is_hud_shown(), "HUD must be inactive on 4th toggle");
        assert!(!is_window_pinned(WindowKind::Main));
        assert!(!is_window_pinned(WindowKind::MiniPlayer));
        assert!(!is_window_pinned(WindowKind::Equalizer));
    }

    #[test]
    fn test_hud_window_move_preserved_across_fullscreen_exit() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. 初始化 4 個拆散的浮動視窗
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 600, top: 100, right: 1000, bottom: 350 };
        let eq_r = RECT { left: 600, top: 400, right: 1000, bottom: 650 };
        let vis_r = RECT { left: 1100, top: 100, right: 1820, bottom: 820 };

        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.register_window(WindowKind::Main, 1 as _);
            mgr.register_window(WindowKind::MiniPlayer, 2 as _);
            mgr.register_window(WindowKind::Equalizer, 3 as _);
            mgr.register_window(WindowKind::Visualizer, 4 as _);

            mgr.on_user_move(WindowKind::Main, main_r);
            mgr.on_user_move(WindowKind::MiniPlayer, mini_r);
            mgr.on_user_move(WindowKind::Equalizer, eq_r);
            mgr.on_user_move(WindowKind::Visualizer, vis_r);
        });

        // 2. 視覺效果視窗進入全螢幕
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.on_visualizer_fullscreen_toggled(true);
        });

        // 3. 呼叫全螢幕 HUD (按房子)
        crate::visualizer::toggle_main_window_hud();
        assert!(crate::visualizer::is_hud_shown());

        // 4. 使用者在全螢幕 HUD 下拖曳移動 MiniPlayer 視窗到新位置（寬 460，高 300 > min_size 295）
        let new_mini_r = RECT { left: 800, top: 200, right: 1260, bottom: 500 };
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.on_user_move(WindowKind::MiniPlayer, new_mini_r);
        });

        // 5. 退出全螢幕
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.on_visualizer_fullscreen_toggled(false);
        });

        // 6. 驗證被移動的視窗「絕對保留在拖曳後的新坐標」，絕不倒退回進入全螢幕前的舊原地！
        crate::window_manager::WindowManager::with_manager(|mgr| {
            let mini = mgr.get_window(WindowKind::MiniPlayer).expect("mini exists");
            assert_eq!(mini.canonical_rect.left, 800);
            assert_eq!(mini.canonical_rect.top, 200);
            assert_eq!(mini.canonical_rect.right, 1260);
            assert_eq!(mini.canonical_rect.bottom, 500);

            if let crate::window_entity::WindowLifecycleState::Floating { rect } = mini.state {
                assert_eq!(rect.left, 800);
                assert_eq!(rect.top, 200);
            } else if let crate::window_entity::WindowLifecycleState::WorkstationDocked { rect, .. } = mini.state {
                assert_eq!(rect.left, 800);
                assert_eq!(rect.top, 200);
            } else {
                panic!("MiniPlayer state unexpected: {:?}", mini.state);
            }
        });
    }

    #[test]
    fn test_minimize_cluster_during_hud_stays_minimized_on_fullscreen_exit() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. 初始化工作站群組：Main + MiniPlayer + Equalizer + Visualizer
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        let eq_r = RECT { left: 500, top: 350, right: 900, bottom: 600 };
        let vis_r = RECT { left: 900, top: 100, right: 1620, bottom: 600 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        // 2. 視覺效果視窗進入全螢幕
        crate::visualizer::set_visualizer_fullscreen_mock(true);
        break_dock_bond();

        // 驗證 Visualizer 進入全螢幕後與工作站群組徹底解耦（無任何殘留 edges）
        assert_eq!(get_window_role(WindowKind::Visualizer), WindowRole::Standalone);

        // 3. 呼叫全螢幕 HUD (按 🏠)
        crate::visualizer::toggle_main_window_hud();
        assert!(crate::visualizer::is_hud_shown());

        // 4. 最小化工作站群組（使用者在全螢幕 HUD 點擊最小化）
        unsafe {
            collapse_cluster_animated(WindowKind::Main);
        }
        assert_eq!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);

        // 5. 退出全螢幕
        crate::visualizer::set_visualizer_fullscreen_mock(false);
        crate::window_manager::WindowManager::with_manager(|mgr| {
            mgr.on_visualizer_fullscreen_toggled(false);
        });

        // 6. 核心不變量驗證：退出全螢幕後，工作站群組【絕對保持最小化】，嚴禁詐屍展開！
        assert_eq!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);
        assert!(!crate::visualizer::is_hud_shown());
    }

    #[test]
    fn test_home_summon_joins_mini_cluster_with_visualizer() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. 設置工作站拆分後的殘留群組：MiniPlayer + Equalizer + Visualizer
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 };
        let vis_r = RECT { left: 1020, top: 100, right: 1480, bottom: 780 };

        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        // 驗證三者已成一組，且主視窗不在其中
        let mini_cluster = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(mini_cluster.len(), 3);
        assert!(mini_cluster.contains(&WindowKind::MiniPlayer));
        assert!(mini_cluster.contains(&WindowKind::Equalizer));
        assert!(mini_cluster.contains(&WindowKind::Visualizer));
        assert!(!mini_cluster.contains(&WindowKind::Main));

        // 2. 主視窗隱藏於托盤
        crate::tray::IS_TRAY_HIDDEN.store(true, Ordering::SeqCst);
        let orig_main_r = RECT { left: -32000, top: -32000, right: -31540, bottom: -31320 };
        update_known_rect(WindowKind::Main, orig_main_r);

        // 3. 模擬使用者在 MiniPlayer 上點擊 🏠 (Home 房子圖標)
        // 模擬主視窗回到目標停靠位 (停在群組最左側即 MiniPlayer 左邊 100..560)
        let summoned_main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        update_known_rect(WindowKind::Main, summoned_main_r);
        crate::tray::IS_TRAY_HIDDEN.store(false, Ordering::SeqCst);

        // 執行主視窗與 MiniPlayer 的 dock 關係綁定
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &summoned_main_r);

        // 4. 驗證全體 4 個視窗全部成功合成同一個群組！
        let final_cluster_from_main = get_cluster_members(WindowKind::Main);
        let final_cluster_from_mini = get_cluster_members(WindowKind::MiniPlayer);
        let final_cluster_from_vis = get_cluster_members(WindowKind::Visualizer);

        assert_eq!(final_cluster_from_main.len(), 4, "Main 必須與原先三視窗群組合併為 4 視窗大群組");
        assert_eq!(final_cluster_from_mini.len(), 4);
        assert_eq!(final_cluster_from_vis.len(), 4);

        assert!(final_cluster_from_mini.contains(&WindowKind::Main));
        assert!(final_cluster_from_mini.contains(&WindowKind::Visualizer));
        assert!(final_cluster_from_mini.contains(&WindowKind::Equalizer));
        assert!(final_cluster_from_mini.contains(&WindowKind::MiniPlayer));
    }

    #[test]
    fn test_per_cluster_collapsed_state_isolation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Group 1: Main + Equalizer
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        let eq_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        set_dock_relation(WindowKind::Equalizer, WindowKind::Main, &eq_r, &main_r);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::Main);

        // 2. Group 2: MiniPlayer + Visualizer
        let mini_r = RECT { left: 1100, top: 100, right: 1560, bottom: 440 };
        let vis_r = RECT { left: 1100, top: 440, right: 1560, bottom: 780 };
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::MiniPlayer);

        // 3. Both groups collapse independently
        set_cluster_state(WindowKind::Main, ClusterState::Minimized);
        set_cluster_state(WindowKind::MiniPlayer, ClusterState::Minimized);

        assert_eq!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);
        assert_eq!(get_cluster_state(WindowKind::MiniPlayer), ClusterState::Minimized);

        // 4. Restore Group 2 (MiniPlayer) first
        assert!(can_start_cluster_restore(WindowKind::MiniPlayer));
        assert_ne!(get_cluster_state(WindowKind::MiniPlayer), ClusterState::Minimized);

        // Critical check: Group 1 (Main) MUST STILL BE MINIMIZED! No state leak or overwrite!
        assert_eq!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);

        // 5. Restore Group 1 (Main)
        assert!(can_start_cluster_restore(WindowKind::Main));
        assert_ne!(get_cluster_state(WindowKind::Main), ClusterState::Minimized);
    }

    #[test]
    fn test_cluster_fsm_lifecycle_and_timeout() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let root = WindowKind::Main;
        assert_eq!(get_cluster_state(root), ClusterState::Normal);

        // 1. Can start minimize
        assert!(can_start_cluster_minimize(root));
        assert!(is_kind_cluster_minimizing(root));
        assert!(is_cluster_minimizing());

        // While minimizing, cannot start another minimize
        assert!(!can_start_cluster_minimize(root));

        // Finish minimize
        finish_cluster_minimize(root);
        assert_eq!(get_cluster_state(root), ClusterState::Minimized);
        assert!(!is_cluster_minimizing());

        // 2. Can start restore
        assert!(can_start_cluster_restore(root));
        assert!(is_kind_cluster_restoring(root));
        assert!(is_cluster_restoring());

        // Finish restore
        finish_cluster_restore(root);
        assert_eq!(get_cluster_state(root), ClusterState::Normal);
        assert!(!is_cluster_restoring());

        // 3. Timeout self-healing simulation (> 450ms)
        let past = std::time::Instant::now() - std::time::Duration::from_millis(500);
        set_cluster_state(root, ClusterState::Minimizing(past));
        assert_eq!(get_cluster_state(root), ClusterState::Minimized, "Minimizing past 450ms must auto-resolve to Minimized");

        set_cluster_state(root, ClusterState::Restoring(past));
        assert_eq!(get_cluster_state(root), ClusterState::Normal, "Restoring past 450ms must auto-resolve to Normal");
    }

    #[test]
    fn test_rebuild_dock_tree_preserves_minimized_cluster_bonds() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Setup layout: Main alone, Group 2: MiniPlayer, Equalizer, Visualizer
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 600, top: 100, right: 1000, bottom: 350 };
        let eq_r = RECT { left: 600, top: 350, right: 1000, bottom: 600 };
        let vis_r = RECT { left: 1000, top: 100, right: 1400, bottom: 600 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        // Group 2 dock relations: Equalizer -> MiniPlayer, Visualizer -> MiniPlayer
        set_dock_relation(WindowKind::Equalizer, WindowKind::MiniPlayer, &eq_r, &mini_r);
        set_dock_relation(WindowKind::Visualizer, WindowKind::MiniPlayer, &vis_r, &mini_r);

        let g2_members = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(g2_members.len(), 3, "Group 2 must have 3 members initially");
        assert!(g2_members.contains(&WindowKind::MiniPlayer));
        assert!(g2_members.contains(&WindowKind::Equalizer));
        assert!(g2_members.contains(&WindowKind::Visualizer));

        // 2. Step 2: Minimize Group 2
        set_cluster_state(WindowKind::MiniPlayer, ClusterState::Minimized);

        // 3. Step 3: Minimize Main Window (calls minimize_window_solo which invokes rebuild_dock_tree)
        unsafe {
            rebuild_dock_tree();
        }

        // 4. Verify that Group 2's dock relations were NOT wiped out by rebuilding while minimized!
        let g2_members_after = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(
            g2_members_after.len(),
            3,
            "Rebuilding dock tree must NOT wipe out minimized cluster members! Found: {:?}",
            g2_members_after
        );
        assert!(g2_members_after.contains(&WindowKind::Equalizer));
        assert!(g2_members_after.contains(&WindowKind::Visualizer));

        // 5. Restore Group 2
        set_cluster_state(WindowKind::MiniPlayer, ClusterState::Normal);
        let g2_members_restored = get_cluster_members(WindowKind::MiniPlayer);
        assert_eq!(g2_members_restored.len(), 3);
    }

    #[test]
    fn test_cluster_fsm_transitions_and_handover() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        let root = WindowKind::MiniPlayer;

        // 1. Strict Transition testing
        assert_eq!(get_cluster_state(root), ClusterState::Normal);

        // Normal -> Minimizing
        assert!(can_start_cluster_minimize(root));
        assert!(is_kind_cluster_minimizing(root));

        // Duplicate minimize rejected while active minimizing
        assert!(!can_start_cluster_minimize(root));

        // Minimizing -> Minimized
        finish_cluster_minimize(root);
        assert_eq!(get_cluster_state(root), ClusterState::Minimized);

        // Minimized -> Restoring
        assert!(can_start_cluster_restore(root));
        assert!(is_kind_cluster_restoring(root));

        // Restoring -> Normal
        finish_cluster_restore(root);
        assert_eq!(get_cluster_state(root), ClusterState::Normal);

        // 2. Topology Handover: Root change simulation
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);

        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);

        let main_state = get_cluster_state(WindowKind::Main);
        assert_eq!(main_state, ClusterState::Normal);

        // Detach MiniPlayer: becomes independent root
        break_dock_relation(WindowKind::MiniPlayer);

        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::MiniPlayer);
        assert_eq!(get_cluster_state(WindowKind::MiniPlayer), ClusterState::Normal);
    }

    #[test]
    fn test_quick_opacity_toggle_lifecycle() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initial State: Opacity = 100%, Unpinned (State 1: Normal)
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(!is_window_pinned(WindowKind::Main));

        // 2. Solo Quick Toggle at 100% -> should jump directly to 30% and auto-lock pinned (State 3: Ghost)
        let next1 = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next1, 30);
        assert!((get_window_opacity(WindowKind::Main) - 0.30).abs() < 0.001);
        assert!(is_window_pinned(WindowKind::Main), "Ghost mode must auto-lock pinned!");

        // 3. Solo Quick Toggle at 30% -> should jump back to 100% (State 2: Pinned Solid)
        let next2 = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next2, 100);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(is_window_pinned(WindowKind::Main));

        // 4. Test Single-click Pin on Ghost Mode (State 3 -> State 1 direct exit):
        // Re-enter 30% Ghost Mode
        let next3 = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next3, 30);
        assert!(is_window_pinned(WindowKind::Main));
        // Single-click Pin to unpin while in Ghost mode -> must restore 100% and unpin (State 1), eliminating dead state!
        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0, "Unpinning from ghost mode must restore 100% opacity");
        assert!(!is_window_pinned(WindowKind::Main), "Window must be unpinned");
        clear_ghost_exit_record();

        // 5. Cluster Quick Toggle across docked windows (100% <-> 30%)
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        // Reset cluster to 100% unpinned
        set_cluster_opacity(WindowKind::Main, 1.0);
        set_window_pinned(WindowKind::Main, false);
        set_window_pinned(WindowKind::MiniPlayer, false);

        // Quick toggle cluster -> all cluster members switch to 30% and auto-lock pinned!
        let c_next1 = toggle_cluster_opacity_quick(WindowKind::Main);
        assert_eq!(c_next1, 30);
        assert!((get_window_opacity(WindowKind::Main) - 0.30).abs() < 0.001);
        assert!((get_window_opacity(WindowKind::MiniPlayer) - 0.30).abs() < 0.001);
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));

        // Quick toggle cluster again -> all switch back to 100%
        let c_next2 = toggle_cluster_opacity_quick(WindowKind::Main);
        assert_eq!(c_next2, 100);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert_eq!(get_window_opacity(WindowKind::MiniPlayer), 1.0);

        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_solution_c_titlebar_tap_to_wake_and_pure_pin() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Initial State 1: 100% Solid, Unpinned
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(!is_window_pinned(WindowKind::Main));

        // 2. Pure Pin Toggle (State 1 <-> State 2)
        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert!(is_window_pinned(WindowKind::Main), "Click Pin in State 1 must transition to State 2 (Pinned)");
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);

        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert!(!is_window_pinned(WindowKind::Main), "Click Pin in State 2 must transition back to State 1 (Unpinned)");
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);

        // 3. Titlebar Double-click: toggles State 2 <-> State 3
        let next = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(next, 30, "Titlebar double-click enters State 3 (30% Ghost, Pinned)");
        assert!(is_window_pinned(WindowKind::Main));

        // 4. Titlebar Symmetric Double-Click in State 3 -> State 2 (100% Solid, Pinned = true)
        let back = toggle_single_window_opacity_quick(WindowKind::Main);
        assert_eq!(back, 100, "Symmetric double-click toggle restores 100% opacity");
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert!(is_window_pinned(WindowKind::Main), "Symmetric double-click must PRESERVE pinned state (State 2)!");

        // Also verify direct wake_single_window_ghost helper
        toggle_single_window_opacity_quick(WindowKind::Main); // Re-enter State 3
        wake_single_window_ghost(WindowKind::Main);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0, "Wake must restore 100% opacity");
        assert!(is_window_pinned(WindowKind::Main), "Wake must PRESERVE pinned state (State 2)!");

        // 5. In State 3, Single-click Pin (Unpin / 拔釘) -> State 1 (100% Solid, Unpinned)
        toggle_single_window_opacity_quick(WindowKind::Main); // Re-enter State 3
        assert_eq!(get_window_opacity(WindowKind::Main), 0.3);
        assert!(is_window_pinned(WindowKind::Main));

        unsafe {
            toggle_window_pin(WindowKind::Main);
        }
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0, "Unpin from State 3 must restore 100% opacity");
        assert!(!is_window_pinned(WindowKind::Main), "Unpin from State 3 must unpin to State 1");

        // 6. Cluster Tap-to-Wake across docked windows
        let main_r = RECT { left: 100, top: 100, right: 500, bottom: 600 };
        let mini_r = RECT { left: 500, top: 100, right: 900, bottom: 350 };
        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        set_dock_relation(WindowKind::MiniPlayer, WindowKind::Main, &mini_r, &main_r);

        // Enter State 3 for cluster
        toggle_cluster_opacity_quick(WindowKind::Main);
        assert_eq!(get_window_opacity(WindowKind::Main), 0.3);
        assert_eq!(get_window_opacity(WindowKind::MiniPlayer), 0.3);
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));

        // Cluster Tap-to-Wake -> all cluster members become 100% and stay pinned (State 2)
        wake_cluster_ghost(WindowKind::Main);
        assert_eq!(get_window_opacity(WindowKind::Main), 1.0);
        assert_eq!(get_window_opacity(WindowKind::MiniPlayer), 1.0);
        assert!(is_window_pinned(WindowKind::Main));
        assert!(is_window_pinned(WindowKind::MiniPlayer));

        break_dock_relation(WindowKind::MiniPlayer);
    }

    #[test]
    fn test_squashed_rect_clamp_and_root_priority_preservation() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_all();

        // 1. Abnormal squashed rect (Win32 iconic 180x27)
        let squashed_main = RECT { left: 400, top: 300, right: 580, bottom: 327 };
        update_known_rect(WindowKind::Main, squashed_main);

        // get_window_normal_rect must clamp to min_size bounds (380x500) and preserve user coordinates
        let norm_r = unsafe { get_window_normal_rect(WindowKind::Main) };
        assert!(norm_r.right - norm_r.left >= 380, "Main width must be at least min_size width (380)");
        assert!(norm_r.bottom - norm_r.top >= 500, "Main height must be at least min_size height (500)");

        // 2. Priority check: Main (1) < MiniPlayer (2) < Equalizer (3) < Visualizer (4)
        assert_eq!(WindowKind::Main.priority(), 1);
        assert_eq!(WindowKind::MiniPlayer.priority(), 2);
        assert_eq!(WindowKind::Equalizer.priority(), 3);
        assert_eq!(WindowKind::Visualizer.priority(), 4);

        // 3. Cluster topology: When all 4 windows are connected, Main must ALWAYS be root
        let main_r = RECT { left: 100, top: 100, right: 560, bottom: 780 };
        let mini_r = RECT { left: 560, top: 100, right: 1020, bottom: 440 };
        let eq_r = RECT { left: 560, top: 440, right: 1020, bottom: 780 };
        let vis_r = RECT { left: 1020, top: 100, right: 1740, bottom: 780 };

        update_known_rect(WindowKind::Main, main_r);
        update_known_rect(WindowKind::MiniPlayer, mini_r);
        update_known_rect(WindowKind::Equalizer, eq_r);
        update_known_rect(WindowKind::Visualizer, vis_r);

        // Add edges: Main <-> MiniPlayer, MiniPlayer <-> Equalizer, MiniPlayer <-> Visualizer
        add_dock_edge(WindowKind::Main, WindowKind::MiniPlayer);
        add_dock_edge(WindowKind::MiniPlayer, WindowKind::Equalizer);
        add_dock_edge(WindowKind::MiniPlayer, WindowKind::Visualizer);

        let active = vec![WindowKind::Main, WindowKind::MiniPlayer, WindowKind::Equalizer, WindowKind::Visualizer];
        let edges = vec![
            (WindowKind::Main, WindowKind::MiniPlayer),
            (WindowKind::MiniPlayer, WindowKind::Equalizer),
            (WindowKind::MiniPlayer, WindowKind::Visualizer),
        ];

        let rect_map = [
            (WindowKind::Main, main_r),
            (WindowKind::MiniPlayer, mini_r),
            (WindowKind::Equalizer, eq_r),
            (WindowKind::Visualizer, vis_r),
        ];

        let tree = build_dock_tree_from_edges_and_rects(&active, &edges, |k| {
            rect_map.iter().find(|(kind, _)| *kind == k).map(|(_, r)| *r)
        });

        with_dock_tree(|t| *t = tree);

        // Root of the connected component must be Main, NEVER Visualizer
        assert_eq!(get_component_root(WindowKind::Main), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::MiniPlayer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::Equalizer), WindowKind::Main);
        assert_eq!(get_component_root(WindowKind::Visualizer), WindowKind::Main);

        // Visualizer is slave bonded, not master
        assert_eq!(get_window_role(WindowKind::Visualizer), WindowRole::ClusterSlave);
        assert_eq!(get_window_role(WindowKind::Main), WindowRole::ClusterMaster);
    }
