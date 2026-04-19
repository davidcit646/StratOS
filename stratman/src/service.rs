use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ServiceManifest {
    pub name: String,
    pub exec: String,
    pub args: Vec<String>,
    pub restart: RestartPolicy,
    pub depends: Vec<String>,
    pub socket: Option<String>,
    pub socket_timeout_ms: u64,
    pub env: Vec<(String, String)>,
    pub oneshot: bool,
    pub namespace: NamespacePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

impl RestartPolicy {
    fn from_str(s: &str) -> Self {
        match s {
            "always" => RestartPolicy::Always,
            "on-failure" => RestartPolicy::OnFailure,
            _ => RestartPolicy::Never,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespacePolicy {
    None,
    ReadonlyUser,
    Strict,
}

impl NamespacePolicy {
    fn from_str(s: &str) -> Self {
        match s {
            "readonly-user" => Self::ReadonlyUser,
            "strict" => Self::Strict,
            _ => Self::None,
        }
    }
}

const MAX_RESTART_ATTEMPTS: u32 = 10;

fn parse_manifest(contents: &str) -> Result<ServiceManifest, String> {
    let value: toml::Value = toml::from_str(contents)
        .map_err(|e| format!("TOML parse error: {}", e))?;
    let t = value.as_table().ok_or("TOML root is not a table")?;

    let name = t.get("name").and_then(|v| v.as_str())
        .ok_or("Missing required field: name")?.to_string();
    let exec = t.get("exec").and_then(|v| v.as_str())
        .ok_or("Missing required field: exec")?.to_string();
    let args = t.get("args").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let restart = t.get("restart").and_then(|v| v.as_str())
        .map(RestartPolicy::from_str).unwrap_or(RestartPolicy::Never);
    let depends = t.get("depends").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let socket = t.get("socket").and_then(|v| v.as_str()).map(String::from);
    let socket_timeout_ms = t.get("socket_timeout_ms").and_then(|v| v.as_integer())
        .unwrap_or(5000) as u64;
    let env = t.get("env").and_then(|v| v.as_array())
        .map(|a| a.iter()
            .filter_map(|v| v.as_str())
            .filter_map(|s| s.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect())
        .unwrap_or_default();
    let oneshot = t.get("oneshot").and_then(|v| v.as_bool()).unwrap_or(false);
    let namespace = t.get("namespace").and_then(|v| v.as_str())
        .map(NamespacePolicy::from_str).unwrap_or(NamespacePolicy::None);

    Ok(ServiceManifest { name, exec, args, restart, depends, socket, socket_timeout_ms, env, oneshot, namespace })
}

pub fn load_and_run_all() -> Result<(), String> {
    let mut manifests = Vec::new();

    // Load built-in manifests
    let seatd_toml = include_str!("../manifests/seatd.toml");
    manifests.push(parse_manifest(seatd_toml)?);

    let strat_live_welcome_toml = include_str!("../manifests/strat-live-welcome.toml");
    manifests.push(parse_manifest(strat_live_welcome_toml)?);

    let stratwm_toml = include_str!("../manifests/stratwm.toml");
    manifests.push(parse_manifest(stratwm_toml)?);

    let strat_wpa_toml = include_str!("../manifests/strat-wpa.toml");
    manifests.push(parse_manifest(strat_wpa_toml)?);

    let strat_network_toml = include_str!("../manifests/strat-network.toml");
    manifests.push(parse_manifest(strat_network_toml)?);

    let validate_boot_toml = include_str!("../manifests/validate-boot.toml");
    manifests.push(parse_manifest(validate_boot_toml)?);

    let indexer_boot_toml = include_str!("../manifests/indexer-boot.toml");
    manifests.push(parse_manifest(indexer_boot_toml)?);

    // Check for overrides in /config/system/services/
    if Path::new("/config/system/services").exists() {
        let entries = fs::read_dir("/config/system/services")
            .map_err(|e| format!("Failed to read /config/system/services: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let _filename = path.file_name()
                    .and_then(|s| s.to_str())
                    .ok_or("Invalid filename")?;
                let contents = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
                let manifest = parse_manifest(&contents)?;
                // Override or add
                if let Some(pos) = manifests.iter().position(|m| m.name == manifest.name) {
                    manifests[pos] = manifest;
                } else {
                    manifests.push(manifest);
                }
            }
        }
    }

    // Topological sort with cycle detection
    let sorted = topological_sort(&manifests)?;

    // Spawn all services in order and track state
    let mut services: HashMap<String, ServiceState> = HashMap::new();
    for manifest in sorted {
        let state = spawn_service(manifest)?;
        services.insert(state.manifest.name.clone(), state);
    }

    let mut monitor = crate::maint::IdleMonitor::init();

    // Main event loop
    loop {
        let mut status: libc::c_int = 0;
        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
        
        if pid > 0 {
            // Maintenance task exit takes priority — check before service lookup
            if monitor.current_task_pid == Some(pid) {
                monitor.handle_task_exit(pid, status);
            } else {
                let mut service_name = None;
                for (name, state) in &services {
                    if state.pid == Some(pid) {
                        service_name = Some(name.clone());
                        break;
                    }
                }
                if let Some(name) = service_name {
                    if let Some(state) = services.get_mut(&name) {
                        handle_service_exit(state, status)?;
                    }
                }
            }
        }

        // Maintenance window — only cancel if task is STILL running
        monitor.check_activity();
        monitor.update_idle_state();

        if monitor.is_idle() && monitor.current_task_pid.is_none() {
            if let Err(e) = monitor.maybe_start_task() {
                eprintln!("stratman: {}", e);
            }
        } else if !monitor.is_idle() && monitor.current_task_pid.is_some() {
            monitor.cancel_current_task();
        }
        
        // Sleep 100ms between polls
        unsafe {
            let ts = libc::timespec { tv_sec: 0, tv_nsec: 100_000_000 };
            libc::nanosleep(&ts, core::ptr::null_mut());
        }
    }
}

struct ServiceState {
    pid: Option<i32>,
    restart_count: u32,
    backoff_ms: u64,
    manifest: ServiceManifest,
}

fn topological_sort(manifests: &[ServiceManifest]) -> Result<Vec<ServiceManifest>, String> {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut service_map: HashMap<&str, &ServiceManifest> = HashMap::new();

    for manifest in manifests {
        service_map.insert(&manifest.name, manifest);
        graph.insert(&manifest.name, Vec::new());
        in_degree.insert(&manifest.name, 0);
    }

    for manifest in manifests {
        for dep in &manifest.depends {
            if service_map.contains_key(dep.as_str()) {
                graph.entry(dep).or_insert_with(Vec::new).push(&manifest.name);
                *in_degree.entry(&manifest.name).or_insert(0) += 1;
            } else {
                eprintln!("stratman: warning: service {} depends on unknown service {}", manifest.name, dep);
            }
        }
    }

    let mut queue: Vec<&str> = in_degree.iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();

    let mut result = Vec::new();
    let mut visited = Vec::new();

    while let Some(name) = queue.pop() {
        if visited.contains(&name) {
            continue;
        }
        visited.push(name);

        if let Some(&manifest) = service_map.get(name) {
            result.push(manifest.clone());
        }

        if let Some(deps) = graph.get(name) {
            for dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(*dep);
                    }
                }
            }
        }
    }

    if result.len() != manifests.len() {
        // Detect cycle
        let remaining: Vec<_> = in_degree.iter()
            .filter(|(_, &deg)| deg > 0)
            .map(|(&name, _)| name)
            .collect();
        return Err(format!("Dependency cycle detected involving services: {:?}", remaining));
    }

    Ok(result)
}

fn apply_namespace_mounts(policy: NamespacePolicy) {
    if policy == NamespacePolicy::None { return; }

    if unsafe { libc::unshare(libc::CLONE_NEWNS) } < 0 {
        eprintln!("stratman: unshare failed");
        unsafe { libc::_exit(126); }
    }

    match policy {
        NamespacePolicy::ReadonlyUser => {
            for path in &[b"/config\0".as_ptr(), b"/home\0".as_ptr()] {
                unsafe {
                    libc::mount(*path as *const libc::c_char, *path as *const libc::c_char,
                        core::ptr::null(), libc::MS_BIND, core::ptr::null());
                    libc::mount(*path as *const libc::c_char, *path as *const libc::c_char,
                        core::ptr::null(), libc::MS_BIND | libc::MS_RDONLY | libc::MS_REMOUNT, core::ptr::null());
                }
            }
        }
        NamespacePolicy::Strict => {
            for path in &[b"/config\0".as_ptr(), b"/home\0".as_ptr()] {
                unsafe { libc::umount2(*path as *const libc::c_char, libc::MNT_DETACH); }
            }
        }
        NamespacePolicy::None => {}
    }
}

fn spawn_service(manifest: ServiceManifest) -> Result<ServiceState, String> {
    let exec_cstr = std::ffi::CString::new(manifest.exec.clone())
        .map_err(|e| format!("Failed to create CString for exec: {}", e))?;
    
    let mut argv_cstr: Vec<std::ffi::CString> = Vec::new();
    argv_cstr.push(exec_cstr);
    
    for arg in &manifest.args {
        argv_cstr.push(std::ffi::CString::new(arg.clone())
            .map_err(|e| format!("Failed to create CString for arg: {}", e))?);
    }
    
    let mut argv: Vec<*const i8> = argv_cstr.iter().map(|c| c.as_ptr()).collect();
    argv.push(std::ptr::null());
    
    let mut env_cstr: Vec<std::ffi::CString> = Vec::new();
    for (key, value) in &manifest.env {
        let env_str = format!("{}={}", key, value);
        env_cstr.push(std::ffi::CString::new(env_str)
            .map_err(|e| format!("Failed to create CString for env: {}", e))?);
    }
    let mut envp: Vec<*const i8> = env_cstr.iter().map(|c| c.as_ptr()).collect();
    envp.push(std::ptr::null());
    
    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            return Err(format!("fork failed: {}", std::io::Error::last_os_error()));
        }

        if pid == 0 {
            apply_namespace_mounts(manifest.namespace);

            // Set environment variables
            for env_ptr in &envp {
                if !env_ptr.is_null() {
                    libc::putenv(*env_ptr as *mut i8);
                }
            }

            libc::execv(argv[0], argv.as_ptr());
            libc::_exit(127);
        }

        // If this service has a socket, wait for it NOW (child is running)
        if let Some(ref socket_path) = manifest.socket {
            if !wait_for_socket(socket_path, manifest.socket_timeout_ms) {
                // timeout — kill child, return error
                libc::kill(pid, libc::SIGTERM);
                return Err(format!("socket {} never appeared", socket_path));
            }
        }

        Ok(ServiceState {
            pid: Some(pid),
            restart_count: 0,
            backoff_ms: 100,
            manifest,
        })
    }
}

