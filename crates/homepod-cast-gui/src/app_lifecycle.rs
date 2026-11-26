//! Application Top-Level Lifecycle Finite State Machine (AppLifecycleFsm).
//!
//! Provides a mathematically rigorous, deterministic state machine governing
//! the overall lifecycle of `win2pod.exe`, from initialization, UI visibility,
//! audio streaming, OS power suspend/resume, to ordered multi-stage teardown.

use std::sync::RwLock;
use std::time::Instant;

/// Top-level lifecycle states of the `win2pod.exe` process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLifecycleState {
    /// Bootstrapping: argument parsing, single-instance mutex check, logging initialization.
    Initializing,
    /// GUI is visible on screen, interactive and active.
    ForegroundUI,
    /// GUI is hidden or minimized to system tray, running silently in background.
    StandbyTray,
    /// Actively transmitting or receiving audio streams.
    Streaming {
        ui_visible: bool,
        target_name: String,
    },
    /// Windows OS sent a sleep/suspend notification (`WM_POWERBROADCAST` / `PBT_APMSUSPEND`).
    SystemSuspending {
        resumed_ui_visible: bool,
    },
    /// Multi-stage graceful shutdown in progress. All new commands rejected.
    Draining {
        start_time: Instant,
    },
    /// Teardown pipeline completed, resources freed, ready for clean exit.
    Terminated,
}

impl Default for AppLifecycleState {
    fn default() -> Self {
        Self::Initializing
    }
}

/// Events driving transitions in the application lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLifecycleEvent {
    /// Initial launch finished, specifying whether starting minimized to tray.
    LaunchComplete { start_minimized: bool },
    /// User or shortcut requested bringing main window to front.
    ShowUI,
    /// User closed/minimized window to tray.
    HideToTray,
    /// Audio stream started (transmitter or receiver).
    StreamStarted { target_name: String },
    /// Audio stream stopped.
    StreamStopped,
    /// System suspend notification arrived from Windows.
    SystemSuspend,
    /// System resume notification arrived from Windows.
    SystemResume,
    /// User or OS requested application shutdown (e.g. GuiCmd::Quit, WM_CLOSE, WM_ENDSESSION).
    QuitRequested,
    /// Teardown pipeline finished releasing hardware and child processes.
    TeardownComplete,
}

/// Ordered stages of the graceful teardown pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppTeardownStage {
    /// Stage 1: Reject new requests and stop active streams.
    StopActiveStreams,
    /// Stage 2: Hardware restoration (unmute native speaker, restore default audio endpoint).
    RestoreAudioHardware,
    /// Stage 3: Gracefully shut down background sidecar daemon.
    ShutdownSidecar,
    /// Stage 4: Close subsidiary window message pumps and release power locks.
    CloseSubsidiaryPumps,
    /// Stage 5: Final process exit.
    ExitProcess,
}

/// Actions produced by application lifecycle transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLifecycleAction {
    None,
    RejectCommand { reason: String },
    InitiateTeardown(Vec<AppTeardownStage>),
}

/// Deterministic Finite State Machine for `win2pod.exe`.
#[derive(Debug, Clone)]
pub struct AppLifecycleFsm {
    state: AppLifecycleState,
}

impl AppLifecycleFsm {
    pub fn new() -> Self {
        Self {
            state: AppLifecycleState::Initializing,
        }
    }

    pub fn state(&self) -> &AppLifecycleState {
        &self.state
    }

    pub fn is_draining(&self) -> bool {
        matches!(self.state, AppLifecycleState::Draining { .. } | AppLifecycleState::Terminated)
    }

    pub fn is_ui_visible(&self) -> bool {
        match &self.state {
            AppLifecycleState::ForegroundUI => true,
            AppLifecycleState::Streaming { ui_visible, .. } => *ui_visible,
            _ => false,
        }
    }

    pub fn is_streaming(&self) -> bool {
        matches!(self.state, AppLifecycleState::Streaming { .. })
    }

