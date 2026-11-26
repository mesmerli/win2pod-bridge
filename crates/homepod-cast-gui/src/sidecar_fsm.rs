//! Sidecar Lifecycle Finite State Machine (FSM).
//!
//! Provides a formal state machine for managing the lifecycle of the
//! background `win2pod-engine.exe` sidecar daemon, eliminating invalid state
//! combinations, zombie resurrects, and race conditions.

use std::time::Instant;

/// Lifecycle states of the sidecar engine process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarLifecycleState {
    /// Engine has not been started yet.
    Unstarted,
    /// Spawning process and establishing IPC named pipe connection.
    Spawning {
        start_time: Instant,
    },
    /// IPC pipe connected, handshake verified, standing by for commands.
    Ready,
    /// Active audio streaming or routing session in progress.
    Streaming {
        target: String,
    },
    /// Unexpected disconnect or crash, undergoing automatic self-healing.
    Recovering {
        attempt: u32,
        next_retry: Instant,
        last_error: String,
    },
    /// Graceful shutdown in progress. Reject all new commands or spawn attempts!
    ShuttingDown,
    /// Engine process and IPC resources have been fully terminated and cleaned up.
    Terminated,
    /// Permanent failure after max retry attempts exhausted.
    Dead {
        reason: String,
    },
}

impl Default for SidecarLifecycleState {
    fn default() -> Self {
        Self::Unstarted
    }
}

/// Events driving state transitions in the sidecar lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarLifecycleEvent {
    /// Request to ensure engine is running.
    EnsureStarted,
    /// IPC connection established and verified.
    Connected,
    /// Audio streaming session started.
    StreamStarted {
        target: String,
    },
    /// Audio streaming session ended.
    StreamStopped,
    /// Unexpected pipe disconnect or process termination.
    PipeBroken {
        error: String,
    },
    /// System or user requested application quit/shutdown.
    ShutdownRequested,
    /// Process and resources successfully cleaned up.
    CleanupComplete,
    /// Retry attempt initiated.
    RetrySpawn,
    /// Permanent fatal error encountered.
    FatalError {
        reason: String,
    },
}

/// Actions produced by state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarAction {
    SpawnOrConnect,
    KillProcess,
    ScheduleRetry {
        attempt: u32,
    },
    NotifyError {
        reason: String,
    },
    NoAction,
}

/// The deterministic Finite State Machine for Sidecar Lifecycle.
#[derive(Debug, Clone)]
pub struct SidecarLifecycleFsm {
    state: SidecarLifecycleState,
    max_recovery_retries: u32,
}

impl SidecarLifecycleFsm {
    pub fn new() -> Self {
        Self {
            state: SidecarLifecycleState::Unstarted,
            max_recovery_retries: 3,
        }
    }

