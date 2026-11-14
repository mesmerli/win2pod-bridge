//! Apple Continuity Protocol (AppleCP) packet parser for AirPods and Beats.
//!
//! Re-implemented in pure Rust based on reverse-engineered Apple BLE Continuity
//! Proximity Pairing protocol (Vendor ID 0x004C, Type 0x07) and inspired by
//! SpriteOvO/AirPodsDesktop (GPL-3.0).

pub const APPLE_VENDOR_ID: u16 = 0x004C; // 76 in decimal
pub const PACKET_TYPE_PROXIMITY_PAIRING: u8 = 0x07;
pub const PROXIMITY_PAIRING_LEN: usize = 27;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirPodsModel {
    AirPods1,
    AirPods2,
    AirPods3,
    AirPods4,
    AirPods4Anc,
    AirPods5,
    AirPodsPro,
    AirPodsPro2,
    AirPodsPro2UsbC,
    AirPodsPro3,
    AirPodsMax,
    AirPodsMaxUsbC,
    BeatsFitPro,
    Powerbeats3,
    Powerbeats4,
    PowerbeatsPro,
    BeatsX,
    BeatsFlex,
    BeatsSolo3,
    BeatsSolo4,
    BeatsSoloPro,
    BeatsSoloBuds,
    BeatsStudio3,
    BeatsStudioPro,
    BeatsStudioBuds,
    BeatsStudioBudsPlus,
    Unknown(u16),
}

impl AirPodsModel {
    pub fn from_id(id: u16) -> Self {
        match id {
            0x2002 => Self::AirPods1,
            0x200F => Self::AirPods2,
            0x2013 => Self::AirPods3,
            0x2019 => Self::AirPods4,
            0x201B => Self::AirPods4Anc,
            0x2030 | 0x2036 => Self::AirPods5,
            0x200E => Self::AirPodsPro,
            0x2014 => Self::AirPodsPro2,
            0x2024 => Self::AirPodsPro2UsbC,
            0x2027 => Self::AirPodsPro3,
            0x200A => Self::AirPodsMax,
            0x201F => Self::AirPodsMaxUsbC,
            0x2012 => Self::BeatsFitPro,
            0x2003 => Self::Powerbeats3,
            0x200D => Self::Powerbeats4,
            0x200B => Self::PowerbeatsPro,
            0x2005 => Self::BeatsX,
            0x200C => Self::BeatsFlex,
            0x2006 => Self::BeatsSolo3,
            0x2025 => Self::BeatsSolo4,
            0x2009 => Self::BeatsSoloPro,
            0x2026 => Self::BeatsSoloBuds,
            0x2010 => Self::BeatsStudio3,
            0x2020 => Self::BeatsStudioPro,
            0x2011 => Self::BeatsStudioBuds,
            0x2021 => Self::BeatsStudioBudsPlus,
            other => Self::Unknown(other),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AirPods1 => "AirPods (1st gen)",
            Self::AirPods2 => "AirPods (2nd gen)",
            Self::AirPods3 => "AirPods (3rd gen)",
            Self::AirPods4 => "AirPods 4",
            Self::AirPods4Anc => "AirPods 4 (ANC)",
            Self::AirPods5 => "AirPods",
            Self::AirPodsPro => "AirPods Pro",
            Self::AirPodsPro2 => "AirPods Pro (2nd gen)",
            Self::AirPodsPro2UsbC => "AirPods Pro (2nd gen, USB-C)",
            Self::AirPodsPro3 => "AirPods Pro (3rd gen)",
            Self::AirPodsMax => "AirPods Max",
            Self::AirPodsMaxUsbC => "AirPods Max (USB-C)",
            Self::BeatsFitPro => "Beats Fit Pro",
            Self::Powerbeats3 => "Powerbeats 3",
            Self::Powerbeats4 => "Powerbeats 4",
            Self::PowerbeatsPro => "Powerbeats Pro",
            Self::BeatsX => "Beats X",
            Self::BeatsFlex => "Beats Flex",
            Self::BeatsSolo3 => "Beats Solo 3",
            Self::BeatsSolo4 => "Beats Solo 4",
            Self::BeatsSoloPro => "Beats Solo Pro",
            Self::BeatsSoloBuds => "Beats Solo Buds",
            Self::BeatsStudio3 => "Beats Studio 3",
            Self::BeatsStudioPro => "Beats Studio Pro",
            Self::BeatsStudioBuds => "Beats Studio Buds",
            Self::BeatsStudioBudsPlus => "Beats Studio Buds +",
            Self::Unknown(_) => "AirPods / Apple Audio Device",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::AirPods1 => "AirPods 1",
            Self::AirPods2 => "AirPods 2",
            Self::AirPods3 => "AirPods 3",
            Self::AirPods4 => "AirPods 4",
            Self::AirPods4Anc => "AirPods 4 ANC",
            Self::AirPods5 => "AirPods",
            Self::AirPodsPro => "AirPods Pro",
            Self::AirPodsPro2 | Self::AirPodsPro2UsbC => "AirPods Pro 2",
            Self::AirPodsPro3 => "AirPods Pro 3",
            Self::AirPodsMax | Self::AirPodsMaxUsbC => "AirPods Max",
            Self::BeatsFitPro => "Beats Fit Pro",
            Self::Powerbeats3 => "Powerbeats 3",
            Self::Powerbeats4 => "Powerbeats 4",
            Self::PowerbeatsPro => "Powerbeats Pro",
            Self::BeatsX => "Beats X",
            Self::BeatsFlex => "Beats Flex",
            Self::BeatsSolo3 => "Beats Solo 3",
            Self::BeatsSolo4 => "Beats Solo 4",
            Self::BeatsSoloPro => "Beats Solo Pro",
            Self::BeatsSoloBuds => "Beats Solo Buds",
            Self::BeatsStudio3 => "Beats Studio 3",
            Self::BeatsStudioPro => "Beats Studio Pro",
            Self::BeatsStudioBuds => "Beats Studio Buds",
            Self::BeatsStudioBudsPlus => "Beats Studio Buds+",
            Self::Unknown(_) => "AirPods",
        }
    }