    /// Pure transition function: (CurrentState, Event) -> Result<(NextState, Action), String>
    pub fn transition(&mut self, event: AppLifecycleEvent) -> Result<AppLifecycleAction, String> {
        let (next_state, action) = match (&self.state, event) {
            // Guard: in Draining or Terminated state, reject any new commands!
            (AppLifecycleState::Draining { .. } | AppLifecycleState::Terminated, AppLifecycleEvent::QuitRequested) => {
                return Ok(AppLifecycleAction::None); // Already draining, ignore duplicate quit
            }
            (AppLifecycleState::Draining { .. } | AppLifecycleState::Terminated, AppLifecycleEvent::TeardownComplete) => {
                (AppLifecycleState::Terminated, AppLifecycleAction::None)
            }
            (AppLifecycleState::Draining { .. } | AppLifecycleState::Terminated, evt) => {
                return Err(format!("Cannot process event {:?}: Application shutdown is in progress", evt));
            }

            // Normal Quit triggers from any active state
            (_, AppLifecycleEvent::QuitRequested) => {
                (
                    AppLifecycleState::Draining { start_time: Instant::now() },
                    AppLifecycleAction::InitiateTeardown(vec![
                        AppTeardownStage::StopActiveStreams,
                        AppTeardownStage::RestoreAudioHardware,
                        AppTeardownStage::ShutdownSidecar,
                        AppTeardownStage::CloseSubsidiaryPumps,
                        AppTeardownStage::ExitProcess,
                    ]),
                )
            }

            // Launch complete
            (AppLifecycleState::Initializing, AppLifecycleEvent::LaunchComplete { start_minimized }) => {
                if start_minimized {
                    (AppLifecycleState::StandbyTray, AppLifecycleAction::None)
                } else {
                    (AppLifecycleState::ForegroundUI, AppLifecycleAction::None)
                }
            }

            // UI Visibility changes
            (AppLifecycleState::StandbyTray, AppLifecycleEvent::ShowUI) => {
                (AppLifecycleState::ForegroundUI, AppLifecycleAction::None)
            }
            (AppLifecycleState::ForegroundUI, AppLifecycleEvent::HideToTray) => {
                (AppLifecycleState::StandbyTray, AppLifecycleAction::None)
            }
            (AppLifecycleState::ForegroundUI, AppLifecycleEvent::ShowUI) => {
                (AppLifecycleState::ForegroundUI, AppLifecycleAction::None)
            }
            (AppLifecycleState::StandbyTray, AppLifecycleEvent::HideToTray) => {
                (AppLifecycleState::StandbyTray, AppLifecycleAction::None)
            }

            // Streaming transitions
            (AppLifecycleState::ForegroundUI, AppLifecycleEvent::StreamStarted { target_name }) => {
                (AppLifecycleState::Streaming { ui_visible: true, target_name }, AppLifecycleAction::None)
            }
            (AppLifecycleState::StandbyTray, AppLifecycleEvent::StreamStarted { target_name }) => {
                (AppLifecycleState::Streaming { ui_visible: false, target_name }, AppLifecycleAction::None)
            }
            (AppLifecycleState::Streaming { target_name, .. }, AppLifecycleEvent::ShowUI) => {
                (AppLifecycleState::Streaming { ui_visible: true, target_name: target_name.clone() }, AppLifecycleAction::None)
            }
            (AppLifecycleState::Streaming { target_name, .. }, AppLifecycleEvent::HideToTray) => {
                (AppLifecycleState::Streaming { ui_visible: false, target_name: target_name.clone() }, AppLifecycleAction::None)
            }
            (AppLifecycleState::Streaming { ui_visible, .. }, AppLifecycleEvent::StreamStopped) => {
                if *ui_visible {
                    (AppLifecycleState::ForegroundUI, AppLifecycleAction::None)
                } else {
                    (AppLifecycleState::StandbyTray, AppLifecycleAction::None)
                }
            }

            // System suspend & resume
            (AppLifecycleState::ForegroundUI, AppLifecycleEvent::SystemSuspend) => {
                (AppLifecycleState::SystemSuspending { resumed_ui_visible: true }, AppLifecycleAction::None)
            }
            (AppLifecycleState::StandbyTray, AppLifecycleEvent::SystemSuspend) => {
                (AppLifecycleState::SystemSuspending { resumed_ui_visible: false }, AppLifecycleAction::None)
            }
            (AppLifecycleState::Streaming { ui_visible, .. }, AppLifecycleEvent::SystemSuspend) => {
                (AppLifecycleState::SystemSuspending { resumed_ui_visible: *ui_visible }, AppLifecycleAction::None)
            }
            (AppLifecycleState::SystemSuspending { resumed_ui_visible }, AppLifecycleEvent::SystemResume) => {
                if *resumed_ui_visible {
                    (AppLifecycleState::ForegroundUI, AppLifecycleAction::None)
                } else {
                    (AppLifecycleState::StandbyTray, AppLifecycleAction::None)
                }
            }

            (cur, evt) => {
                return Err(format!("Invalid application lifecycle transition from {:?} via {:?}", cur, evt));
            }
        };

        self.state = next_state;
        Ok(action)
    }
}

// Global thread-safe singleton manager for AppLifecycleFsm
static GLOBAL_APP_FSM: RwLock<Option<AppLifecycleFsm>> = RwLock::new(None);

/// Initialize the global AppLifecycleFsm.
pub fn init_app_lifecycle(start_minimized: bool) {
    let mut guard = GLOBAL_APP_FSM.write().unwrap();
    let mut fsm = AppLifecycleFsm::new();
    let _ = fsm.transition(AppLifecycleEvent::LaunchComplete { start_minimized });
    *guard = Some(fsm);
}

/// Query current global AppLifecycleState.
pub fn get_app_lifecycle_state() -> AppLifecycleState {
    let guard = GLOBAL_APP_FSM.read().unwrap();
    guard.as_ref().map(|f| f.state().clone()).unwrap_or(AppLifecycleState::Initializing)
}

