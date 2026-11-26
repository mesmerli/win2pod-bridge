//! Power Management & Sleep Suppression Finite State Machine (FSM)
//!
//! Windows `SetThreadExecutionState` is strictly thread-affine (tied to the specific
//! OS thread that called it). Calling `ES_SYSTEM_REQUIRED` on one Tokio worker thread
//! and subsequently calling `ES_CONTINUOUS` on a different worker thread leaves the
//! power request stranded in the Windows kernel Power Manager (`powercfg /requests`),
//! preventing the PC from sleeping indefinitely.
//!
//! To solve this deterministically, `PowerArbiter` hosts a dedicated, persistent OS
//! thread (`win2pod-power-arbiter`) that alone interacts with the Windows Power API,
//! driven by a pure, formally verifiable state machine (`PowerStateMachine`).

use std::sync::mpsc::{channel, Sender};
use serde::{Deserialize, Serialize};
use tracing::info;

/// High-level power management state of Win2Pod
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    /// Normal standby / idle. Windows is fully free to sleep and turn off display.
    Standby,

    /// Streaming / AoMode is connected, but silent for >30s or muted (Eco Mode).
    /// Sleep suppression is released so Windows idle sleep timer accumulates naturally.
    EcoSilent,

    /// Streaming / AoMode is active with real audio output, and user enabled "Prevent Sleep".
    /// System sleep is suppressed (`ES_SYSTEM_REQUIRED`).
    AwakeStreaming,

    /// Visualizer window is in Fullscreen presentation mode.
    /// Both display sleep and system sleep are suppressed (`ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED`).
    AwakePresentation,

    /// Operating system signaled suspend / sleep transition. All activity frozen.
    Suspended,
}

impl PowerState {
    pub fn is_sleep_suppressed(&self) -> bool {
        matches!(self, Self::AwakeStreaming | Self::AwakePresentation)
    }

    pub fn is_display_required(&self) -> bool {
        matches!(self, Self::AwakePresentation)
    }

    pub fn is_eco_mode(&self) -> bool {
        matches!(self, Self::EcoSilent)
    }
}

/// Physical action commanded to the dedicated Win32 power thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    AllowSleep,
    SetSystemAwake,
    SetDisplayAndSystemAwake,
}

/// Events that drive state machine transitions
#[derive(Debug, Clone, PartialEq)]
pub enum PowerEvent {
    /// User toggled "Prevent Sleep" checkbox in GUI
    UserPreventSleepChanged(bool),

    /// Audio routing mode changed (AirPlay connected, Sidecar connected, AoMode active, or Idle)
    RouteActiveChanged(bool),

    /// Audio activity updated (checked against 30s silence threshold)
    AudioActivityChanged { is_active: bool, peak_level: f32 },

    /// Master speaker / source mute toggled
    MuteChanged(bool),

    /// Visualizer window entered or exited fullscreen mode
    VisualizerFullscreenChanged(bool),

    /// OS suspend broadcast received (WM_POWERBROADCAST PBT_APMSUSPEND)
    SystemSuspend,

    /// OS resume broadcast received (PBT_APMRESUMEAUTOMATIC / PBT_APMRESUMESUSPEND)
    SystemResume,
}

/// Pure state machine governing sleep suppression transitions
#[derive(Debug, Clone, PartialEq)]
pub struct PowerStateMachine {
    current_state: PowerState,
    prevent_sleep: bool,
    is_route_active: bool,
    is_audio_active: bool,
    is_muted: bool,
    is_fullscreen: bool,
    is_suspended: bool,
}

impl PowerStateMachine {
    pub fn new(initial_prevent_sleep: bool) -> Self {
        let mut fsm = Self {
            current_state: PowerState::Standby,
            prevent_sleep: initial_prevent_sleep,
            is_route_active: false,
            is_audio_active: false,
            is_muted: false,
            is_fullscreen: false,
            is_suspended: false,
        };
        fsm.current_state = fsm.compute_state();
        fsm
    }

    pub fn current_state(&self) -> PowerState {
        self.current_state
    }

    /// Evaluates the target `PowerState` from the current inputs
    fn compute_state(&self) -> PowerState {
        if self.is_suspended {
            PowerState::Suspended
        } else if self.is_fullscreen {
            PowerState::AwakePresentation
        } else if self.is_route_active && self.prevent_sleep {
            if self.is_audio_active && !self.is_muted {
                PowerState::AwakeStreaming
            } else {
                PowerState::EcoSilent
            }
        } else {
            PowerState::Standby
        }
    }

