//! Minimalist network manager for StratOS
//!
//! Ethernet and Wi-Fi (after association): link monitoring and DHCP without NetworkManager.
//! Wi-Fi association is handled by **`strat-wpa`** (`wpa_supplicant`); this process runs DHCP.
//!
//! Design: Run as stratman child process, restart on failure, signal state via exit codes.

use std::fs;
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
    pub server_ip: [u8; 4],  // DHCP server for renewals
    pub mac: [u8; 6],         // Client MAC for renewals
}

impl DhcpLease {
    /// Calculate T1 (renewal at 50% of lease)
    fn t1(&self) -> Duration {
        Duration::from_secs(self.lease_time as u64 / 2)
    }
    
    /// Calculate T2 (rebinding at 87.5% of lease)
    fn t2(&self) -> Duration {
        Duration::from_secs((self.lease_time as u64 * 7) / 8)
    }
    
    /// Time elapsed since lease obtained
    fn elapsed(&self) -> Duration {
        self.obtained.elapsed()
    }
    
    /// Check if lease has expired
    fn is_expired(&self) -> bool {
        self.elapsed() >= Duration::from_secs(self.lease_time as u64)
    }
    
    /// Check if we should renew (at T1)
    fn should_renew(&self) -> bool {
        let elapsed = self.elapsed();
        elapsed >= self.t1() && elapsed < self.t2()
    }
    
    /// Check if we should rebind (at T2)
    fn should_rebind(&self) -> bool {
        let elapsed = self.elapsed();
        elapsed >= self.t2() && !self.is_expired()
    }
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
            interface: "auto".to_string(),
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

fn skip_iface(name: &str) -> bool {
    matches!(name, "lo" | "lo0")
        || name.starts_with("docker")
        || name.starts_with("br-")
        || name.starts_with("virbr")
        || name.starts_with("veth")
        || name.starts_with("tun")
        || name.starts_with("tap")
}

fn is_wireless(interface: &str) -> bool {
    interface.starts_with("wl")
        || fs::metadata(format!("/sys/class/net/{}/phy80211", interface)).is_ok()
}

fn read_operstate(interface: &str) -> Option<String> {
    fs::read_to_string(format!("/sys/class/net/{}/operstate", interface))
        .ok()
        .map(|s| s.trim().to_string())
}

/// L2 ready for DHCP: Ethernet/USB NIC carrier; Wi-Fi `operstate` up when associated.
pub fn read_link_ready(interface: &str) -> LinkState {
    if is_wireless(interface) {
        match read_operstate(interface).as_deref() {
            Some("up") | Some("unknown") => return LinkState::Up,
            Some("down") | Some("dormant") => return LinkState::Down,
            _ => {}
        }
    }
    read_carrier(interface)
}

fn list_physical_interfaces() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(dir) = fs::read_dir("/sys/class/net") else {
        return out;
    };
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if skip_iface(&name) {
            continue;
        }
        out.push(name);
    }
    out.sort();
    out
}

fn pick_auto_interface() -> Option<String> {
    let ifs = list_physical_interfaces();
    if ifs.is_empty() {
        return None;
    }
    let wired: Vec<String> = ifs.iter().filter(|n| !is_wireless(n)).cloned().collect();
    let wifi: Vec<String> = ifs.iter().filter(|n| is_wireless(n)).cloned().collect();

    for n in &wired {
        if read_carrier(n) == LinkState::Up {
            return Some(n.clone());
        }
    }
    for n in &wifi {
        if read_operstate(n).as_deref() == Some("up") {
            return Some(n.clone());
        }
    }
    wired.into_iter().next().or_else(|| wifi.into_iter().next())
}