/// Check if application is in draining/teardown state.
pub fn is_app_draining() -> bool {
    let guard = GLOBAL_APP_FSM.read().unwrap();
    guard.as_ref().map(|f| f.is_draining()).unwrap_or(false)
}

/// Dispatch an event to the global AppLifecycleFsm.
pub fn dispatch_app_lifecycle_event(event: AppLifecycleEvent) -> Result<AppLifecycleAction, String> {
    let mut guard = GLOBAL_APP_FSM.write().unwrap();
    if let Some(fsm) = guard.as_mut() {
        fsm.transition(event)
    } else {
        // Auto-initialize if not yet created
        let mut fsm = AppLifecycleFsm::new();
        let res = fsm.transition(event);
        *guard = Some(fsm);
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_app_lifecycle_flow() {
        let mut fsm = AppLifecycleFsm::new();
        assert_eq!(*fsm.state(), AppLifecycleState::Initializing);

        // 1. Launch normally -> ForegroundUI
        let act = fsm.transition(AppLifecycleEvent::LaunchComplete { start_minimized: false }).unwrap();
        assert_eq!(act, AppLifecycleAction::None);
        assert_eq!(*fsm.state(), AppLifecycleState::ForegroundUI);
        assert!(fsm.is_ui_visible());
        assert!(!fsm.is_streaming());

        // 2. Hide to Tray -> StandbyTray
        fsm.transition(AppLifecycleEvent::HideToTray).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::StandbyTray);
        assert!(!fsm.is_ui_visible());

        // 3. Show UI -> ForegroundUI
        fsm.transition(AppLifecycleEvent::ShowUI).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::ForegroundUI);

        // 4. Start Streaming -> Streaming (ui_visible: true)
        fsm.transition(AppLifecycleEvent::StreamStarted { target_name: "Living Room".into() }).unwrap();
        assert!(matches!(fsm.state(), AppLifecycleState::Streaming { ui_visible: true, target_name } if target_name == "Living Room"));
        assert!(fsm.is_streaming());

        // 5. Hide to Tray while streaming
        fsm.transition(AppLifecycleEvent::HideToTray).unwrap();
        assert!(matches!(fsm.state(), AppLifecycleState::Streaming { ui_visible: false, .. }));
        assert!(fsm.is_streaming());

        // 6. Stop streaming while in tray -> StandbyTray
        fsm.transition(AppLifecycleEvent::StreamStopped).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::StandbyTray);
        assert!(!fsm.is_streaming());

        // 7. Quit Requested -> Draining with multi-stage actions
        let act = fsm.transition(AppLifecycleEvent::QuitRequested).unwrap();
        assert!(matches!(act, AppLifecycleAction::InitiateTeardown(stages) if stages.len() == 5));
        assert!(fsm.is_draining());

        // 8. Teardown Complete -> Terminated
        fsm.transition(AppLifecycleEvent::TeardownComplete).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::Terminated);
    }

    #[test]
    fn test_draining_guards_against_reentrant_and_spurious_events() {
        let mut fsm = AppLifecycleFsm::new();
        fsm.transition(AppLifecycleEvent::LaunchComplete { start_minimized: false }).unwrap();

        // Initiate teardown
        let act = fsm.transition(AppLifecycleEvent::QuitRequested).unwrap();
        assert!(matches!(act, AppLifecycleAction::InitiateTeardown(_)));

        // Duplicate QuitRequested is a clean no-op
        let act2 = fsm.transition(AppLifecycleEvent::QuitRequested).unwrap();
        assert_eq!(act2, AppLifecycleAction::None);

        // Any regular commands while draining are strictly rejected with an error!
        let err1 = fsm.transition(AppLifecycleEvent::ShowUI).unwrap_err();
        assert!(err1.contains("shutdown is in progress"));

        let err2 = fsm.transition(AppLifecycleEvent::StreamStarted { target_name: "Dining".into() }).unwrap_err();
        assert!(err2.contains("shutdown is in progress"));
    }

    #[test]
    fn test_system_suspend_and_resume() {
        let mut fsm = AppLifecycleFsm::new();
        fsm.transition(AppLifecycleEvent::LaunchComplete { start_minimized: false }).unwrap();

        // Suspend when UI was visible
        fsm.transition(AppLifecycleEvent::SystemSuspend).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::SystemSuspending { resumed_ui_visible: true });

        // Resume restores ForegroundUI
        fsm.transition(AppLifecycleEvent::SystemResume).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::ForegroundUI);

        // Suspend when UI was hidden in tray
        fsm.transition(AppLifecycleEvent::HideToTray).unwrap();
        fsm.transition(AppLifecycleEvent::SystemSuspend).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::SystemSuspending { resumed_ui_visible: false });

        // Resume restores StandbyTray
        fsm.transition(AppLifecycleEvent::SystemResume).unwrap();
        assert_eq!(*fsm.state(), AppLifecycleState::StandbyTray);
    }
}
