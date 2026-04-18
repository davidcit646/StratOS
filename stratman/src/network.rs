//! Minimalist network manager for StratOS
//! 
//! Handles Ethernet link monitoring and DHCP without external daemons.
//! WiFi is delegated to iwd (separate service).
//! 
//! Design: Run as stratman child process, restart on failure, signal state via exit codes.

use std::fs;
use std::os::unix::io::RawFd;
use std::time::{Duration, Instant};

/// Network interface state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkState {
    Up,
    Down,
    Unknown,
}

/// DHCP lease state
#[derive(Debug, Clone)]
pub struct DhcpLease {
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns: Vec<[u8; 4]>,
    pub lease_time: u32,
    pub obtained: Instant,
}

/// Network manager configuration
pub struct NetworkConfig {
    pub interface: String,
    pub use_dhcp: bool,
    pub static_ip: Option<[u8; 4]>,
    pub static_netmask: Option<[u8; 4]>,
    pub static_gateway: Option<[u8; 4]>,
    pub retry_interval: Duration,
    pub max_retries: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            interface: "eth0".to_string(),
            use_dhcp: true,
            static_ip: None,
            static_netmask: None,
            static_gateway: None,
            retry_interval: Duration::from_secs(5),
            max_retries: u32::MAX, // Infinite retries
        }
    }
}

/// Check carrier (physical link) state
pub fn read_carrier(interface: &str) -> LinkState {
    let path = format!("/sys/class/net/{}/carrier", interface);
    match fs::read_to_string(&path) {
        Ok(content) => {
            match content.trim() {
                "1" => LinkState::Up,
                "0" => LinkState::Down,
                _ => LinkState::Unknown,
            }
        }
        Err(_) => LinkState::Unknown,
    }
}

/// Check if interface exists
pub fn interface_exists(interface: &str) -> bool {
    fs::metadata(format!("/sys/class/net/{}", interface)).is_ok()
}

/// Simple DHCP client using raw sockets
/// 
/// Minimal RFC 2131 implementation:
/// 1. Send DHCPDISCOVER (broadcast)
/// 2. Wait for DHCPOFFER (unicast or broadcast)
/// 3. Send DHCPREQUEST
/// 4. Wait for DHCPACK
/// 5. Configure interface
/// 
/// Exit codes:
/// - 0: Success (should not exit normally)
/// - 100: Link down (expected, retry)
/// - 101: DHCP failed (retry with backoff)
/// - 102: Interface missing (fatal, check hardware)
pub fn run_network_manager(config: NetworkConfig) -> ! {
    let iface = &config.interface;
    
    // Verify interface exists
    if !interface_exists(iface) {
        eprintln!("strat-network: interface {} not found", iface);
        unsafe { libc::exit(102); }
    }
    
    let mut retry_count = 0;
    let mut last_state = LinkState::Unknown;
    let mut backoff = config.retry_interval;
    
    loop {
        let carrier = read_carrier(iface);
        
        // State change logging
        if carrier != last_state {
            match carrier {
                LinkState::Up => println!("strat-network: {} link up", iface),
                LinkState::Down => println!("strat-network: {} link down", iface),
                LinkState::Unknown => println!("strat-network: {} link state unknown", iface),
            }
            last_state = carrier;
        }
        
        match carrier {
            LinkState::Up => {
                // Try to get IP
                if config.use_dhcp {
                    match dhcp_request(iface) {
                        Ok(lease) => {
                            println!("strat-network: DHCP success - IP {},{},{},{}",
                                lease.ip[0], lease.ip[1], lease.ip[2], lease.ip[3]);
                            
                            // Configure interface
                            if let Err(e) = configure_interface(iface, &lease) {
                                eprintln!("strat-network: failed to configure interface: {}", e);
                                retry_count += 1;
                            } else {
                                // Success - maintain lease
                                retry_count = 0;
                                backoff = config.retry_interval;
                                
                                // Monitor lease and renew as needed
                                maintain_lease(iface, &lease, &config);
                            }
                        }
                        Err(e) => {
                            eprintln!("strat-network: DHCP failed: {}", e);
                            retry_count += 1;
                        }
                    }
                } else if let (Some(ip), Some(netmask), Some(gateway)) = 
                    (config.static_ip, config.static_netmask, config.static_gateway) {
                    // Static configuration
                    if let Err(e) = configure_static(iface, ip, netmask, gateway) {
                        eprintln!("strat-network: static config failed: {}", e);
                        retry_count += 1;
                    } else {
                        // Static config persists until link down
                        retry_count = 0;
                        wait_for_link_down(iface);
                    }
                }
            }
            LinkState::Down => {
                // Link is down - signal for retry but don't flood logs
                if retry_count == 0 {
                    println!("strat-network: waiting for link...");
                }
                retry_count += 1;
            }
            LinkState::Unknown => {
                eprintln!("strat-network: cannot read carrier state");
                retry_count += 1;
            }
        }
        
        // Check retry limit
        if config.max_retries != u32::MAX && retry_count >= config.max_retries {
            eprintln!("strat-network: max retries exceeded");
            unsafe { libc::exit(101); }
        }
        
        // Exponential backoff (capped at 60s)
        std::thread::sleep(backoff);
        backoff = std::cmp::min(backoff * 2, Duration::from_secs(60));
    }
}

/// DHCP discovery request
/// 
/// Returns lease on success, error string on failure
fn dhcp_request(_iface: &str) -> Result<DhcpLease, String> {
    // TODO: Implement raw socket DHCP
    // For now, return placeholder error to signal unimplemented
    Err("DHCP not yet implemented".to_string())
}

/// Configure interface with DHCP lease
fn configure_interface(_iface: &str, _lease: &DhcpLease) -> Result<(), String> {
    // TODO: Use netlink socket or ioctl to set IP, route, DNS
    // ip addr add {ip}/{mask} dev {iface}
    // ip route add default via {gateway}
    // Write DNS to /etc/resolv.conf
    Err("Interface configuration not yet implemented".to_string())
}

/// Configure static IP
fn configure_static(_iface: &str, _ip: [u8; 4], _netmask: [u8; 4], _gateway: [u8; 4]) -> Result<(), String> {
    // TODO: Same as configure_interface but with static params
    Err("Static configuration not yet implemented".to_string())
}

/// Maintain DHCP lease (renewal)
fn maintain_lease(_iface: &str, _lease: &DhcpLease, _config: &NetworkConfig) {
    // TODO: Monitor lease time, renew at T1 (50%) or rebind at T2 (87.5%)
    // For now, block until link goes down
    wait_for_link_down(&_iface);
}

/// Block until link goes down
fn wait_for_link_down(iface: &str) {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        if read_carrier(iface) != LinkState::Up {
            println!("strat-network: link lost, releasing IP");
            // TODO: Release DHCP lease gracefully (DHCPRELEASE)
            return;
        }
    }
}

/// Check if system is online (has routable IP)
pub fn is_online() -> bool {
    // Check if default route exists
    fs::read_to_string("/proc/net/route")
        .map(|content| content.contains("00000000")) // 0.0.0.0 destination = default route
        .unwrap_or(false)
}

/// Get online state for maintenance window decisions
pub fn network_status() -> (bool, Option<String>) {
    let online = is_online();
    let iface = "eth0"; // TODO: Detect primary interface
    let status = if online {
        Some(format!("{} online", iface))
    } else {
        Some(format!("{} offline", iface))
    };
    (online, status)
}
