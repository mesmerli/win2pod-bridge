//! Network interface enumeration, virtual/TUN adapter filtering, and subnet matching.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Network interface details with classification for physical vs virtual adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInterface {
    /// Friendly name (e.g., "乙太網路", "Wi-Fi", "ZeroTier One [xxxxxxxx]")
    pub name: String,
    /// Adapter description (e.g., "Intel(R) Ethernet Connection (16) I219-LM", "Tailscale Tunnel")
    pub description: String,
    /// System adapter identifier or GUID
    pub adapter_name: String,
    /// Bound IP address
    pub ip: IpAddr,
    /// Subnet mask (if available)
    pub netmask: Option<IpAddr>,
    /// Subnet prefix length (e.g. 24 for 255.255.255.0)
    pub prefix_len: u8,
    /// Windows interface type (e.g. 6 = Ethernet, 71 = Wi-Fi, 24 = Loopback, 131 = Tunnel)
    pub if_type: u32,
    /// Whether this interface is identified as virtual, TUN/TAP, or VPN
    pub is_virtual: bool,
    /// Whether this interface is a loopback adapter
    pub is_loopback: bool,
    /// Whether the interface is operationally up and active
    pub is_up: bool,
    /// Whether this interface is configured with a default gateway
    pub has_gateway: bool,
}

impl NetworkInterface {
    /// Whether this is a usable, active physical network interface.
    pub fn is_physical(&self) -> bool {
        !self.is_virtual && !self.is_loopback && self.is_up && !is_virtual_or_special_ip(&self.ip)
    }

    /// Check if a target IP address belongs to the same subnet as this interface.
    pub fn matches_subnet(&self, target: &IpAddr) -> bool {
        match (&self.ip, target) {
            (IpAddr::V4(local_v4), IpAddr::V4(target_v4)) => {
                is_ipv4_in_subnet(*local_v4, self.prefix_len, *target_v4)
            }
            (IpAddr::V6(local_v6), IpAddr::V6(target_v6)) => {
                is_ipv6_in_subnet(local_v6, self.prefix_len, target_v6)
            }
            _ => false,
        }
    }
}

/// Check if an IPv4 address is in the subnet defined by `local_ip` and `prefix_len`.
pub fn is_ipv4_in_subnet(local_ip: Ipv4Addr, prefix_len: u8, target_ip: Ipv4Addr) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let mask: u32 = if prefix_len >= 32 {
        !0u32
    } else {
        !0u32 << (32 - prefix_len)
    };
    (u32::from(local_ip) & mask) == (u32::from(target_ip) & mask)
}

/// Check if an IPv6 address is in the subnet defined by `local_ip` and `prefix_len`.
pub fn is_ipv6_in_subnet(local_ip: &Ipv6Addr, prefix_len: u8, target_ip: &Ipv6Addr) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let local_bytes = local_ip.octets();
    let target_bytes = target_ip.octets();
    let full_bytes = (prefix_len / 8) as usize;
    let rem_bits = prefix_len % 8;

    if local_bytes[..full_bytes] != target_bytes[..full_bytes] {
        return false;
    }
    if rem_bits > 0 && full_bytes < 16 {
        let mask = (!0u8) << (8 - rem_bits);
        if (local_bytes[full_bytes] & mask) != (target_bytes[full_bytes] & mask) {
            return false;
        }
    }
    true
}

/// Check if an interface is virtual based on its type, friendly name, and description.
pub fn is_virtual_adapter(name: &str, description: &str, if_type: u32) -> bool {
    // Standard Windows interface types:
    // IF_TYPE_SOFTWARE_LOOPBACK = 24
    // IF_TYPE_PROP_VIRTUAL = 53
    // IF_TYPE_TUNNEL = 131
    if if_type == 24 || if_type == 53 || if_type == 131 {
        return true;
    }

    let name_lower = name.to_ascii_lowercase();
    let desc_lower = description.to_ascii_lowercase();

    const VIRTUAL_KEYWORDS: &[&str] = &[
        "zerotier",
        "tailscale",
        "wireguard",
        "wsl",
        "hyper-v",
        "vethernet",
        "vmware",
        "vmnet",
        "virtualbox",
        "vbox",
        "tap",
        "tun",
        "wintun",
        "virtual",
        "vpn",
        "pseudo",
        "bluetooth",
        "bthpan",
        "nordlynx",
        "openvpn",
        "cloudflare warp",
        "surfshark",
        "proton",
        "softether",
        "radmin",
        "hamachi",
    ];

    for kw in VIRTUAL_KEYWORDS {
        if name_lower.contains(kw) || desc_lower.contains(kw) {
            return true;
        }
    }

    false
}

