//! Event-driven media playback handoff manager.
//! Replaces brittle time-delay sleeps with deterministic state convergence.
//! Monitors audio device changes, loopback readiness, and GSMTC playback status
//! to guarantee that media playback resumes reliably across audio route transitions.

use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffTarget {
    DefaultSpeakers,
    HomePod,
    AirPods,
}

#[derive(Debug)]
struct HandoffState {
    armed_at: Instant,
    deadline: Instant,
    target: HandoffTarget,
    route_ready: bool,
    default_device_confirmed: bool,
    play_sent_count: u32,
    last_play_sent: Option<Instant>,
    verified_playing: bool,
    bluetooth_disconnected_seen: bool,
}

static HANDOFF_STATE: Mutex<Option<HandoffState>> = Mutex::new(None);

/// Arms the event-driven handoff controller for a specific target endpoint.
pub fn arm(target: HandoffTarget, timeout: Duration) {
    let now = Instant::now();
    let mut guard = match HANDOFF_STATE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    tracing::info!(
        "[Event-Driven Handoff] Armed handoff for target {:?} with timeout {:?}",
        target,
        timeout
    );

    *guard = Some(HandoffState {
        armed_at: now,
        deadline: now + timeout,
        target,
        route_ready: false,
        default_device_confirmed: false,
        play_sent_count: 0,
        last_play_sent: None,
        verified_playing: false,
        bluetooth_disconnected_seen: false,
    });
}

/// Disarms any active handoff (e.g. user manually paused or handoff completed).
pub fn disarm() {
    let mut guard = match HANDOFF_STATE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if guard.is_some() {
        tracing::info!("[Event-Driven Handoff] Disarmed.");
        *guard = None;
    }
}

/// Returns true if handoff is currently armed and within deadline.
pub fn is_armed() -> bool {
    let guard = match HANDOFF_STATE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(ref st) = *guard {
        Instant::now() <= st.deadline
    } else {
        false
    }
}

/// Called when audio pipeline transition finishes (e.g. AO runner spawned or AirPlay stream running).
pub fn on_route_ready() {
    let should_trigger = {
        let mut guard = match HANDOFF_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(ref mut st) = *guard {
            if Instant::now() > st.deadline {
                *guard = None;
                return;
            }
            tracing::info!(
                "[Event-Driven Handoff] Route pipeline ready for {:?}! (elapsed: {:?})",
                st.target,
                st.armed_at.elapsed()
            );
            st.route_ready = true;
            true
        } else {
            false
        }
    };

    if should_trigger {
        evaluate_and_trigger(false);
    }
}

/// Called when Windows AudioSrv confirms the default render device via IMMNotificationClient::OnDefaultDeviceChanged.
pub fn on_default_device_confirmed() {
    let should_trigger = {
        let mut guard = match HANDOFF_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(ref mut st) = *guard {
            if Instant::now() > st.deadline {
                *guard = None;
                return;
            }
            if st.target == HandoffTarget::DefaultSpeakers {
                tracing::info!(
                    "[Event-Driven Handoff] Windows default audio endpoint confirmed by AudioSrv!"
                );
                st.default_device_confirmed = true;
                st.route_ready = true;
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    if should_trigger {
        evaluate_and_trigger(false);
    }
}

/// Called when an audio device changes state (e.g. AirPods Bluetooth endpoint unplugged/removed).
pub fn on_bluetooth_endpoint_changed(is_removed: bool) {
    if !is_removed {
        return;
    }

    let should_handle = {
        let mut guard = match HANDOFF_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(ref mut st) = *guard {
            if Instant::now() > st.deadline {
                *guard = None;
                return;
            }
            if st.route_ready {
                st.bluetooth_disconnected_seen = true;
                tracing::info!(
                    "[Event-Driven Handoff] Bluetooth endpoint removal detected while handoff is active. Scheduling recovery check."
                );
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    if should_handle {
        // Yield 200ms in background so the media player app can process the endpoint disconnection notification,
        // then re-verify / re-assert playback so it doesn't remain paused!
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(200));
            evaluate_and_trigger(true);
        });
    }
}

/// Called from media_control.rs on SMTC PlaybackInfoChanged.
pub fn on_playback_status_changed(is_playing: bool) {
    let (should_reassert, should_disarm) = {
        let mut guard = match HANDOFF_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(ref mut st) = *guard {
            if Instant::now() > st.deadline {
                *guard = None;
                return;
            }

            if is_playing {
                tracing::info!(
                    "[Event-Driven Handoff] Playback verified as PLAYING on {:?} (attempt #{})",
                    st.target,
                    st.play_sent_count
                );
                st.verified_playing = true;
                // If bluetooth endpoint already disconnected or this was default speakers,
                // we keep arm active for another 2 seconds to absorb any trailing device removal pause,
                // then disarm cleanly.
                (false, false)
            } else {
                // If it paused while route is ready and handoff is armed:
                if st.route_ready && st.play_sent_count < 5 {
                    tracing::info!(
                        "[Event-Driven Handoff] Media reported PAUSED during active handoff (attempt #{}). Enforcing Play!",
                        st.play_sent_count
                    );
                    (true, false)
                } else if st.play_sent_count >= 5 {
                    tracing::warn!(
                        "[Event-Driven Handoff] Max play enforcement attempts (5) reached, giving up."
                    );
                    (false, true)
                } else {
                    (false, false)
                }
            }
        } else {
            (false, false)
        }
    };

    if should_disarm {
        disarm();
    } else if should_reassert {
        evaluate_and_trigger(true);
    }
}

/// Evaluates state and sends Play if appropriate.
fn evaluate_and_trigger(force: bool) {
    let can_send = {
        let mut guard = match HANDOFF_STATE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(ref mut st) = *guard {
            if Instant::now() > st.deadline {
                *guard = None;
                return;
            }
            if !st.route_ready {
                return;
            }

            // Debounce: minimum 350ms between Play commands unless forced
            if !force {
                if let Some(last) = st.last_play_sent {
                    if last.elapsed() < Duration::from_millis(350) {
                        return;
                    }
                }
            }

            st.play_sent_count += 1;
            st.last_play_sent = Some(Instant::now());
            let count = st.play_sent_count;
            let target = st.target;
            tracing::info!(
                "[Event-Driven Handoff] Sending Play command to enforce handoff to {:?} (attempt #{})",
                target,
                count
            );
            true
        } else {
            false
        }
    };

    if can_send {
        crate::media_control::send_media_control(crate::media_control::MediaControlCmd::Play);
    }
}