fn resolve_auto_interface(max_wait: Duration) -> String {
    let steps = max_wait.as_secs().max(1);
    for _ in 0..steps {
        if let Some(i) = pick_auto_interface() {
            return i;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    eprintln!(
        "strat-network: auto: no usable interface after {}s, falling back to eth0",
        steps
    );
    "eth0".to_string()
}

fn wait_for_interface(name: &str, max_wait: Duration) {
    let steps = max_wait.as_secs().max(1);
    for _ in 0..steps {
        if interface_exists(name) {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn parse_ipv4_dotted(s: &str) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    for (i, p) in parts.iter().enumerate() {
        let n: u32 = p.parse().ok()?;
        if n > 255 {
            return None;
        }
        out[i] = n as u8;
    }
    Some(out)
}

fn opt_ipv4_string(field: &Option<String>) -> Option<[u8; 4]> {
    field
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .and_then(parse_ipv4_dotted)
}

fn apply_modular_network(cfg: &mut NetworkConfig, n: &stratsettings::NetworkSettings) {
    cfg.interface = n.interface.clone();
    cfg.use_dhcp = n.use_dhcp;
    cfg.static_ip = opt_ipv4_string(&n.static_ip);
    cfg.static_netmask = opt_ipv4_string(&n.static_netmask);
    cfg.static_gateway = opt_ipv4_string(&n.static_gateway);
    cfg.retry_interval = Duration::from_secs(n.retry_interval_secs.max(1));
    cfg.max_retries = n.max_retries.unwrap_or(u32::MAX);
}

fn apply_legacy_network_toml_shim(cfg: &mut NetworkConfig) {
    if let Ok(s) = fs::read_to_string("/config/strat/network.toml") {
        if let Ok(v) = toml::from_str::<toml::Value>(&s) {
            if let Some(t) = v.as_table() {
                if let Some(i) = t.get("interface").and_then(|x| x.as_str()) {
                    cfg.interface = i.to_string();
                }
                if let Some(d) = t.get("use_dhcp").and_then(|x| x.as_bool()) {
                    cfg.use_dhcp = d;
                }
            }
        }
    }
}

/// Load network options: built-in defaults, optional legacy `/config/strat/network.toml` (narrow shim),
/// merged modular [`stratsettings::StratSettings`] table `[network]` (overrides shim when load succeeds),
/// then `STRAT_NETWORK_INTERFACE` (interface name only).
pub fn load_network_config() -> NetworkConfig {
    let mut cfg = NetworkConfig::default();
    apply_legacy_network_toml_shim(&mut cfg);

    match stratsettings::StratSettings::load_from(std::path::Path::new(stratsettings::CONFIG_DIR)) {
        Ok(s) => apply_modular_network(&mut cfg, &s.network),
        Err(e) => eprintln!(
            "strat-network: modular settings unavailable ({}), using defaults/shim only",
            e
        ),
    }

    if let Ok(s) = std::env::var("STRAT_NETWORK_INTERFACE") {
        if !s.is_empty() {
            cfg.interface = s;
        }
    }
    cfg
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
    let mut cfg = config;
    if cfg.interface == "auto" {
        println!("strat-network: resolving interface (auto)…");
        cfg.interface = resolve_auto_interface(Duration::from_secs(120));
        println!("strat-network: selected {}", cfg.interface);
    }
    wait_for_interface(&cfg.interface, Duration::from_secs(120));
    if !interface_exists(&cfg.interface) {
        eprintln!("strat-network: interface {} not found", cfg.interface);
        unsafe { libc::exit(102); }
    }

    let iface = cfg.interface.clone();
    let iface = iface.as_str();

    let mut retry_count = 0;
    let mut last_state = LinkState::Unknown;
    let mut backoff = cfg.retry_interval;

    loop {
        let link = read_link_ready(iface);

        // State change logging
        if link != last_state {
            match link {
                LinkState::Up => println!("strat-network: {} link up", iface),
                LinkState::Down => println!("strat-network: {} link down", iface),
                LinkState::Unknown => println!("strat-network: {} link state unknown", iface),
            }
            last_state = link;
        }

        match link {
            LinkState::Up => {
                // Try to get IP
                if cfg.use_dhcp {
                    match dhcp_request(iface) {
                        Ok(mut lease) => {
                            println!("strat-network: DHCP success - IP {},{},{},{}",
                                lease.ip[0], lease.ip[1], lease.ip[2], lease.ip[3]);
                            
                            // Configure interface
                            if let Err(e) = configure_interface(iface, &lease) {
                                eprintln!("strat-network: failed to configure interface: {}", e);
                                retry_count += 1;
                            } else {
                                // Success - maintain lease (renewal at T1/T2)
                                retry_count = 0;
                                backoff = cfg.retry_interval;
                                
                                // Monitor lease and renew as needed
                                maintain_lease(iface, &mut lease, &cfg);
                            }
                        }
                        Err(e) => {
                            eprintln!("strat-network: DHCP failed: {}", e);
                            retry_count += 1;
                        }
                    }
                } else if let (Some(ip), Some(netmask), Some(gateway)) = 
                    (cfg.static_ip, cfg.static_netmask, cfg.static_gateway) {
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
        if cfg.max_retries != u32::MAX && retry_count >= cfg.max_retries {
            eprintln!("strat-network: max retries exceeded");
            unsafe { libc::exit(101); }
        }
        
        // Exponential backoff (capped at 60s)
        std::thread::sleep(backoff);
        backoff = std::cmp::min(backoff * 2, Duration::from_secs(60));
    }
}

/// DHCP packet structure (RFC 2131)
#[repr(C)]
struct DhcpPacket {
    op: u8,           // 1=request, 2=reply
    htype: u8,        // HW type (1=Ethernet)
    hlen: u8,         // HW address length (6 for MAC)
    hops: u8,         // 0 for client
    xid: u32,         // Transaction ID
    secs: u16,        // Seconds since start
    flags: u16,       // 0x8000 for broadcast
    ciaddr: [u8; 4],  // Client IP (0 for discover)
    yiaddr: [u8; 4],  // Your IP (server assigned)
    siaddr: [u8; 4],  // Server IP
    giaddr: [u8; 4],  // Gateway IP
    chaddr: [u8; 16], // Client HW addr (MAC)
    sname: [u8; 64],  // Server name (null)
    file: [u8; 128],  // Boot file (null)
    options: [u8; 312], // Options (magic cookie + options)
}

impl DhcpPacket {
    fn new_discover(xid: u32, mac: &[u8; 6]) -> Self {
        let mut pkt = Self {
            op: 1, htype: 1, hlen: 6, hops: 0,
            xid, secs: 0, flags: 0x8000, // Broadcast
            ciaddr: [0; 4], yiaddr: [0; 4], siaddr: [0; 4], giaddr: [0; 4],
            chaddr: [0; 16], sname: [0; 64], file: [0; 128], options: [0; 312],
        };
        pkt.chaddr[0..6].copy_from_slice(mac);
        // Magic cookie: 0x63825363
        pkt.options[0] = 99;
        pkt.options[1] = 130;
        pkt.options[2] = 83;
        pkt.options[3] = 99;
        // Option 53: DHCP Message Type = 1 (DISCOVER)
        pkt.options[4] = 53;
        pkt.options[5] = 1;
        pkt.options[6] = 1;
        // Option 55: Parameter Request List (subnet mask, router, DNS, domain)
        pkt.options[7] = 55;
        pkt.options[8] = 4;
        pkt.options[9] = 1;   // Subnet mask
        pkt.options[10] = 3;  // Router
        pkt.options[11] = 6;  // DNS
        pkt.options[12] = 15; // Domain name
        // End option
        pkt.options[13] = 255;
        pkt
    }

    fn new_request(xid: u32, mac: &[u8; 6], offered_ip: [u8; 4], server_ip: [u8; 4]) -> Self {
        let mut pkt = Self::new_discover(xid, mac);
        pkt.ciaddr = offered_ip;
        // Update message type to REQUEST (3)
        pkt.options[6] = 3;
        // Option 50: Requested IP
        pkt.options[13] = 50;
        pkt.options[14] = 4;
        pkt.options[15..19].copy_from_slice(&offered_ip);
        // Option 54: Server Identifier
        pkt.options[19] = 54;
        pkt.options[20] = 4;
        pkt.options[21..25].copy_from_slice(&server_ip);
        pkt.options[25] = 255;
        pkt
    }

    fn new_renew(xid: u32, mac: &[u8; 6], current_ip: [u8; 4], server_ip: [u8; 4]) -> Self {
        let mut pkt = Self::new_discover(xid, mac);
        pkt.ciaddr = current_ip; // Already have IP, unicast to server
        pkt.options[6] = 3; // REQUEST
        // Option 54: Server Identifier
        pkt.options[13] = 54;
        pkt.options[14] = 4;
        pkt.options[15..19].copy_from_slice(&server_ip);
        pkt.options[19] = 255;
        pkt
    }

    fn new_release(mac: &[u8; 6], current_ip: [u8; 4], server_ip: [u8; 4]) -> Self {
        let mut pkt = Self::new_discover(0, mac); // xid not important for release
        pkt.ciaddr = current_ip;
        pkt.options[6] = 7; // RELEASE (RFC 2131)
        // Option 54: Server Identifier
        pkt.options[7] = 54;
        pkt.options[8] = 4;
        pkt.options[9..13].copy_from_slice(&server_ip);
        pkt.options[13] = 255;
        pkt
    }
}

/// Get MAC address from interface
fn get_mac_address(iface: &str) -> Result<[u8; 6], String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err("Failed to create socket".to_string());
    }
    
    let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
    let name_bytes = iface.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(
            name_bytes.as_ptr() as *const i8,
            ifr.ifr_name.as_mut_ptr(),
            name_bytes.len().min(15)
        );
    }
    
    let ret = unsafe { libc::ioctl(sock, libc::SIOCGIFHWADDR, &mut ifr) };
    unsafe { libc::close(sock); }
    
    if ret < 0 {
        return Err(format!("Failed to get MAC address for {}", iface));
    }
    
    let mut mac = [0u8; 6];
    unsafe {
        let sa_data = &ifr.ifr_ifru.ifru_hwaddr.sa_data[..6];
        mac.copy_from_slice(&*(sa_data as *const [i8] as *const [u8]));
    }
    Ok(mac)
}

/// Create raw socket for DHCP
fn create_dhcp_socket() -> Result<i32, String> {
    let sock = unsafe {
        libc::socket(
            libc::AF_INET,
            libc::SOCK_DGRAM,
            libc::IPPROTO_UDP,
        )
    };
    
    if sock < 0 {
        return Err("Failed to create DHCP socket".to_string());
    }
    
    // Allow reuse
    let opt: libc::c_int = 1;
    unsafe {
        libc::setsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &opt as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
    }
    
    // Bind to client port 68
    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 68u16.to_be(),
        sin_addr: libc::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    
    let ret = unsafe {
        libc::bind(
            sock,
            &addr as *const _ as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    
    if ret < 0 {
        unsafe { libc::close(sock); }
        return Err("Failed to bind DHCP socket to port 68".to_string());
    }
    
    Ok(sock)
}

/// Send DHCP packet (broadcast)
fn send_dhcp(sock: i32, pkt: &DhcpPacket) -> Result<(), String> {
    let server_addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 67u16.to_be(),
        sin_addr: libc::in_addr { s_addr: u32::MAX }, // 255.255.255.255
        sin_zero: [0; 8],
    };
    
    let ret = unsafe {
        libc::sendto(
            sock,
            pkt as *const _ as *const libc::c_void,
            std::mem::size_of::<DhcpPacket>(),
            0,
            &server_addr as *const _ as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    
    if ret < 0 {
        return Err("Failed to send DHCP packet".to_string());
    }
    
    Ok(())
}

/// Send DHCP packet unicast (for renewals to specific server)
fn send_dhcp_unicast(sock: i32, pkt: &DhcpPacket, server_ip: [u8; 4]) -> Result<(), String> {
    let server_addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 67u16.to_be(),
        sin_addr: libc::in_addr { 
            s_addr: u32::from_be_bytes(server_ip),
        },
        sin_zero: [0; 8],
    };
    
    let ret = unsafe {
        libc::sendto(
            sock,
            pkt as *const _ as *const libc::c_void,
            std::mem::size_of::<DhcpPacket>(),
            0,
            &server_addr as *const _ as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    
    if ret < 0 {
        return Err("Failed to send unicast DHCP packet".to_string());
    }
    
    Ok(())
}

/// Receive DHCP response with timeout
fn recv_dhcp(sock: i32, xid: u32, timeout_ms: u64) -> Result<DhcpPacket, String> {
    let mut pkt: DhcpPacket = unsafe { std::mem::zeroed() };
    let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    let mut addr_len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
    
    // Set timeout
    let tv = libc::timeval {
        tv_sec: (timeout_ms / 1000) as libc::time_t,
        tv_usec: ((timeout_ms % 1000) * 1000) as libc::suseconds_t,
    };
    unsafe {
        libc::setsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &tv as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::timeval>() as libc::socklen_t,
        );
    }
    
    let ret = unsafe {
        libc::recvfrom(
            sock,
            &mut pkt as *mut _ as *mut libc::c_void,
            std::mem::size_of::<DhcpPacket>(),
            0,
            &mut addr as *mut _ as *mut libc::sockaddr,
            &mut addr_len,
        )
    };
    
    if ret < 0 {
        return Err("DHCP receive timeout".to_string());
    }
    
    if pkt.xid != xid {
        return Err("Transaction ID mismatch".to_string());
    }
    
    Ok(pkt)
}

/// Parse DHCP options from packet
fn parse_dhcp_options(pkt: &DhcpPacket) -> (Option<[u8; 4]>, Option<[u8; 4]>, Option<u32>, Vec<[u8; 4]>) {
    let mut netmask = None;
    let mut gateway = None;
    let mut lease_time = None;
    let mut dns_servers = Vec::new();
    
    let mut i = 0;
    while i < pkt.options.len() - 1 {
        let opt = pkt.options[i];
        if opt == 255 {
            break; // End option
        }
        if opt == 0 {
            i += 1; // Pad option
            continue;
        }
        
        if i + 1 >= pkt.options.len() {
            break;
        }
        let len = pkt.options[i + 1] as usize;
        if i + 2 + len > pkt.options.len() {
            break;
        }
        
        let data = &pkt.options[i + 2..i + 2 + len];
        
        match opt {
            1 if len == 4 => { // Subnet mask
                let mut mask = [0u8; 4];
                mask.copy_from_slice(data);
                netmask = Some(mask);
            }
            3 if len >= 4 => { // Router
                let mut gw = [0u8; 4];
                gw.copy_from_slice(&data[..4]);
                gateway = Some(gw);
            }
            6 => { // DNS servers
                for chunk in data.chunks(4) {
                    if chunk.len() == 4 {
                        let mut dns = [0u8; 4];
                        dns.copy_from_slice(chunk);
                        dns_servers.push(dns);
                    }
                }
            }
            51 if len == 4 => { // Lease time
                lease_time = Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
            }
            _ => {}
        }
        
        i += 2 + len;
    }
    
    (netmask, gateway, lease_time, dns_servers)
}

/// DHCP discovery request - full DORA process
fn dhcp_request(iface: &str) -> Result<DhcpLease, String> {
    let mac = get_mac_address(iface)?;
    let xid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0x12345678);
    
    let sock = create_dhcp_socket()?;
    
    // Step 1: Send DISCOVER
    let discover = DhcpPacket::new_discover(xid, &mac);
    send_dhcp(sock, &discover)?;
    println!("strat-network: DHCP DISCOVER sent");
    
    // Step 2: Receive OFFER
    let offer = recv_dhcp(sock, xid, 5000)?;
    if offer.op != 2 {
        unsafe { libc::close(sock); }
        return Err("Invalid DHCP response".to_string());
    }
    
    let offered_ip = offer.yiaddr;
    let server_ip = offer.siaddr;
    println!("strat-network: DHCP OFFER received from {},{},{},{}",
        server_ip[0], server_ip[1], server_ip[2], server_ip[3]);
    
    // Step 3: Send REQUEST
    let request = DhcpPacket::new_request(xid, &mac, offered_ip, server_ip);
    send_dhcp(sock, &request)?;
    println!("strat-network: DHCP REQUEST sent");
    
    // Step 4: Receive ACK
    let ack = recv_dhcp(sock, xid, 5000)?;
    if ack.op != 2 {
        unsafe { libc::close(sock); }
        return Err("No DHCP ACK received".to_string());
    }
    
    let assigned_ip = ack.yiaddr;
    let (netmask, gateway, lease_time, dns) = parse_dhcp_options(&ack);
    
    unsafe { libc::close(sock); }
    
    println!("strat-network: DHCP ACK received - IP {},{},{},{}",
        assigned_ip[0], assigned_ip[1], assigned_ip[2], assigned_ip[3]);
    
    Ok(DhcpLease {
        ip: assigned_ip,
        netmask: netmask.unwrap_or([255, 255, 255, 0]),
        gateway: gateway.unwrap_or([0, 0, 0, 0]),
        dns: if dns.is_empty() { vec![[8, 8, 8, 8]] } else { dns },
        lease_time: lease_time.unwrap_or(3600),
        obtained: Instant::now(),
        server_ip,
        mac,
    })
}

/// Set IP address using ioctl (SIOCSIFADDR)
fn set_ip_address(iface: &str, ip: [u8; 4], netmask: [u8; 4]) -> Result<(), String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err("Failed to create socket for IP config".to_string());
    }
    
    let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
    let name_bytes = iface.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(
            name_bytes.as_ptr() as *const i8,
            ifr.ifr_name.as_mut_ptr(),
            name_bytes.len().min(15)
        );
    }
    
    // Set IP address
    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from_be_bytes(ip),
        },
        sin_zero: [0; 8],
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &addr as *const _ as *const u8,
            &mut ifr.ifr_ifru.ifru_addr as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    
    let ret = unsafe { libc::ioctl(sock, libc::SIOCSIFADDR, &ifr) };
    if ret < 0 {
        unsafe { libc::close(sock); }
        return Err(format!("Failed to set IP address: {}", std::io::Error::last_os_error()));
    }
    
    // Set netmask
    let netmask_addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from_be_bytes(netmask),
        },
        sin_zero: [0; 8],
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &netmask_addr as *const _ as *const u8,
            &mut ifr.ifr_ifru.ifru_netmask as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    
    let ret = unsafe { libc::ioctl(sock, libc::SIOCSIFNETMASK, &ifr) };
    unsafe { libc::close(sock); }
    
    if ret < 0 {
        return Err(format!("Failed to set netmask: {}", std::io::Error::last_os_error()));
    }
    
    Ok(())
}

/// Bring interface up (SIOCSIFFLAGS)
fn bring_interface_up(iface: &str) -> Result<(), String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err("Failed to create socket".to_string());
    }
    
    let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
    let name_bytes = iface.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(
            name_bytes.as_ptr() as *const i8,
            ifr.ifr_name.as_mut_ptr(),
            name_bytes.len().min(15)
        );
    }
    
    // Get current flags
    let mut ret = unsafe { libc::ioctl(sock, libc::SIOCGIFFLAGS, &mut ifr) };
    if ret < 0 {
        unsafe { libc::close(sock); }
        return Err("Failed to get interface flags".to_string());
    }
    
    // Set UP flag
    unsafe {
        ifr.ifr_ifru.ifru_flags |= libc::IFF_UP as i16;
    }
    
    ret = unsafe { libc::ioctl(sock, libc::SIOCSIFFLAGS, &ifr) };
    unsafe { libc::close(sock); }
    
    if ret < 0 {
        return Err("Failed to bring interface up".to_string());
    }
    
    Ok(())
}