/// Check if an IP address is virtual, loopback, link-local, CGNAT (Tailscale), or unspecified.
pub fn is_virtual_or_special_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()                           // 127.0.0.0/8
                || v4.is_link_local()                  // 169.254.0.0/16
                || v4.is_unspecified()                 // 0.0.0.0
                || v4.is_broadcast()                   // 255.255.255.255
                // 100.64.0.0/10: RFC 6598 Shared Address Space / CGNAT used by Tailscale
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || is_link_local_v6(v6)
        }
    }
}

/// Check if an IPv6 address is link-local (fe80::/10).
pub fn is_link_local_v6(addr: &Ipv6Addr) -> bool {
    let segments = addr.segments();
    (segments[0] & 0xffc0) == 0xfe80
}

/// Retrieve all network interfaces with physical/virtual classification.
#[cfg(windows)]
pub fn get_network_interfaces() -> Vec<NetworkInterface> {
    use std::ffi::CStr;
    use std::ptr;
    use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_PREFIX, GAA_FLAG_SKIP_ANYCAST,
        GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
    };
    use windows_sys::Win32::System::Memory::{
        GetProcessHeap, HeapAlloc, HeapFree, HEAP_NONE, HEAP_ZERO_MEMORY,
    };

    let mut interfaces = Vec::new();
    let mut buffer_size: u32 = 16384;
    let mut adapter_addresses: *mut IP_ADAPTER_ADDRESSES_LH;

    unsafe {
        loop {
            adapter_addresses = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, buffer_size as usize)
                as *mut IP_ADAPTER_ADDRESSES_LH;
            if adapter_addresses.is_null() {
                return interfaces;
            }

            let flags = GAA_FLAG_SKIP_ANYCAST
                | GAA_FLAG_SKIP_MULTICAST
                | GAA_FLAG_SKIP_DNS_SERVER
                | GAA_FLAG_INCLUDE_PREFIX;

            let ret = GetAdaptersAddresses(
                AF_UNSPEC as u32,
                flags,
                ptr::null_mut(),
                adapter_addresses,
                &mut buffer_size,
            );

            if ret == ERROR_SUCCESS {
                break;
            } else if ret == ERROR_BUFFER_OVERFLOW {
                HeapFree(GetProcessHeap(), HEAP_NONE, adapter_addresses as _);
                buffer_size *= 2;
                continue;
            } else {
                HeapFree(GetProcessHeap(), HEAP_NONE, adapter_addresses as _);
                return interfaces;
            }
        }

        let mut curr = adapter_addresses;
        while !curr.is_null() {
            let adapter = &*curr;

            let friendly_name = if !adapter.FriendlyName.is_null() {
                let len = (0..).take_while(|&i| *adapter.FriendlyName.add(i) != 0).count();
                let slice = std::slice::from_raw_parts(adapter.FriendlyName, len);
                String::from_utf16_lossy(slice)
            } else {
                String::new()
            };

            let description = if !adapter.Description.is_null() {
                let len = (0..).take_while(|&i| *adapter.Description.add(i) != 0).count();
                let slice = std::slice::from_raw_parts(adapter.Description, len);
                String::from_utf16_lossy(slice)
            } else {
                String::new()
            };

            let adapter_name = if !adapter.AdapterName.is_null() {
                CStr::from_ptr(adapter.AdapterName as _).to_string_lossy().into_owned()
            } else {
                String::new()
            };

            let if_type = adapter.IfType;
            let is_up = adapter.OperStatus == 1; // IfOperStatusUp
            let has_gateway = !adapter.FirstGatewayAddress.is_null();

            let is_virt = is_virtual_adapter(&friendly_name, &description, if_type);

            let mut unicast = adapter.FirstUnicastAddress;
            while !unicast.is_null() {
                let u = &*unicast;
                let sockaddr_ptr = u.Address.lpSockaddr;
                if !sockaddr_ptr.is_null() {
                    let family = (*sockaddr_ptr).sa_family as u32;
                    let prefix_len = u.OnLinkPrefixLength;

                    let (ip_opt, netmask_opt, is_loopback) = if family == AF_INET as u32 {
                        let sin = &*(sockaddr_ptr as *const SOCKADDR_IN);
                        let ip_bytes = sin.sin_addr.S_un.S_addr.to_ne_bytes();
                        let v4 = Ipv4Addr::from(ip_bytes);
                        let mask = if prefix_len == 0 {
                            Ipv4Addr::new(0, 0, 0, 0)
                        } else if prefix_len >= 32 {
                            Ipv4Addr::new(255, 255, 255, 255)
                        } else {
                            Ipv4Addr::from(!0u32 << (32 - prefix_len))
                        };
                        (Some(IpAddr::V4(v4)), Some(IpAddr::V4(mask)), v4.is_loopback())
                    } else if family == AF_INET6 as u32 {
                        let sin6 = &*(sockaddr_ptr as *const SOCKADDR_IN6);
                        let ip_bytes = sin6.sin6_addr.u.Byte;
                        let v6 = Ipv6Addr::from(ip_bytes);
                        let is_lb = v6.is_loopback();
                        (Some(IpAddr::V6(v6)), None, is_lb)
                    } else {
                        (None, None, false)
                    };

                    if let Some(ip) = ip_opt {
                        interfaces.push(NetworkInterface {
                            name: friendly_name.clone(),
                            description: description.clone(),
                            adapter_name: adapter_name.clone(),
                            ip,
                            netmask: netmask_opt,
                            prefix_len,
                            if_type,
                            is_virtual: is_virt,
                            is_loopback,
                            is_up,
                            has_gateway,
                        });
                    }
                }
                unicast = (*unicast).Next;
            }

            curr = (*curr).Next;
        }

        HeapFree(GetProcessHeap(), HEAP_NONE, adapter_addresses as _);
    }

    interfaces
}