    pub fn state(&self) -> &SidecarLifecycleState {
        &self.state
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state, SidecarLifecycleState::Ready | SidecarLifecycleState::Streaming { .. })
    }

    pub fn is_shutting_down(&self) -> bool {
        matches!(self.state, SidecarLifecycleState::ShuttingDown | SidecarLifecycleState::Terminated)
    }

    /// Pure transition function: (CurrentState, Event) -> (NextState, Action)
    pub fn transition(&mut self, event: SidecarLifecycleEvent) -> Result<SidecarAction, String> {
        let (next_state, action) = match (&self.state, event) {
            // Guard: in ShuttingDown or Terminated state, strictly reject spawn attempts!
            (SidecarLifecycleState::ShuttingDown | SidecarLifecycleState::Terminated, SidecarLifecycleEvent::EnsureStarted) => {
                return Err("Cannot start sidecar: shutdown is already in progress or completed".into());
            }

            // In Ready or Streaming, EnsureStarted is a no-op
            (SidecarLifecycleState::Ready | SidecarLifecycleState::Streaming { .. }, SidecarLifecycleEvent::EnsureStarted) => {
                return Ok(SidecarAction::NoAction);
            }

            // Normal spawn triggers
            (SidecarLifecycleState::Unstarted | SidecarLifecycleState::Dead { .. }, SidecarLifecycleEvent::EnsureStarted) => {
                (SidecarLifecycleState::Spawning { start_time: Instant::now() }, SidecarAction::SpawnOrConnect)
            }

            (SidecarLifecycleState::Recovering { .. }, SidecarLifecycleEvent::RetrySpawn) => {
                (SidecarLifecycleState::Spawning { start_time: Instant::now() }, SidecarAction::SpawnOrConnect)
            }

            // Connection established
            (SidecarLifecycleState::Spawning { .. }, SidecarLifecycleEvent::Connected) => {
                (SidecarLifecycleState::Ready, SidecarAction::NoAction)
            }

            // Stream started
            (SidecarLifecycleState::Ready, SidecarLifecycleEvent::StreamStarted { target }) => {
                (SidecarLifecycleState::Streaming { target }, SidecarAction::NoAction)
            }

            // Stream stopped
            (SidecarLifecycleState::Streaming { .. }, SidecarLifecycleEvent::StreamStopped) => {
                (SidecarLifecycleState::Ready, SidecarAction::NoAction)
            }

            // Unexpected pipe broken (NOT during shutdown!)
            (SidecarLifecycleState::Ready | SidecarLifecycleState::Streaming { .. }, SidecarLifecycleEvent::PipeBroken { error }) => {
                let attempt = 1;
                (
                    SidecarLifecycleState::Recovering {
                        attempt,
                        next_retry: Instant::now(),
                        last_error: error,
                    },
                    SidecarAction::ScheduleRetry { attempt },
                )
            }

            (SidecarLifecycleState::Recovering { attempt, .. }, SidecarLifecycleEvent::PipeBroken { error }) => {
                let next_attempt = attempt + 1;
                if next_attempt > self.max_recovery_retries {
                    (
                        SidecarLifecycleState::Dead {
                            reason: format!("Max retries ({}) exceeded. Last error: {}", self.max_recovery_retries, error),
                        },
                        SidecarAction::NotifyError {
                            reason: format!("Sidecar engine crashed and failed to recover after {} attempts", self.max_recovery_retries),
                        },
                    )
                } else {
                    (
                        SidecarLifecycleState::Recovering {
                            attempt: next_attempt,
                            next_retry: Instant::now(),
                            last_error: error,
                        },
                        SidecarAction::ScheduleRetry { attempt: next_attempt },
                    )
                }
            }

            // Pipe broken during active shutdown is expected and ignored
            (SidecarLifecycleState::ShuttingDown, SidecarLifecycleEvent::PipeBroken { .. }) => {
                (SidecarLifecycleState::ShuttingDown, SidecarAction::NoAction)
            }

            // Shutdown request from ANY state
            (state, SidecarLifecycleEvent::ShutdownRequested) => {
                if *state == SidecarLifecycleState::Terminated {
                    return Ok(SidecarAction::NoAction);
                }
                (SidecarLifecycleState::ShuttingDown, SidecarAction::KillProcess)
            }

            // Cleanup complete after shutdown
            (SidecarLifecycleState::ShuttingDown, SidecarLifecycleEvent::CleanupComplete) => {
                (SidecarLifecycleState::Terminated, SidecarAction::NoAction)
            }

            // Fatal error
            (_, SidecarLifecycleEvent::FatalError { reason }) => {
                (SidecarLifecycleState::Dead { reason: reason.clone() }, SidecarAction::NotifyError { reason })
            }

            // Catch-all invalid transition
            (s, e) => {
                return Err(format!("Invalid state transition from {:?} with event {:?}", s, e));
            }
        };

        self.state = next_state;
        Ok(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsm_normal_lifecycle() {
        let mut fsm = SidecarLifecycleFsm::new();
        assert_eq!(*fsm.state(), SidecarLifecycleState::Unstarted);
        assert!(!fsm.is_connected());

        // 1. EnsureStarted -> Spawning
        let act = fsm.transition(SidecarLifecycleEvent::EnsureStarted).unwrap();
        assert_eq!(act, SidecarAction::SpawnOrConnect);
        assert!(matches!(fsm.state(), SidecarLifecycleState::Spawning { .. }));

        // 2. Connected -> Ready
        let act = fsm.transition(SidecarLifecycleEvent::Connected).unwrap();
        assert_eq!(act, SidecarAction::NoAction);
        assert_eq!(*fsm.state(), SidecarLifecycleState::Ready);
        assert!(fsm.is_connected());

        // 3. StreamStarted -> Streaming
        let act = fsm.transition(SidecarLifecycleEvent::StreamStarted { target: "Living Room".into() }).unwrap();
        assert_eq!(act, SidecarAction::NoAction);
        assert!(matches!(fsm.state(), SidecarLifecycleState::Streaming { target } if target == "Living Room"));
        assert!(fsm.is_connected());

        // 4. StreamStopped -> Ready
        let act = fsm.transition(SidecarLifecycleEvent::StreamStopped).unwrap();
        assert_eq!(act, SidecarAction::NoAction);
        assert_eq!(*fsm.state(), SidecarLifecycleState::Ready);

        // 5. ShutdownRequested -> ShuttingDown
        let act = fsm.transition(SidecarLifecycleEvent::ShutdownRequested).unwrap();
        assert_eq!(act, SidecarAction::KillProcess);
        assert_eq!(*fsm.state(), SidecarLifecycleState::ShuttingDown);
        assert!(fsm.is_shutting_down());

        // 6. CleanupComplete -> Terminated
        let act = fsm.transition(SidecarLifecycleEvent::CleanupComplete).unwrap();
        assert_eq!(act, SidecarAction::NoAction);
        assert_eq!(*fsm.state(), SidecarLifecycleState::Terminated);
    }

    #[test]
    fn test_fsm_prevents_zombie_resurrect_during_shutdown() {
        let mut fsm = SidecarLifecycleFsm::new();
        fsm.transition(SidecarLifecycleEvent::EnsureStarted).unwrap();
        fsm.transition(SidecarLifecycleEvent::Connected).unwrap();

        // Initiate shutdown
        fsm.transition(SidecarLifecycleEvent::ShutdownRequested).unwrap();
        assert_eq!(*fsm.state(), SidecarLifecycleState::ShuttingDown);

        // A background poll or async task tries to EnsureStarted during shutdown
        let err = fsm.transition(SidecarLifecycleEvent::EnsureStarted).unwrap_err();
        assert!(err.contains("shutdown is already in progress"));
        assert_eq!(*fsm.state(), SidecarLifecycleState::ShuttingDown);

        // PipeBroken during shutdown is completely benign and does NOT enter Recovering
        let act = fsm.transition(SidecarLifecycleEvent::PipeBroken { error: "EOF".into() }).unwrap();
        assert_eq!(act, SidecarAction::NoAction);
        assert_eq!(*fsm.state(), SidecarLifecycleState::ShuttingDown);
    }

    #[test]
    fn test_fsm_recovery_and_exhaustion() {
        let mut fsm = SidecarLifecycleFsm::new();
        fsm.transition(SidecarLifecycleEvent::EnsureStarted).unwrap();
        fsm.transition(SidecarLifecycleEvent::Connected).unwrap();

        // Unexpected crash (PipeBroken)
        let act = fsm.transition(SidecarLifecycleEvent::PipeBroken { error: "Crash 1".into() }).unwrap();
        assert_eq!(act, SidecarAction::ScheduleRetry { attempt: 1 });
        assert!(matches!(fsm.state(), SidecarLifecycleState::Recovering { attempt: 1, .. }));

        // Crash 2
        let act = fsm.transition(SidecarLifecycleEvent::PipeBroken { error: "Crash 2".into() }).unwrap();
        assert_eq!(act, SidecarAction::ScheduleRetry { attempt: 2 });

        // Crash 3
        let act = fsm.transition(SidecarLifecycleEvent::PipeBroken { error: "Crash 3".into() }).unwrap();
        assert_eq!(act, SidecarAction::ScheduleRetry { attempt: 3 });

        // Crash 4 -> Max retries exceeded -> Dead
        let act = fsm.transition(SidecarLifecycleEvent::PipeBroken { error: "Crash 4".into() }).unwrap();
        assert!(matches!(act, SidecarAction::NotifyError { .. }));
        assert!(matches!(fsm.state(), SidecarLifecycleState::Dead { .. }));
    }
}