/// Add default route using ioctl (SIOCADDRT)
fn add_default_route(gateway: [u8; 4]) -> Result<(), String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err("Failed to create socket for route".to_string());
    }
    
    let mut rt: libc::rtentry = unsafe { std::mem::zeroed() };
    
    // Destination: 0.0.0.0 (default)
    let dst = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &dst as *const _ as *const u8,
            &mut rt.rt_dst as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    rt.rt_dst.sa_family = libc::AF_INET as libc::sa_family_t;
    
    // Gateway
    let gw = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from_be_bytes(gateway),
        },
        sin_zero: [0; 8],
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &gw as *const _ as *const u8,
            &mut rt.rt_gateway as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    rt.rt_gateway.sa_family = libc::AF_INET as libc::sa_family_t;
    
    // Netmask: 0.0.0.0 for default route
    let genmask = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &genmask as *const _ as *const u8,
            &mut rt.rt_genmask as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    rt.rt_genmask.sa_family = libc::AF_INET as libc::sa_family_t;
    
    // Flags: RTF_UP | RTF_GATEWAY
    rt.rt_flags = libc::RTF_UP | libc::RTF_GATEWAY;
    rt.rt_metric = 0;
    
    let ret = unsafe { libc::ioctl(sock, libc::SIOCADDRT, &rt) };
    unsafe { libc::close(sock); }
    
    // Ignore "route already exists" error
    if ret < 0 {
        let errno = unsafe { *libc::__errno_location() };
        if errno != libc::EEXIST {
            return Err(format!("Failed to add default route: errno {}", errno));
        }
    }
    
    Ok(())
}

