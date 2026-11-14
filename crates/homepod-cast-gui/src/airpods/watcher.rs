//! WinRT BLE Advertisement Watcher for continuous AirPods presence & in-ear detection.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::Result as WinResult;
use windows::Devices::Bluetooth::Advertisement::*;
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::DataReader;

use super::protocol::{AirPodsAdvertisement, AirPodsModel, APPLE_VENDOR_ID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AirPodsEvent {
    StateChanged {
        model: AirPodsModel,
        rssi: i16,
        is_any_in_ear: bool,
        is_both_in_ear: bool,
        left_in_ear: bool,
        right_in_ear: bool,
        left_battery: Option<u8>,
        right_battery: Option<u8>,
        case_battery: Option<u8>,
        left_charging: bool,
        right_charging: bool,
        case_charging: bool,
        is_lid_opened: bool,
        is_both_pods_in_case: bool,
    },
}

pub struct AirPodsWatcher {
    watcher: BluetoothLEAdvertisementWatcher,
    is_running: Arc<AtomicBool>,
}

impl AirPodsWatcher {
    /// Create and start a new background AirPods BLE watcher.
    /// Whenever an AirPods advertisement is received with state updates, `on_event` is called.
    pub fn start<F>(on_event: F) -> WinResult<Self>
    where
        F: Fn(AirPodsEvent) + Send + Sync + 'static,
    {
        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        let _ = watcher.SetScanningMode(BluetoothLEScanningMode::Active);

        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = is_running.clone();
        let callback = Arc::new(on_event);

        watcher.Received(&TypedEventHandler::new(
            move |_sender, args: windows_core::Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
                if !running_clone.load(Ordering::Relaxed) {
                    return Ok(());
                }

                if let Some(event_args) = args.as_ref() {
                    let rssi = event_args.RawSignalStrengthInDBm().unwrap_or(-127);
                    if let Ok(adv) = event_args.Advertisement() {
                        if let Ok(mfr_data_list) = adv.ManufacturerData() {
                            for mfr_data in mfr_data_list {
                                if let Ok(company_id) = mfr_data.CompanyId() {
                                    if company_id == APPLE_VENDOR_ID {
                                        if let Ok(buffer) = mfr_data.Data() {
                                            if let Ok(reader) = DataReader::FromBuffer(&buffer) {
                                                if let Ok(len) = reader.UnconsumedBufferLength() {
                                                    let mut bytes = vec![0u8; len as usize];
                                                    if reader.ReadBytes(&mut bytes).is_ok() {
                                                        if let Some(parsed) = AirPodsAdvertisement::parse(&bytes) {
                                                            callback(AirPodsEvent::StateChanged {
                                                                model: parsed.model,
                                                                rssi,
                                                                is_any_in_ear: parsed.is_any_in_ear(),
                                                                is_both_in_ear: parsed.is_both_in_ear(),
                                                                left_in_ear: parsed.left.is_in_ear,
                                                                right_in_ear: parsed.right.is_in_ear,
                                                                left_battery: parsed.left.battery,
                                                                right_battery: parsed.right.battery,
                                                                case_battery: parsed.case.battery,
                                                                left_charging: parsed.left.is_charging,
                                                                right_charging: parsed.right.is_charging,
                                                                case_charging: parsed.case.is_charging,
                                                                is_lid_opened: parsed.case.is_lid_opened,
                                                                is_both_pods_in_case: parsed.case.is_both_pods_in_case,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            },
        ))?;

        watcher.Start()?;
        tracing::info!("[AirPods] WinRT BLE Advertisement Watcher successfully started");

        Ok(Self {
            watcher,
            is_running,
        })
    }

    /// Stop the BLE advertisement watcher.
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
        let _ = self.watcher.Stop();
        tracing::info!("[AirPods] WinRT BLE Advertisement Watcher stopped");
    }
}

impl Drop for AirPodsWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
