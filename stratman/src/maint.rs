use libc;
use toml;


#[derive(Debug, Clone)]
pub struct MaintenanceTask {
    pub name: String,
    pub exec: String,
    pub args: Vec<String>,
}

pub fn parse_maint_task(contents: &str) -> Result<MaintenanceTask, String> {
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
    Ok(MaintenanceTask { name, exec, args })
}

pub struct IdleMonitor {
    pub input_fds: Vec<i32>,
    pub last_activity_ms: u64,
    pub idle_threshold_ms: u64,
    pub idle: bool,
    pub current_task_pid: Option<i32>,
    pub task_queue: Vec<MaintenanceTask>,
    pub task_index: usize,
}

impl IdleMonitor {
    pub fn check_activity(&mut self) {
        if self.input_fds.is_empty() {
            return;
        }

        let mut pollfds: Vec<libc::pollfd> = self.input_fds.iter().map(|&fd| libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        }).collect();

        let ret = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, 0) };
        if ret < 0 {
            return;
        }

        let mut had_activity = false;
        for pfd in &pollfds {
            if pfd.revents & libc::POLLIN != 0 {
                let mut buf = [0u8; 1024];
                unsafe {
                    libc::read(pfd.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                }
                had_activity = true;
            }
        }

        if had_activity {
            self.last_activity_ms = now_ms();
            self.idle = false;
        }
    }

    pub fn update_idle_state(&mut self) {
        if self.input_fds.is_empty() {
            return;
        }

        if now_ms() - self.last_activity_ms > self.idle_threshold_ms {
            self.idle = true;
        }
    }

    pub fn is_idle(&self) -> bool {
        self.idle && !self.input_fds.is_empty()
    }
}

fn now_ms() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts); }
    (ts.tv_sec as u64) * 1000 + (ts.tv_nsec as u64) / 1_000_000
}

impl IdleMonitor {
    pub fn init() -> IdleMonitor {
        let mut input_fds = Vec::new();
        
        unsafe {
            let path = std::ffi::CString::new("/dev/input").unwrap();
            let dir = libc::opendir(path.as_ptr());
            
            if !dir.is_null() {
                let mut entry: *mut libc::dirent;
                loop {
                    entry = libc::readdir(dir);
                    if entry.is_null() {
                        break;
                    }
                    
                    let d_name = std::ffi::CStr::from_ptr((*entry).d_name.as_ptr());
                    let d_name_str = d_name.to_str().unwrap_or("");
                    
                    if d_name_str.starts_with("event") {
                        let event_path = format!("/dev/input/{}", d_name_str);
                        let event_path_cstr = std::ffi::CString::new(event_path).unwrap();
                        let fd = libc::open(
                            event_path_cstr.as_ptr(),
                            libc::O_RDONLY | libc::O_NONBLOCK
                        );
                        
                        if fd >= 0 {
                            input_fds.push(fd);
                        }
                    }
                }
                libc::closedir(dir);
            }
        }
        
        let mut monitor = IdleMonitor {
            input_fds,
            last_activity_ms: now_ms(),
            idle_threshold_ms: 300_000,
            idle: false,
            current_task_pid: None,
            task_queue: Vec::new(),
            task_index: 0,
        };
        monitor.load_builtin_tasks();
        monitor.load_user_tasks();
        monitor
    }
    
    pub fn load_builtin_tasks(&mut self) {
        let ldconfig_toml = include_str!("../manifests/maint-ldconfig.toml");
        if let Ok(task) = parse_maint_task(ldconfig_toml) {
            self.task_queue.push(task);
        }
        
        let fontcache_toml = include_str!("../manifests/maint-fontcache.toml");
        if let Ok(task) = parse_maint_task(fontcache_toml) {
            self.task_queue.push(task);
        }
        
        let integrity_toml = include_str!("../manifests/maint-integrity.toml");
        if let Ok(task) = parse_maint_task(integrity_toml) {
            self.task_queue.push(task);
        }
    }
    
    pub fn load_user_tasks(&mut self) {
        if std::path::Path::new("/config/system/maintenance").exists() {
            if let Ok(entries) = std::fs::read_dir("/config/system/maintenance") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                        if let Ok(contents) = std::fs::read_to_string(&path) {
                            if let Ok(task) = parse_maint_task(&contents) {
                                self.task_queue.push(task);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl IdleMonitor {
    pub fn maybe_start_task(&mut self) -> Result<(), String> {
        if self.is_idle() && self.current_task_pid.is_none() && self.task_index < self.task_queue.len() {
            let task = &self.task_queue[self.task_index];

            let exec_cstr = std::ffi::CString::new(task.exec.clone())
                .map_err(|e| format!("Failed to create CString for exec: {}", e))?;

            let mut argv_cstr: Vec<std::ffi::CString> = Vec::new();
            argv_cstr.push(exec_cstr);

            for arg in &task.args {
                argv_cstr.push(std::ffi::CString::new(arg.clone())
                    .map_err(|e| format!("Failed to create CString for arg: {}", e))?);
            }

            let mut argv: Vec<*const i8> = argv_cstr.iter().map(|c| c.as_ptr()).collect();
            argv.push(std::ptr::null());

            unsafe {
                let pid = libc::fork();
                if pid < 0 {
                    return Err(format!("fork failed for maintenance task {}", task.name));
                }

                if pid == 0 {
                    // Maintenance tasks always get strict isolation
                    if libc::unshare(libc::CLONE_NEWNS) < 0 {
                        eprintln!("stratman: unshare failed for maintenance task");
                        libc::_exit(126);
                    }
                    libc::umount2(b"/config\0".as_ptr() as *const libc::c_char, libc::MNT_DETACH);
                    libc::umount2(b"/home\0".as_ptr() as *const libc::c_char, libc::MNT_DETACH);
                    libc::execv(argv[0], argv.as_ptr());
                    libc::_exit(127);
                }

                self.current_task_pid = Some(pid);
                eprintln!("stratman: started maintenance task {}", task.name);
            }
        }
        Ok(())
    }

    pub fn cancel_current_task(&mut self) {
        if let Some(pid) = self.current_task_pid {
            let task_name = self.task_queue.get(self.task_index)
                .map(|t| t.name.as_str())
                .unwrap_or("unknown");

            unsafe {
                libc::kill(pid, libc::SIGKILL);
                let mut status: libc::c_int = 0;
                libc::waitpid(pid, &mut status, 0);
            }

            self.current_task_pid = None;
            self.idle = false;
            eprintln!("stratman: user activity resumed, deferring {}", task_name);
            // Do NOT advance task_index — task reruns next idle window
        }
    }

    pub fn handle_task_exit(&mut self, pid: i32, _status: libc::c_int) {
        if self.current_task_pid == Some(pid) {
            self.current_task_pid = None;
            self.task_index += 1;

            if self.task_index >= self.task_queue.len() {
                self.task_index = 0; // cycle tasks
            }
        }
    }
}