/// Write DNS servers to /etc/resolv.conf
fn write_dns_config(dns_servers: &[[u8; 4]]) -> Result<(), String> {
    use std::io::Write;
    
    let mut content = String::new();
    content.push_str("# Generated by strat-network\n");
    for dns in dns_servers {
        content.push_str(&format!("nameserver {}.{}.{}.{}",
            dns[0], dns[1], dns[2], dns[3]));
        content.push('\n');
    }
    
    let mut file = std::fs::File::create("/etc/resolv.conf")
        .map_err(|e| format!("Failed to create resolv.conf: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write resolv.conf: {}", e))?;
    
    Ok(())
}

/// Configure interface with DHCP lease
fn configure_interface(iface: &str, lease: &DhcpLease) -> Result<(), String> {
    // Bring interface up
    bring_interface_up(iface)?;
    
    // Set IP address and netmask
    set_ip_address(iface, lease.ip, lease.netmask)?;
    
    // Add default route if gateway is valid
    if lease.gateway != [0, 0, 0, 0] {
        add_default_route(lease.gateway)?;
    }
    
    // Write DNS configuration
    write_dns_config(&lease.dns)?;
    
    println!("strat-network: interface {} configured", iface);
    Ok(())
}

/// Configure static IP
fn configure_static(iface: &str, ip: [u8; 4], netmask: [u8; 4], gateway: [u8; 4]) -> Result<(), String> {
    // Get MAC for the lease (needed for potential future DHCP fallback)
    let mac = get_mac_address(iface).unwrap_or([0, 0, 0, 0, 0, 0]);
    
    let lease = DhcpLease {
        ip,
        netmask,
        gateway,
        dns: vec![[8, 8, 8, 8]],
        lease_time: u32::MAX,
        obtained: Instant::now(),
        server_ip: [0, 0, 0, 0], // No DHCP server for static
        mac,
    };
    configure_interface(iface, &lease)
}

