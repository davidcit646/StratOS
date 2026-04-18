extern crate libc;

mod cmdline;
mod maint;
mod network;
mod service;

fn main() -> ! {
    // Check for network manager mode (called as child process)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--network" {
        // Run as network manager child process
        let config = network::NetworkConfig::default();
        network::run_network_manager(config);
    }
    
    unsafe {
        attach_console_stdio();
        set_environment_variables();
        mount_filesystems();
        
        if let Err(e) = service::load_and_run_all() {
            eprintln!("stratman: service manager failed: {}", e);
            emergency_shell();
        }
        
        emergency_shell();
        reboot(libc::RB_POWER_OFF);
        libc::exit(0);
    }
}

unsafe fn attach_console_stdio() {
    let fd = libc::open(b"/dev/console\0".as_ptr() as *const i8, libc::O_RDWR);
    if fd < 0 {
        return;
    }
    libc::dup2(fd, libc::STDIN_FILENO);
    libc::dup2(fd, libc::STDOUT_FILENO);
    libc::dup2(fd, libc::STDERR_FILENO);
    if fd > libc::STDERR_FILENO {
        libc::close(fd);
    }
}

unsafe fn set_environment_variables() {
    libc::setenv(b"HOME\0".as_ptr() as *const i8, b"/home\0".as_ptr() as *const i8, 1);
    libc::setenv(b"PATH\0".as_ptr() as *const i8, b"/bin:/sbin:/usr/bin:/usr/sbin\0".as_ptr() as *const i8, 1);
    libc::setenv(b"TERM\0".as_ptr() as *const i8, b"linux\0".as_ptr() as *const i8, 1);
    libc::setenv(b"XDG_RUNTIME_DIR\0".as_ptr() as *const i8, b"/run\0".as_ptr() as *const i8, 1);
    libc::setenv(b"WLR_RENDERER\0".as_ptr() as *const i8, b"pixman\0".as_ptr() as *const i8, 1);
    libc::setenv(b"WLR_LIBINPUT_NO_DEVICES\0".as_ptr() as *const i8, b"1\0".as_ptr() as *const i8, 1);
    libc::setenv(b"WLR_RENDERER_ALLOW_SOFTWARE\0".as_ptr() as *const i8, b"1\0".as_ptr() as *const i8, 1);
}

unsafe fn mount_filesystems() {
    let cl = cmdline::read_proc_cmdline();
    let cfg = cmdline::path_to_cstring(&cmdline::resolved_partition(&cl, "config", 5))
        .expect("stratman: config device path");
    let apps = cmdline::path_to_cstring(&cmdline::resolved_partition(&cl, "apps", 6))
        .expect("stratman: apps device path");
    let home = cmdline::path_to_cstring(&cmdline::resolved_partition(&cl, "home", 7))
        .expect("stratman: home device path");

    ensure_dir(b"/proc\0");
    ensure_dir(b"/sys\0");
    ensure_dir(b"/dev\0");
    ensure_dir(b"/run\0");
    ensure_dir(b"/tmp\0");
    ensure_dir(b"/dev/shm\0");
    ensure_dir(b"/config\0");
    ensure_dir(b"/apps\0");
    ensure_dir(b"/home\0");
    ensure_dir(b"/var\0");
    ensure_dir(b"/config/var\0");
    ensure_dir(b"/etc\0");
    ensure_dir(b"/config/etc\0");

    mount_best_effort(b"proc\0", b"/proc\0", b"proc\0", 0, core::ptr::null());
    mount_best_effort(b"sys\0", b"/sys\0", b"sysfs\0", 0, core::ptr::null());
    mount_best_effort(b"dev\0", b"/dev\0", b"devtmpfs\0", 0, core::ptr::null());
    mount_best_effort_source(cfg.as_ptr(), b"/config\0", b"ext4\0", 0, core::ptr::null());
    mount_best_effort_source(apps.as_ptr(), b"/apps\0", b"ext4\0", 0, core::ptr::null());
    mount_best_effort_source(home.as_ptr(), b"/home\0", b"btrfs\0", 0, core::ptr::null());
    mount_best_effort(b"/config/var\0", b"/var\0", b"", libc::MS_BIND, core::ptr::null());
    mount_best_effort(b"/config/etc\0", b"/etc\0", b"", libc::MS_BIND, core::ptr::null());
    ensure_dir(b"/var/cache\0");
    ensure_dir(b"/var/cache/fontconfig\0");
    ensure_dir(b"/dev/pts\0");
    mount_best_effort(b"devpts\0", b"/dev/pts\0", b"devpts\0", 0, b"mode=620,ptmxmode=666\0".as_ptr() as *const i8);
    mount_best_effort(b"/dev/pts/ptmx\0", b"/dev/ptmx\0", b"", libc::MS_BIND, core::ptr::null());
    mount_best_effort(b"tmpfs\0", b"/run\0", b"tmpfs\0", 0, b"mode=755\0".as_ptr() as *const i8);
    mount_best_effort(b"tmpfs\0", b"/tmp\0", b"tmpfs\0", 0, b"mode=1777\0".as_ptr() as *const i8);
    mount_best_effort(b"tmpfs\0", b"/dev/shm\0", b"tmpfs\0", 0, b"mode=1777\0".as_ptr() as *const i8);
}

