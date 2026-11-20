//! mDNS Service Advertisement for AirPlay 1 RAOP.

use std::collections::HashMap;
use std::net::IpAddr;
use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::config::AirPlayReceiverConfig;

pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl MdnsAdvertiser {
    pub fn start(config: &AirPlayReceiverConfig, local_ip: IpAddr) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let daemon = ServiceDaemon::new()?;
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "win2pod".to_string());

        let mut properties = HashMap::new();
        properties.insert("txtvers".to_string(), "1".to_string());
        properties.insert("ch".to_string(), "2".to_string());
        properties.insert("cn".to_string(), "0,1".to_string());
        properties.insert("et".to_string(), "0,1".to_string());
        properties.insert("md".to_string(), "0,1,2".to_string());
        properties.insert("pw".to_string(), "false".to_string());
        properties.insert("sr".to_string(), "44100".to_string());
        properties.insert("ss".to_string(), "16".to_string());
        properties.insert("tp".to_string(), "UDP".to_string());
        properties.insert("vn".to_string(), "3".to_string());
        properties.insert("vs".to_string(), "220.68".to_string());
        properties.insert("am".to_string(), "AirPort4,107".to_string());
        properties.insert("sf".to_string(), "0x4".to_string());

        let mac_prefix = format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            config.mac_address[0], config.mac_address[1], config.mac_address[2],
            config.mac_address[3], config.mac_address[4], config.mac_address[5]
        );
        let service_instance = format!("{}@{}", mac_prefix, config.name);
        let service_type = "_raop._tcp.local.";
        let host_domain = format!("{}.local.", hostname);

        let service_info = ServiceInfo::new(
            service_type,
            &service_instance,
            &host_domain,
            local_ip.to_string(),
            config.rtsp_port,
            properties,
        )?;

        let fullname = service_info.get_fullname().to_string();
        daemon.register(service_info)?;
        tracing::info!("mDNS advertisement started: {}", fullname);

        Ok(Self { daemon, fullname })
    }

    /// Explicitly unregisters mDNS announcement, broadcasting Goodbye packets (TTL=0)
    /// to notify all network devices that this receiver has gone offline.
    pub fn stop(&mut self) {
        if let Ok(rx) = self.daemon.unregister(&self.fullname) {
            let _ = rx.recv_timeout(std::time::Duration::from_millis(300));
        }
        if let Ok(rx) = self.daemon.shutdown() {
            let _ = rx.recv_timeout(std::time::Duration::from_millis(300));
        }
        tracing::info!("mDNS advertisement stopped and Goodbye broadcast sent: {}", self.fullname);
    }
}

impl Drop for MdnsAdvertiser {
    fn drop(&mut self) {
        self.stop();
    }
}