/// Fallback for non-Windows platforms.
#[cfg(not(windows))]
pub fn get_network_interfaces() -> Vec<NetworkInterface> {
    Vec::new()
}

/// Return all active physical network interfaces (non-virtual, non-loopback, operational).
pub fn get_physical_interfaces() -> Vec<NetworkInterface> {
    get_network_interfaces()
        .into_iter()
        .filter(|i| i.is_physical())
        .collect()
}

/// Return all detected virtual/TUN/TAP adapters.
pub fn get_virtual_interfaces() -> Vec<NetworkInterface> {
    get_network_interfaces()
        .into_iter()
        .filter(|i| i.is_virtual)
        .collect()
}

/// Find the best physical network interface matching the target receiver's subnet.
///
/// Priority:
/// 1. Direct subnet match (longest prefix match if multiple physical interfaces match)
/// 2. Active physical interface with a default gateway
/// 3. Any active physical IPv4 interface
pub fn find_best_physical_interface(target_ip: &IpAddr) -> Option<NetworkInterface> {
    let physicals = get_physical_interfaces();

    // 1. Direct subnet match
    let mut matches: Vec<NetworkInterface> = physicals
        .iter()
        .filter(|iface| iface.matches_subnet(target_ip))
        .cloned()
        .collect();

    if !matches.is_empty() {
        // Sort by prefix length descending (longest prefix / most specific subnet first)
        matches.sort_by(|a, b| b.prefix_len.cmp(&a.prefix_len));
        return Some(matches.remove(0));
    }

    // 2. Active physical interface with default gateway (preferred IPv4)
    if let Some(iface) = physicals.iter().find(|i| i.has_gateway && i.ip.is_ipv4()) {
        return Some(iface.clone());
    }

    // 3. Any active physical IPv4 interface
    if let Some(iface) = physicals.iter().find(|i| i.ip.is_ipv4()) {
        return Some(iface.clone());
    }

    // 4. Any physical interface
    physicals.into_iter().next()
}

/// Select the best receiver address from a list of candidate addresses (e.g. from mDNS).
///
/// Priority:
/// 1. An address matching a local physical interface's subnet
/// 2. Any non-virtual, non-link-local IPv4 address
/// 3. Any non-link-local IPv6 address
/// 4. Fallback to first address
pub fn select_best_receiver_address(addresses: &[IpAddr]) -> Option<IpAddr> {
    let physicals = get_physical_interfaces();

    // 1. Address that directly matches one of our physical subnets
    for addr in addresses {
        if !is_virtual_or_special_ip(addr) {
            for phys in &physicals {
                if phys.matches_subnet(addr) {
                    tracing::debug!(
                        "Selected address {} matching physical subnet {} ({})",
                        addr, phys.ip, phys.name
                    );
                    return Some(*addr);
                }
            }
        }
    }

    // 2. Any valid non-virtual, non-link-local IPv4
    for addr in addresses {
        if addr.is_ipv4() && !is_virtual_or_special_ip(addr) {
            tracing::debug!("Selected IPv4 receiver address: {}", addr);
            return Some(*addr);
        }
    }

    // 3. Global IPv6 (non-link-local)
    for addr in addresses {
        if let IpAddr::V6(v6) = addr {
            if !v6.is_loopback() && !is_link_local_v6(v6) {
                tracing::debug!("Selected global IPv6 receiver address: {}", addr);
                return Some(*addr);
            }
        }
    }

    // 4. Fallback to first address if any
    addresses.first().copied()
}

