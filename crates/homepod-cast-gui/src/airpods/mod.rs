//! AirPods integration module for win2pod-bridge.
//!
//! Provides:
//! - Apple Continuity Protocol (AppleCP) BLE parsing (AirPods model, battery, in-ear state).
//! - WinRT BLE Advertisement Watcher for continuous AirPods presence detection.
//! - Priority audio routing and DSP enhancement management.

pub mod bt_connect;
pub mod protocol;
pub mod watcher;

pub use bt_connect::{enumerate_paired_airpods, PairedAirPodsDevice};
pub use protocol::{AirPodsAdvertisement, AirPodsDeviceState, AirPodsModel};
pub use watcher::{AirPodsEvent, AirPodsWatcher};