    /// Feeds an event into the state machine and returns `(new_state, Option<PowerAction>)`.
    /// `Some(PowerAction)` is returned only when the underlying hardware action changes.
    pub fn handle_event(&mut self, event: PowerEvent) -> (PowerState, Option<PowerAction>) {
        let prev_state = self.current_state;

        match event {
            PowerEvent::UserPreventSleepChanged(val) => self.prevent_sleep = val,
            PowerEvent::RouteActiveChanged(val) => self.is_route_active = val,
            PowerEvent::AudioActivityChanged { is_active, .. } => self.is_audio_active = is_active,
            PowerEvent::MuteChanged(val) => self.is_muted = val,
            PowerEvent::VisualizerFullscreenChanged(val) => self.is_fullscreen = val,
            PowerEvent::SystemSuspend => self.is_suspended = true,
            PowerEvent::SystemResume => self.is_suspended = false,
        }

        let next_state = self.compute_state();
        self.current_state = next_state;

        let prev_action = Self::state_to_action(prev_state);
        let next_action = Self::state_to_action(next_state);

        let action_change = if prev_action != next_action {
            Some(next_action)
        } else {
            None
        };

        (next_state, action_change)
    }

    fn state_to_action(state: PowerState) -> PowerAction {
        match state {
            PowerState::AwakePresentation => PowerAction::SetDisplayAndSystemAwake,
            PowerState::AwakeStreaming => PowerAction::SetSystemAwake,
            PowerState::Standby | PowerState::EcoSilent | PowerState::Suspended => PowerAction::AllowSleep,
        }
    }
}

/// Central arbiter running a dedicated persistent OS thread for thread-safe Win32 Power APIs
pub struct PowerArbiter {
    fsm: PowerStateMachine,
    action_tx: Option<Sender<PowerAction>>,
}

impl PowerArbiter {
    pub fn new(prevent_sleep: bool) -> Self {
        let fsm = PowerStateMachine::new(prevent_sleep);
        let (action_tx, action_rx) = channel::<PowerAction>();

        // Spawn dedicated thread with constant thread identity
        let _ = std::thread::Builder::new()
            .name("win2pod-power-arbiter".into())
            .spawn(move || {
                unsafe {
                    while let Ok(action) = action_rx.recv() {
                        let flags = match action {
                            PowerAction::SetDisplayAndSystemAwake => {
                                windows_sys::Win32::System::Power::ES_CONTINUOUS
                                    | windows_sys::Win32::System::Power::ES_SYSTEM_REQUIRED
                                    | windows_sys::Win32::System::Power::ES_DISPLAY_REQUIRED
                            }
                            PowerAction::SetSystemAwake => {
                                windows_sys::Win32::System::Power::ES_CONTINUOUS
                                    | windows_sys::Win32::System::Power::ES_SYSTEM_REQUIRED
                            }
                            PowerAction::AllowSleep => {
                                windows_sys::Win32::System::Power::ES_CONTINUOUS
                            }
                        };
                        windows_sys::Win32::System::Power::SetThreadExecutionState(flags);
                    }
                    // Reset to continuous on thread exit
                    windows_sys::Win32::System::Power::SetThreadExecutionState(
                        windows_sys::Win32::System::Power::ES_CONTINUOUS,
                    );
                }
            });

        let arbiter = Self {
            fsm,
            action_tx: Some(action_tx),
        };

        // Apply initial state to hardware
        let initial_action = PowerStateMachine::state_to_action(arbiter.fsm.current_state());
        arbiter.dispatch_action(initial_action);

        arbiter
    }

    pub fn current_state(&self) -> PowerState {
        self.fsm.current_state()
    }

    pub fn dispatch_event(&mut self, event: PowerEvent) -> PowerState {
        let (next_state, action_opt) = self.fsm.handle_event(event.clone());
        if let Some(action) = action_opt {
            info!(
                "[PowerFSM] State transition: {:?} -> {:?} (Action: {:?}, Event: {:?})",
                self.fsm.current_state, next_state, action, event
            );
            self.dispatch_action(action);
        }
        next_state
    }