/// Send DHCPRELEASE to gracefully release lease
fn dhcp_release(iface: &str, lease: &DhcpLease) {
    println!("strat-network: Sending DHCPRELEASE for {},{},{},{}",
        lease.ip[0], lease.ip[1], lease.ip[2], lease.ip[3]);
    
    let sock = create_dhcp_socket();
    if let Ok(sock) = sock {
        let release = DhcpPacket::new_release(&lease.mac, lease.ip, lease.server_ip);
        // Send release (best effort, don't care if it fails)
        let _ = send_dhcp_unicast(sock, &release, lease.server_ip);
        unsafe { libc::close(sock); }
    }
    
    // Remove IP from interface
    let _ = remove_ip_address(iface);
}

/// Remove IP address from interface (set to 0.0.0.0)
fn remove_ip_address(iface: &str) -> Result<(), String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err("Failed to create socket".to_string());
    }
    
    let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
    let name_bytes = iface.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(
            name_bytes.as_ptr() as *const i8,
            ifr.ifr_name.as_mut_ptr(),
            name_bytes.len().min(15)
        );
    }
    
    // Set IP to 0.0.0.0 (removes address)
    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &addr as *const _ as *const u8,
            &mut ifr.ifr_ifru.ifru_addr as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    
    let ret = unsafe { libc::ioctl(sock, libc::SIOCSIFADDR, &ifr) };
    unsafe { libc::close(sock); }
    
    if ret < 0 {
        return Err("Failed to remove IP address".to_string());
    }
    
    Ok(())
}