    /// Checks whether this parsed AirPods model corresponds to a paired device name.
    pub fn matches_device_name(&self, dev_name: &str) -> bool {
        let lower = dev_name.to_lowercase();
        match self {
            Self::AirPodsMax | Self::AirPodsMaxUsbC => lower.contains("max"),
            Self::AirPodsPro
            | Self::AirPodsPro2
            | Self::AirPodsPro2UsbC
            | Self::AirPodsPro3 => lower.contains("pro"),
            Self::BeatsFitPro => lower.contains("fit"),
            Self::BeatsStudioBuds | Self::BeatsStudioBudsPlus => {
                lower.contains("studio") && lower.contains("buds")
            }
            Self::BeatsStudio3 | Self::BeatsStudioPro => {
                lower.contains("studio") && !lower.contains("buds")
            }
            Self::BeatsSolo3
            | Self::BeatsSolo4
            | Self::BeatsSoloPro
            | Self::BeatsSoloBuds => lower.contains("solo"),
            Self::BeatsFlex => lower.contains("flex"),
            Self::BeatsX => lower.contains("beatsx") || lower.contains("beats x"),
            Self::Powerbeats3 | Self::Powerbeats4 | Self::PowerbeatsPro => {
                lower.contains("powerbeats")
            }
            Self::AirPods1
            | Self::AirPods2
            | Self::AirPods3
            | Self::AirPods4
            | Self::AirPods4Anc
            | Self::AirPods5 => {
                !lower.contains("pro") && !lower.contains("max") && !lower.contains("beats")
            }
            Self::Unknown(_) => true,
        }
    }
}

/// Dedicated state slot for each individual paired AirPods device.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AirPodsDeviceState {
    pub model: Option<String>,
    pub in_ear: bool,
    pub left_in_ear: bool,
    pub right_in_ear: bool,
    pub left_battery: Option<u8>,
    pub right_battery: Option<u8>,
    pub case_battery: Option<u8>,
    pub left_charging: bool,
    pub right_charging: bool,
    pub case_charging: bool,
    pub is_lid_opened: Option<bool>,
    pub is_both_pods_in_case: Option<bool>,
    pub rssi: i16,
    pub is_connected: bool,
    pub is_in_use: bool,
}