unsafe fn ensure_dir(path: &[u8]) {
    let path_ptr = path.as_ptr() as *const i8;
    libc::mkdir(path_ptr, 0o755);
}

unsafe fn mount_best_effort_source(
    source: *const i8,
    target: &[u8],
    fstype: &[u8],
    flags: libc::c_ulong,
    data: *const i8,
) {
    let target_ptr = target.as_ptr() as *const i8;
    let fstype_ptr = if fstype.is_empty() {
        core::ptr::null()
    } else {
        fstype.as_ptr() as *const i8
    };

    let result = libc::mount(
        source,
        target_ptr,
        fstype_ptr,
        flags,
        data as *const libc::c_void,
    );
    if result < 0 {
        let errno = *libc::__errno_location();
        if errno != libc::EBUSY && errno != libc::EEXIST {
            let msg = b"stratman: mount failed\0";
            libc::write(libc::STDERR_FILENO, msg.as_ptr() as *const libc::c_void, msg.len());
            libc::write(libc::STDERR_FILENO, target_ptr as *const libc::c_void, target.len());
            let newline = b"\n\0";
            libc::write(libc::STDERR_FILENO, newline.as_ptr() as *const libc::c_void, newline.len());
        }
    }
}

unsafe fn mount_best_effort(
    source: &[u8],
    target: &[u8],
    fstype: &[u8],
    flags: libc::c_ulong,
    data: *const i8,
) {
    let source_ptr = source.as_ptr() as *const i8;
    let target_ptr = target.as_ptr() as *const i8;
    let fstype_ptr = if fstype.is_empty() {
        core::ptr::null()
    } else {
        fstype.as_ptr() as *const i8
    };

    let result = libc::mount(source_ptr, target_ptr, fstype_ptr, flags, data as *const libc::c_void);
    if result < 0 {
        let errno = *libc::__errno_location();
        if errno != libc::EBUSY && errno != libc::EEXIST {
            let msg = b"stratman: mount failed\0";
            libc::write(libc::STDERR_FILENO, msg.as_ptr() as *const libc::c_void, msg.len());
            libc::write(libc::STDERR_FILENO, target_ptr as *const libc::c_void, target.len());
            let newline = b"\n\0";
            libc::write(libc::STDERR_FILENO, newline.as_ptr() as *const libc::c_void, newline.len());
        }
    }
}

unsafe fn emergency_shell() {
    let prompt = b"stratos# \0";
    let mut line: [i8; 512] = [0; 512];

    let stdin = libc::fdopen(libc::STDIN_FILENO, b"r\0".as_ptr() as *const i8);
    if stdin.is_null() {
        libc::exit(1);
    }

    loop {
        libc::write(libc::STDERR_FILENO, prompt.as_ptr() as *const libc::c_void, prompt.len());

        if libc::fgets(line.as_mut_ptr(), line.len() as libc::c_int, stdin).is_null() {
            libc::sleep(1);
            continue;
        }

        let len = libc::strlen(line.as_ptr());
        if len > 0 && *line.as_ptr().add(len - 1) == b'\n' as i8 {
            *line.as_mut_ptr().add(len - 1) = 0;
        }

        if *line.as_ptr() == 0 {
            continue;
        }

        let sh_path = b"/bin/sh\0";
        let sh_arg = b"-c\0";
        let mut argv = [
            sh_path.as_ptr() as *const i8,
            sh_arg.as_ptr() as *const i8,
            line.as_ptr(),
            core::ptr::null(),
        ];

        let pid = libc::fork();
        if pid < 0 {
            continue;
        }

        if pid == 0 {
            libc::execvp(sh_path.as_ptr() as *const i8, argv.as_mut_ptr());
            libc::_exit(127);
        }

        let mut status: libc::c_int = 0;
        libc::waitpid(pid, &mut status, 0);
    }
}

unsafe fn reboot(cmd: libc::c_int) {
    libc::reboot(cmd);
    libc::exit(1);
}