/// Attempt to renew lease with current server (unicast at T1)
fn dhcp_renew(_iface: &str, lease: &DhcpLease) -> Result<DhcpLease, String> {
    println!("strat-network: Renewing lease at T1 ({}s / {}s elapsed)",
        lease.t1().as_secs(), lease.elapsed().as_secs());
    
    let sock = create_dhcp_socket()?;
    let xid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0x87654321);
    
    let renew = DhcpPacket::new_renew(xid, &lease.mac, lease.ip, lease.server_ip);
    
    // Unicast to server at T1
    send_dhcp_unicast(sock, &renew, lease.server_ip)?;
    
    // Wait for ACK
    let ack = recv_dhcp(sock, xid, 5000)?;
    if ack.op != 2 {
        unsafe { libc::close(sock); }
        return Err("No ACK for renewal".to_string());
    }
    
    let (netmask, gateway, new_lease_time, dns) = parse_dhcp_options(&ack);
    unsafe { libc::close(sock); }
    
    // If server responds with NAK, we need to restart DORA
    // Check if yiaddr changed (should stay same for renewal)
    if ack.yiaddr != lease.ip {
        return Err("Server assigned different IP, need to restart".to_string());
    }
    
    println!("strat-network: Lease renewed for {}s", 
        new_lease_time.unwrap_or(lease.lease_time));
    
    Ok(DhcpLease {
        ip: lease.ip,
        netmask: netmask.unwrap_or(lease.netmask),
        gateway: gateway.unwrap_or(lease.gateway),
        dns: if dns.is_empty() { lease.dns.clone() } else { dns },
        lease_time: new_lease_time.unwrap_or(lease.lease_time),
        obtained: Instant::now(),
        server_ip: lease.server_ip,
        mac: lease.mac,
    })
}