impl Default for AirPodsDeviceState {
    fn default() -> Self {
        Self {
            model: None,
            in_ear: false,
            left_in_ear: false,
            right_in_ear: false,
            left_battery: None,
            right_battery: None,
            case_battery: None,
            left_charging: false,
            right_charging: false,
            case_charging: false,
            is_lid_opened: None,
            is_both_pods_in_case: None,
            rssi: -127,
            is_connected: false,
            is_in_use: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroadcastSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EarbudState {
    /// Battery level 0%..=100%, or None if unavailable/disconnected
    pub battery: Option<u8>,
    pub is_charging: bool,
    pub is_in_ear: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaseState {
    /// Battery level 0%..=100%, or None if unavailable/disconnected
    pub battery: Option<u8>,
    pub is_charging: bool,
    pub is_both_pods_in_case: bool,
    pub is_lid_opened: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirPodsAdvertisement {
    pub model: AirPodsModel,
    pub broadcast_side: BroadcastSide,
    pub left: EarbudState,
    pub right: EarbudState,
    pub case: CaseState,
    pub color_code: u8,
}

impl AirPodsAdvertisement {
    /// Attempt to parse an Apple Continuity Proximity Pairing BLE payload (at least 27 bytes)
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < PROXIMITY_PAIRING_LEN {
            return None;
        }

        let packet_type = data[0];
        let remaining_len = data[1];

        // Type 0x07 (Proximity Pairing) and remaining length must be at least 25
        if packet_type != PACKET_TYPE_PROXIMITY_PAIRING || remaining_len < 25 {
            return None;
        }

        // Model ID is stored in little-endian format at offset 3..5
        let model_id = u16::from_le_bytes([data[3], data[4]]);
        let model = AirPodsModel::from_id(model_id);

        let status_byte = data[5];
        let curr_in_ear = (status_byte & 0x02) != 0;
        let both_in_case = (status_byte & 0x04) != 0;
        let anot_in_ear = (status_byte & 0x08) != 0;
        let broadcast_is_left = (status_byte & 0x20) != 0;
        let broadcast_side = if broadcast_is_left {
            BroadcastSide::Left
        } else {
            BroadcastSide::Right
        };

        let bat_byte1 = data[6];
        let raw_curr_bat = bat_byte1 & 0x0F;
        let raw_anot_bat = (bat_byte1 >> 4) & 0x0F;

        let bat_byte2 = data[7];
        let raw_case_bat = bat_byte2 & 0x0F;
        let curr_charging = (bat_byte2 & 0x10) != 0;
        let anot_charging = (bat_byte2 & 0x20) != 0;
        let case_charging = (bat_byte2 & 0x40) != 0;

        let lid_byte = data[8];
        let lid_closed = (lid_byte & 0x08) != 0;
        let is_lid_opened = !lid_closed;

        let color_code = data[9];

        // Resolve which channel is Left vs Right based on broadcast_side
        let (raw_left_bat, raw_right_bat, left_charging, right_charging, left_in_ear_flag, right_in_ear_flag) =
            if broadcast_is_left {
                (raw_curr_bat, raw_anot_bat, curr_charging, anot_charging, curr_in_ear, anot_in_ear)
            } else {
                (raw_anot_bat, raw_curr_bat, anot_charging, curr_charging, anot_in_ear, curr_in_ear)
            };

        let left_battery = if raw_left_bat <= 10 { Some(raw_left_bat * 10) } else { None };
        let right_battery = if raw_right_bat <= 10 { Some(raw_right_bat * 10) } else { None };
        let case_battery = if raw_case_bat <= 10 { Some(raw_case_bat * 10) } else { None };

        // When earbud is charging in the case, or when stored in the case / lid closed,
        // the optical ear sensor bit can be spuriously set or cached.
        // If both pods are in case, or lid is closed, earbuds CANNOT be in ear!
        let in_case_override = both_in_case || lid_closed;
        let left_is_in_ear = !in_case_override && !left_charging && left_in_ear_flag;
        let right_is_in_ear = !in_case_override && !right_charging && right_in_ear_flag;

        Some(Self {
            model,
            broadcast_side,
            left: EarbudState {
                battery: left_battery,
                is_charging: left_charging,
                is_in_ear: left_is_in_ear,
            },
            right: EarbudState {
                battery: right_battery,
                is_charging: right_charging,
                is_in_ear: right_is_in_ear,
            },
            case: CaseState {
                battery: case_battery,
                is_charging: case_charging,
                is_both_pods_in_case: both_in_case,
                is_lid_opened,
            },
            color_code,
        })
    }

    /// Check if either earbud is currently detected as worn in ear
    pub fn is_any_in_ear(&self) -> bool {
        self.left.is_in_ear || self.right.is_in_ear
    }

    /// Check if both earbuds are detected as worn in ear
    pub fn is_both_in_ear(&self) -> bool {
        self.left.is_in_ear && self.right.is_in_ear
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_airpods_captured_left() {
        // Captured Left ear packet from AirPodsDesktop/Tests/AirPodsDomainTests.cpp
        let captured_left: [u8; 27] = [
            0x07, 0x19, 0x01, 0x30, 0x20, 0x35, 0xaa, 0xba, 0x32, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let adv = AirPodsAdvertisement::parse(&captured_left).expect("Should parse valid left packet");
        assert_eq!(adv.model, AirPodsModel::AirPods5);
        assert_eq!(adv.broadcast_side, BroadcastSide::Left);
        assert_eq!(adv.left.battery, Some(100));
        assert_eq!(adv.right.battery, Some(100));
        assert_eq!(adv.case.battery, Some(100));
        assert!(adv.left.is_charging);
        assert!(adv.right.is_charging);
        assert!(!adv.case.is_charging);
        assert!(!adv.left.is_in_ear);
        assert!(!adv.right.is_in_ear);
        assert!(adv.case.is_both_pods_in_case);
        assert!(adv.case.is_lid_opened);
        assert!(!adv.is_any_in_ear());
    }

    #[test]
    fn test_parse_airpods_captured_right() {
        // Captured Right ear packet from AirPodsDesktop/Tests/AirPodsDomainTests.cpp
        let captured_right: [u8; 27] = [
            0x07, 0x19, 0x01, 0x30, 0x20, 0x55, 0xaa, 0xba, 0x32, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let adv = AirPodsAdvertisement::parse(&captured_right).expect("Should parse valid right packet");
        assert_eq!(adv.model, AirPodsModel::AirPods5);
        assert_eq!(adv.broadcast_side, BroadcastSide::Right);
        assert_eq!(adv.left.battery, Some(100));
        assert_eq!(adv.right.battery, Some(100));
        assert_eq!(adv.case.battery, Some(100));
        assert!(adv.left.is_charging);
        assert!(adv.right.is_charging);
        assert!(!adv.case.is_charging);
        assert!(!adv.left.is_in_ear);
        assert!(!adv.right.is_in_ear);
        assert!(adv.case.is_both_pods_in_case);
        assert!(adv.case.is_lid_opened);
    }

    #[test]
    fn test_in_ear_detection() {
        // Construct a packet where Left ear is in ear (not charging, curr_in_ear=1, broadcast=Left)
        // status_byte: broadcast_is_left(0x20) | curr_in_ear(0x02) = 0x22
        // bat_byte1: left=8 (80%), right=7 (70%) => 0x78
        // bat_byte2: case=5 (50%), not charging => 0x05
        let mut packet = [0u8; 27];
        packet[0] = 0x07;
        packet[1] = 0x19;
        packet[2] = 0x01;
        packet[3] = 0x0E;
        packet[4] = 0x20; // 0x200E -> AirPods Pro
        packet[5] = 0x22; // Left broadcast, curr_in_ear=true
        packet[6] = 0x78; // left=80%, right=70%
        packet[7] = 0x05; // case=50%, no charging
        packet[8] = 0x00; // lid_closed=false (lid opened)

        let adv = AirPodsAdvertisement::parse(&packet).expect("Should parse");
        assert_eq!(adv.model, AirPodsModel::AirPodsPro);
        assert_eq!(adv.broadcast_side, BroadcastSide::Left);
        assert_eq!(adv.left.battery, Some(80));
        assert_eq!(adv.right.battery, Some(70));
        assert_eq!(adv.case.battery, Some(50));
        assert!(adv.left.is_in_ear);
        assert!(!adv.right.is_in_ear);
        assert!(adv.case.is_lid_opened);
        assert!(adv.is_any_in_ear());
        assert!(!adv.is_both_in_ear());
    }

    #[test]
    fn test_invalid_length_or_type() {
        let mut short_packet = [0u8; 20];
        short_packet[0] = 0x07;
        assert!(AirPodsAdvertisement::parse(&short_packet).is_none());

        let mut wrong_type = [0u8; 27];
        wrong_type[0] = 0x05;
        wrong_type[1] = 0x19;
        assert!(AirPodsAdvertisement::parse(&wrong_type).is_none());
    }
}