fn handle_service_exit(state: &mut ServiceState, status: libc::c_int) -> Result<(), String> {
    state.pid = None;
    
    let exited = libc::WIFEXITED(status);
    let exit_status = if exited { libc::WEXITSTATUS(status) } else { 1 };
    let signaled = libc::WIFSIGNALED(status);
    let signal = if signaled { libc::WTERMSIG(status) } else { 0 };

    if state.manifest.oneshot {
        eprintln!("stratman: oneshot service {} exited (status={}, signal={})", state.manifest.name, exit_status, signal);
        return Ok(());
    }

    let should_restart = match state.manifest.restart {
        RestartPolicy::Always => true,
        RestartPolicy::OnFailure => exit_status != 0 || signaled,
        RestartPolicy::Never => false,
    };

    if should_restart {
        if state.restart_count >= MAX_RESTART_ATTEMPTS {
            eprintln!("stratman: service {} exceeded max restart attempts ({}), giving up", 
                state.manifest.name, MAX_RESTART_ATTEMPTS);
            return Ok(());
        }
        
        state.restart_count += 1;
        state.backoff_ms = exponential_backoff(state.restart_count);
        
        eprintln!("stratman: service {} exited (status={}, signal={}), restarting in {}ms", 
            state.manifest.name, exit_status, signal, state.backoff_ms);
        
        // Sleep for backoff
        unsafe {
            let sec = state.backoff_ms / 1000;
            let nsec = (state.backoff_ms % 1000) * 1_000_000;
            let ts = libc::timespec { tv_sec: sec as i64, tv_nsec: nsec as i64 };
            libc::nanosleep(&ts, core::ptr::null_mut());
        }
        
        // Respawn service
        match spawn_service(state.manifest.clone()) {
            Ok(new_state) => {
                state.pid = new_state.pid;
            }
            Err(e) => {
                eprintln!("stratman: failed to respawn service {}: {}", state.manifest.name, e);
            }
        }
    } else {
        eprintln!("stratman: service {} exited cleanly, not restarting", state.manifest.name);
    }
    
    Ok(())
}


fn wait_for_socket(path: &str, timeout_ms: u64) -> bool {
    let path_cstr = std::ffi::CString::new(path).unwrap();
    let mut elapsed_ms: u64 = 0;
    
    while elapsed_ms < timeout_ms {
        unsafe {
            if libc::access(path_cstr.as_ptr(), libc::F_OK) == 0 {
                return true;
            }
        }
        
        unsafe {
            let ts = libc::timespec { tv_sec: 0, tv_nsec: 100_000_000 };
            libc::nanosleep(&ts, core::ptr::null_mut());
        }
        elapsed_ms += 100;
    }

    false
}

fn exponential_backoff(restart_count: u32) -> u64 {
    let base_ms: u64 = 100;
    let max_ms: u64 = 30000;
    let backoff = base_ms * 2u64.pow(restart_count.saturating_sub(1).min(8));
    backoff.min(max_ms)
}