/// Rebind lease - broadcast REQUEST at T2 when unicast to original server failed
fn dhcp_rebind(iface: &str, lease: &DhcpLease) -> Result<DhcpLease, String> {
    println!("strat-network: Rebinding at T2 ({}s / {}s elapsed) - broadcasting",
        lease.t2().as_secs(), lease.elapsed().as_secs());
    
    // Rebinding is essentially a new DORA but with current IP as ciaddr
    let sock = create_dhcp_socket()?;
    let xid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0xABCDEF01);
    
    // Broadcast DISCOVER with current IP as "preferred"
    let mut discover = DhcpPacket::new_discover(xid, &lease.mac);
    discover.ciaddr = lease.ip; // Suggest we want to keep this IP
    
    // Add option 50 to request same IP
    discover.options[13] = 50;
    discover.options[14] = 4;
    discover.options[15..19].copy_from_slice(&lease.ip);
    discover.options[19] = 255;
    
    send_dhcp(sock, &discover)?;
    
    let offer = recv_dhcp(sock, xid, 5000)?;
    if offer.op != 2 {
        unsafe { libc::close(sock); }
        return Err("No OFFER during rebind".to_string());
    }
    
    let new_server = offer.siaddr;
    let new_ip = offer.yiaddr;
    
    // Send REQUEST
    let request = DhcpPacket::new_request(xid, &lease.mac, new_ip, new_server);
    send_dhcp(sock, &request)?;
    
    let ack = recv_dhcp(sock, xid, 5000)?;
    if ack.op != 2 {
        unsafe { libc::close(sock); }
        return Err("No ACK during rebind".to_string());
    }
    
    let (netmask, gateway, lease_time, dns) = parse_dhcp_options(&ack);
    unsafe { libc::close(sock); }
    
    // Reconfigure interface if IP changed
    if new_ip != lease.ip {
        println!("strat-network: Rebind got new IP {},{},{},{}",
            new_ip[0], new_ip[1], new_ip[2], new_ip[3]);
        // Remove old IP, add new one
        let _ = remove_ip_address(iface);
    }
    
    Ok(DhcpLease {
        ip: new_ip,
        netmask: netmask.unwrap_or(lease.netmask),
        gateway: gateway.unwrap_or(lease.gateway),
        dns: if dns.is_empty() { lease.dns.clone() } else { dns },
        lease_time: lease_time.unwrap_or(lease.lease_time),
        obtained: Instant::now(),
        server_ip: new_server,
        mac: lease.mac,
    })
}