    fn dispatch_action(&self, action: PowerAction) {
        if let Some(ref tx) = self.action_tx {
            let _ = tx.send(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_fsm_initial_state() {
        let fsm = PowerStateMachine::new(true);
        // Initial state is Standby until route is activated
        assert_eq!(fsm.current_state(), PowerState::Standby);
        assert!(!fsm.current_state().is_sleep_suppressed());
    }

    #[test]
    fn test_streaming_and_30s_silence_eco_transition() {
        let mut fsm = PowerStateMachine::new(true);

        // 1. Connect route without audio yet -> EcoSilent (since is_audio_active is false)
        let (state, action) = fsm.handle_event(PowerEvent::RouteActiveChanged(true));
        assert_eq!(state, PowerState::EcoSilent);
        assert_eq!(action, None); // Standby -> EcoSilent both map to AllowSleep

        // 2. Audio arrives -> AwakeStreaming
        let (state, action) = fsm.handle_event(PowerEvent::AudioActivityChanged { is_active: true, peak_level: 0.5 });
        assert_eq!(state, PowerState::AwakeStreaming);
        assert_eq!(action, Some(PowerAction::SetSystemAwake));
        assert!(state.is_sleep_suppressed());

        // 3. Silence exceeds 30 seconds -> EcoSilent
        let (state, action) = fsm.handle_event(PowerEvent::AudioActivityChanged { is_active: false, peak_level: 0.0 });
        assert_eq!(state, PowerState::EcoSilent);
        assert_eq!(action, Some(PowerAction::AllowSleep));
        assert!(!state.is_sleep_suppressed());
        assert!(state.is_eco_mode());

        // 4. Audio arrives again -> AwakeStreaming (zero-latency wake)
        let (state, action) = fsm.handle_event(PowerEvent::AudioActivityChanged { is_active: true, peak_level: 0.3 });
        assert_eq!(state, PowerState::AwakeStreaming);
        assert_eq!(action, Some(PowerAction::SetSystemAwake));

        // 5. User mutes -> immediately enters EcoSilent
        let (state, action) = fsm.handle_event(PowerEvent::MuteChanged(true));
        assert_eq!(state, PowerState::EcoSilent);
        assert_eq!(action, Some(PowerAction::AllowSleep));

        // 6. User unmutes -> returns to AwakeStreaming
        let (state, action) = fsm.handle_event(PowerEvent::MuteChanged(false));
        assert_eq!(state, PowerState::AwakeStreaming);
        assert_eq!(action, Some(PowerAction::SetSystemAwake));

        // 7. Disconnect -> Standby
        let (state, action) = fsm.handle_event(PowerEvent::RouteActiveChanged(false));
        assert_eq!(state, PowerState::Standby);
        assert_eq!(action, Some(PowerAction::AllowSleep));
    }

    #[test]
    fn test_visualizer_fullscreen_presentation_priority() {
        let mut fsm = PowerStateMachine::new(true);

        // Enter fullscreen presentation
        let (state, action) = fsm.handle_event(PowerEvent::VisualizerFullscreenChanged(true));
        assert_eq!(state, PowerState::AwakePresentation);
        assert_eq!(action, Some(PowerAction::SetDisplayAndSystemAwake));
        assert!(state.is_display_required());

        // Exit fullscreen while idle -> Standby
        let (state, action) = fsm.handle_event(PowerEvent::VisualizerFullscreenChanged(false));
        assert_eq!(state, PowerState::Standby);
        assert_eq!(action, Some(PowerAction::AllowSleep));
    }

    #[test]
    fn test_system_suspend_and_resume_lifecycle() {
        let mut fsm = PowerStateMachine::new(true);
        fsm.handle_event(PowerEvent::RouteActiveChanged(true));
        fsm.handle_event(PowerEvent::AudioActivityChanged { is_active: true, peak_level: 0.8 });
        assert_eq!(fsm.current_state(), PowerState::AwakeStreaming);

        // Suspend
        let (state, action) = fsm.handle_event(PowerEvent::SystemSuspend);
        assert_eq!(state, PowerState::Suspended);
        assert_eq!(action, Some(PowerAction::AllowSleep));

        // Resume
        let (state, action) = fsm.handle_event(PowerEvent::SystemResume);
        assert_eq!(state, PowerState::AwakeStreaming);
        assert_eq!(action, Some(PowerAction::SetSystemAwake));
    }

    #[test]
    fn test_user_disables_prevent_sleep() {
        let mut fsm = PowerStateMachine::new(true);
        fsm.handle_event(PowerEvent::RouteActiveChanged(true));
        fsm.handle_event(PowerEvent::AudioActivityChanged { is_active: true, peak_level: 0.8 });
        assert_eq!(fsm.current_state(), PowerState::AwakeStreaming);

        // User unchecks "Prevent Sleep"
        let (state, action) = fsm.handle_event(PowerEvent::UserPreventSleepChanged(false));
        assert_eq!(state, PowerState::Standby);
        assert_eq!(action, Some(PowerAction::AllowSleep));
    }
}