/// Convenience function: get the local physical IP address to communicate with a target IP.
pub fn select_physical_local_ip_for_target(target_ip: &IpAddr) -> Option<IpAddr> {
    find_best_physical_interface(target_ip).map(|i| i.ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_adapter_keywords() {
        assert!(is_virtual_adapter("Tailscale", "Tailscale Tunnel", 131));
        assert!(is_virtual_adapter("ZeroTier One [12345]", "ZeroTier Virtual Network Adapter", 6));
        assert!(is_virtual_adapter("WireGuard", "WireGuard Tunnel", 131));
        assert!(is_virtual_adapter("vEthernet (Default Switch)", "Hyper-V Virtual Ethernet Adapter", 6));
        assert!(is_virtual_adapter("VMware Network Adapter VMnet1", "VMware Virtual Ethernet Adapter", 6));
        assert!(is_virtual_adapter("乙太網路 2", "TAP-Windows Adapter V9", 6));
        assert!(is_virtual_adapter("OpenVPN", "Wintun Userspace Tunnel", 131));

        // Physical adapters must NOT be flagged as virtual
        assert!(!is_virtual_adapter("乙太網路", "Intel(R) Ethernet Connection (16) I219-LM", 6));
        assert!(!is_virtual_adapter("Wi-Fi", "Intel(R) Wi-Fi 6 AX201 160MHz", 71));
        assert!(!is_virtual_adapter("Ethernet", "Realtek PCIe 2.5GbE Family Controller", 6));
    }

    #[test]
    fn test_virtual_or_special_ips() {
        assert!(is_virtual_or_special_ip(&"127.0.0.1".parse().unwrap()));
        assert!(is_virtual_or_special_ip(&"169.254.10.20".parse().unwrap()));
        assert!(is_virtual_or_special_ip(&"0.0.0.0".parse().unwrap()));
        assert!(is_virtual_or_special_ip(&"100.64.0.1".parse().unwrap())); // Tailscale CGNAT
        assert!(is_virtual_or_special_ip(&"100.120.45.67".parse().unwrap())); // Tailscale CGNAT
        assert!(is_virtual_or_special_ip(&"fe80::1234".parse().unwrap()));

        assert!(!is_virtual_or_special_ip(&"192.168.1.100".parse().unwrap()));
        assert!(!is_virtual_or_special_ip(&"10.0.0.5".parse().unwrap()));
        assert!(!is_virtual_or_special_ip(&"172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn test_ipv4_subnet_match() {
        let local: Ipv4Addr = "192.168.1.50".parse().unwrap();
        let target_same: Ipv4Addr = "192.168.1.200".parse().unwrap();
        let target_diff: Ipv4Addr = "192.168.2.200".parse().unwrap();

        assert!(is_ipv4_in_subnet(local, 24, target_same));
        assert!(!is_ipv4_in_subnet(local, 24, target_diff));

        // /16 subnet
        assert!(is_ipv4_in_subnet(local, 16, target_diff));
    }

    #[test]
    #[cfg(windows)]
    fn test_enumerate_system_adapters() {
        let all = get_network_interfaces();
        println!("All interfaces ({}):", all.len());
        for iface in &all {
            println!(
                "  - [{}] {} ({}) ip={} mask={:?} prefix={} virt={} up={} gw={}",
                if iface.is_physical() { "PHYSICAL" } else { "VIRTUAL" },
                iface.name,
                iface.description,
                iface.ip,
                iface.netmask,
                iface.prefix_len,
                iface.is_virtual,
                iface.is_up,
                iface.has_gateway
            );
        }
        let physicals = get_physical_interfaces();
        println!("Physical interfaces ({}):", physicals.len());
        assert!(!physicals.is_empty(), "Should have at least one physical interface");
    }

    #[test]
    fn test_select_best_receiver_address_prioritizes_physical_subnet() {
        let candidates = vec![
            "100.85.12.34".parse::<IpAddr>().unwrap(), // Tailscale
            "192.168.168.50".parse::<IpAddr>().unwrap(), // HomePod on physical subnet
            "fe80::abcd".parse::<IpAddr>().unwrap(),    // link-local
        ];

        let best = select_best_receiver_address(&candidates);
        assert_eq!(best, Some("192.168.168.50".parse().unwrap()));
    }
}