/// Maintain DHCP lease (renewal at T1, rebind at T2, release on link down)
fn maintain_lease(iface: &str, lease: &mut DhcpLease, _config: &NetworkConfig) {
    let mut current_lease = lease.clone();
    let mut renewal_attempts = 0;
    const MAX_RENEWAL_ATTEMPTS: u32 = 3;
    
    loop {
        // Check link state first
        if read_link_ready(iface) != LinkState::Up {
            println!("strat-network: link down, releasing lease");
            dhcp_release(iface, &current_lease);
            unsafe { libc::exit(100); } // Signal link down, need to restart
        }
        
        // Check if lease expired
        if current_lease.is_expired() {
            eprintln!("strat-network: lease expired, restarting DORA");
            unsafe { libc::exit(101); } // Signal need full restart
        }
        
        // Check if we should renew (at T1)
        if current_lease.should_renew() {
            match dhcp_renew(iface, &current_lease) {
                Ok(new_lease) => {
                    current_lease = new_lease;
                    renewal_attempts = 0;
                    // Update the original lease reference
                    *lease = current_lease.clone();
                }
                Err(e) => {
                    eprintln!("strat-network: renewal failed: {}", e);
                    renewal_attempts += 1;
                    if renewal_attempts >= MAX_RENEWAL_ATTEMPTS {
                        // Let it go to T2 for rebind
                        renewal_attempts = 0;
                    }
                }
            }
        }
        
        // Check if we should rebind (at T2)
        if current_lease.should_rebind() {
            match dhcp_rebind(iface, &current_lease) {
                Ok(new_lease) => {
                    // Reconfigure if IP changed
                    if new_lease.ip != current_lease.ip {
                        if let Err(e) = configure_interface(iface, &new_lease) {
                            eprintln!("strat-network: rebind reconfig failed: {}", e);
                        }
                    }
                    current_lease = new_lease;
                    *lease = current_lease.clone();
                }
                Err(e) => {
                    eprintln!("strat-network: rebind failed: {}", e);
                    // If rebind fails, lease will expire and we exit
                }
            }
        }
        
        // Sleep until next check (every 5 seconds)
        std::thread::sleep(Duration::from_secs(5));
    }
}

/// Block until link goes down
fn wait_for_link_down(iface: &str) {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        if read_link_ready(iface) != LinkState::Up {
            println!("strat-network: link lost");
            return;
        }
    }
}

/// Check if system is online (has routable IP)
#[allow(dead_code)]
pub fn is_online() -> bool {
    // Check if default route exists
    fs::read_to_string("/proc/net/route")
        .map(|content| content.contains("00000000")) // 0.0.0.0 destination = default route
        .unwrap_or(false)
}

/// Get online state for maintenance window decisions
#[allow(dead_code)]
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
