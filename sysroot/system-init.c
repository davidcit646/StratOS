#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static void attach_console_stdio(void) {
    int fd = open("/dev/console", O_RDWR);
    if (fd < 0) {
        return;
    }
    (void)dup2(fd, STDIN_FILENO);
    (void)dup2(fd, STDOUT_FILENO);
    (void)dup2(fd, STDERR_FILENO);
    if (fd > STDERR_FILENO) {
        close(fd);
    }
}

static void log_status(const char *msg) {
    if (msg == NULL) {
        return;
    }
    fprintf(stderr, "system-init: %s\n", msg);
    int kmsg = open("/dev/kmsg", O_WRONLY);
    if (kmsg >= 0) {
        dprintf(kmsg, "<0>system-init: %s\n", msg);
        close(kmsg);
    }
}

static void ensure_dir(const char *path) {
    if (mkdir(path, 0755) < 0 && errno != EEXIST) {
        fprintf(stderr, "system-init: mkdir %s failed: %s\n", path, strerror(errno));
    }
}

static void mount_best_effort(
    const char *source,
    const char *target,
    const char *fstype,
    unsigned long flags,
    const char *data
) {
    if (mount(source, target, fstype, flags, data) < 0 &&
        errno != EBUSY &&
        errno != EEXIST) {
        fprintf(stderr, "system-init: mount %s on %s failed: %s\n", source, target, strerror(errno));
    }
}

static int try_exec(const char *path) {
    char msg[256];
    if (access(path, X_OK) != 0) {
        snprintf(msg, sizeof(msg), "access %s failed: %s", path, strerror(errno));
        log_status(msg);
        return -1;
    }

    char *const argv[] = {(char *)path, NULL};
    execv(path, argv);
    snprintf(msg, sizeof(msg), "exec %s failed: %s", path, strerror(errno));
    log_status(msg);
    return -1;
}

static int spawn_and_wait(const char *path) {
    pid_t pid = fork();
    if (pid < 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "fork %s failed: %s", path, strerror(errno));
        log_status(msg);
        return -1;
    }

    if (pid == 0) {
        char *const argv[] = {(char *)path, NULL};
        execv(path, argv);
        dprintf(STDERR_FILENO, "system-init: exec %s failed in child: %s\n", path, strerror(errno));
        _exit(111);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        char msg[256];
        snprintf(msg, sizeof(msg), "waitpid %s failed: %s", path, strerror(errno));
        log_status(msg);
        return -1;
    }

    if (WIFEXITED(status)) {
        char msg[256];
        snprintf(msg, sizeof(msg), "%s exited status=%d", path, WEXITSTATUS(status));
        log_status(msg);
        return WEXITSTATUS(status);
    }

    if (WIFSIGNALED(status)) {
        char msg[256];
        snprintf(msg, sizeof(msg), "%s killed by signal=%d", path, WTERMSIG(status));
        log_status(msg);
    }
    return -1;
}

static void run_once_if_present(const char *path) {
    if (access(path, X_OK) != 0) {
        return;
    }

    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "system-init: fork for %s failed: %s\n", path, strerror(errno));
        return;
    }

    if (pid == 0) {
        char *const argv[] = {(char *)path, NULL};
        execv(path, argv);
        _exit(127);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        fprintf(stderr, "system-init: waitpid for %s failed: %s\n", path, strerror(errno));
        return;
    }

    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        fprintf(stderr, "system-init: %s exited non-zero (%d)\n", path, status);
    }
}

static void probe_file(const char *path) {
    struct stat st;
    char msg[256];
    if (stat(path, &st) == 0) {
        snprintf(msg, sizeof(msg), "probe ok: %s", path);
    } else {
        snprintf(msg, sizeof(msg), "probe missing: %s (%s)", path, strerror(errno));
    }
    log_status(msg);
}

static void probe_dynamic_linker(const char *path) {
    pid_t pid = fork();
    if (pid < 0) {
        return;
    }
    if (pid == 0) {
        char *const argv[] = {
            "/lib64/ld-linux-x86-64.so.2",
            "--list",
            (char *)path,
            NULL
        };
        execv(argv[0], argv);
        _exit(111);
    }
    int status = 0;
    (void)waitpid(pid, &status, 0);
}

static pid_t spawn_seatd(void) {
    if (access("/bin/seatd", X_OK) != 0) {
        log_status("seatd not found, skipping");
        return -1;
    }
    pid_t pid = fork();
    if (pid < 0) {
        log_status("fork seatd failed");
        return -1;
    }
    if (pid == 0) {
        char *const argv[] = {"/bin/seatd", NULL};
        execv("/bin/seatd", argv);
        _exit(127);
    }
    log_status("seatd spawned");
    return pid;
}

static int wait_for_socket(const char *path, int retries, int delay_ms) {
    struct stat st;
    for (int i = 0; i < retries; i++) {
        if (stat(path, &st) == 0) return 0;
        usleep((useconds_t)delay_ms * 1000);
    }
    return -1;
}

static void emergency_shell(void) {
    char line[512];
    log_status("entering emergency shell");
    setenv("PATH", "/bin:/usr/bin:/sbin:/usr/sbin", 1);

    for (;;) {
        fprintf(stderr, "stratos# ");
        fflush(stderr);

        if (fgets(line, sizeof(line), stdin) == NULL) {
            sleep(1);
            continue;
        }

        size_t len = strlen(line);
        if (len > 0 && line[len - 1] == '\n') {
            line[len - 1] = '\0';
        }
        if (line[0] == '\0') {
            continue;
        }

        if (strcmp(line, "exit") == 0) {
            continue;
        }

        if (strncmp(line, "cd ", 3) == 0) {
            if (chdir(line + 3) < 0) {
                fprintf(stderr, "cd failed: %s\n", strerror(errno));
            }
            continue;
        }

        char *argv[64];
        size_t argc = 0;
        char *tok = strtok(line, " \t");
        while (tok != NULL && argc < (sizeof(argv) / sizeof(argv[0]) - 1)) {
            argv[argc++] = tok;
            tok = strtok(NULL, " \t");
        }
        argv[argc] = NULL;
        if (argc == 0) {
            continue;
        }

        pid_t pid = fork();
        if (pid < 0) {
            fprintf(stderr, "fork failed: %s\n", strerror(errno));
            continue;
        }
        if (pid == 0) {
            execvp(argv[0], argv);
            fprintf(stderr, "exec %s failed: %s\n", argv[0], strerror(errno));
            _exit(127);
        }

        int status = 0;
        (void)waitpid(pid, &status, 0);
    }
}

int main(void) {
    attach_console_stdio();
    log_status("start");
    setenv("HOME", "/home", 1);
    setenv("XDG_RUNTIME_DIR", "/run", 1);
    setenv("XDG_CACHE_HOME", "/var/cache", 1);
    setenv("WAYLAND_DEBUG", "1", 1);
    unsetenv("LIBSEAT_BACKEND");
    unsetenv("SEATD_SOCK");
    setenv("WLR_RENDERER_ALLOW_SOFTWARE", "1", 1);
    setenv("WLR_RENDERER", "pixman", 1);
    setenv("WLR_LIBINPUT_NO_DEVICES", "1", 1);
    setenv("LD_LIBRARY_PATH", "/lib64:/usr/lib64:/usr/lib/x86_64-linux-gnu:/lib/x86_64-linux-gnu", 1);
    probe_file("/bin/stratwm");
    probe_file("/bin/sh");
    probe_file("/lib64/ld-linux-x86-64.so.2");
    probe_file("/lib64/libwlroots-0.19.so");
    probe_file("/lib64/libreadline.so.8");
    probe_dynamic_linker("/bin/stratwm");
    probe_dynamic_linker("/bin/sh");

    ensure_dir("/proc");
    ensure_dir("/sys");
    ensure_dir("/dev");
    ensure_dir("/run");
    ensure_dir("/tmp");
    ensure_dir("/config");
    ensure_dir("/apps");
    ensure_dir("/home");
    ensure_dir("/var");

    mount_best_effort("proc", "/proc", "proc", 0, NULL);
    mount_best_effort("sys", "/sys", "sysfs", 0, NULL);
    mount_best_effort("dev", "/dev", "devtmpfs", 0, NULL);
    ensure_dir("/dev/shm");
    ensure_dir("/dev/pts");
    mount_best_effort("/dev/sda5", "/config", "ext4", 0, NULL);
    mount_best_effort("/dev/sda6", "/apps", "ext4", 0, NULL);
    mount_best_effort("/dev/sda7", "/home", "btrfs", 0, NULL);
    ensure_dir("/config/var");
    mount_best_effort("/config/var", "/var", NULL, MS_BIND, NULL);
    mount_best_effort("devpts", "/dev/pts", "devpts", 0, "mode=620,ptmxmode=666");
    mount_best_effort("tmpfs", "/run", "tmpfs", 0, "mode=755");
    mount_best_effort("tmpfs", "/tmp", "tmpfs", 0, "mode=1777");
    mount_best_effort("tmpfs", "/dev/shm", "tmpfs", 0, "mode=1777");
    probe_file("/dev/pts/ptmx");
    if (access("/dev/ptmx", F_OK) == 0 && unlink("/dev/ptmx") < 0) {
        fprintf(stderr, "system-init: unlink /dev/ptmx failed: %s\n", strerror(errno));
    }
    if (mount("/dev/pts/ptmx", "/dev/ptmx", NULL, MS_BIND, NULL) < 0) {
        fprintf(stderr, "system-init: bind /dev/ptmx failed: %s, trying symlink\n", strerror(errno));
        if (symlink("pts/ptmx", "/dev/ptmx") < 0) {
            fprintf(stderr, "system-init: symlink /dev/ptmx -> pts/ptmx failed: %s\n", strerror(errno));
        }
    }
    probe_file("/dev/ptmx");

    probe_file("/bin/seatd");
    spawn_seatd();
    if (wait_for_socket("/run/seatd.sock", 50, 10) == 0) {
        log_status("seatd socket ready");
        setenv("LIBSEAT_BACKEND", "seatd", 1);
        setenv("SEATD_SOCK", "/run/seatd.sock", 1);
    } else {
        log_status("seatd socket timeout, continuing anyway");
    }

    ensure_dir("/var/cache");
    ensure_dir("/var/cache/fontconfig");

    run_once_if_present("/bin/strat-validate-boot");
    run_once_if_present("/bin/strat-indexer-boot.sh");

    // Wait for input devices so libinput can discover them.
    // QEMU PS/2 devices appear late; without udev, wlroots can't hotplug.
    ensure_dir("/dev/input");
    if (wait_for_socket("/dev/input/event0", 100, 20) == 0) {
        log_status("input devices ready");
    } else {
        log_status("input devices not found, continuing anyway");
    }

    // Keep PID 1 alive: spawn boot target as a child and observe exit status.
    // If stratwm exits quickly, fall back to a shell instead of panicking.
    log_status("trying /bin/stratwm");
    if (access("/bin/stratwm", X_OK) == 0) {
        int rc = spawn_and_wait("/bin/stratwm");
        if (rc == 0) {
            log_status("/bin/stratwm exited cleanly, entering idle loop");
            for (;;) {
                pause();
            }
        }
    } else {
        try_exec("/bin/stratwm");
    }

    log_status("trying /usr/bin/stratwm");
    if (access("/usr/bin/stratwm", X_OK) == 0) {
        int rc = spawn_and_wait("/usr/bin/stratwm");
        if (rc == 0) {
            log_status("/usr/bin/stratwm exited cleanly, entering idle loop");
            for (;;) {
                pause();
            }
        }
    } else {
        try_exec("/usr/bin/stratwm");
    }

    log_status("trying /bin/sh");
    if (access("/bin/sh", X_OK) == 0) {
        int rc = spawn_and_wait("/bin/sh");
        if (rc == 0) {
            log_status("/bin/sh exited cleanly");
        } else {
            log_status("/bin/sh failed, using built-in emergency shell");
        }
        emergency_shell();
    } else {
        try_exec("/bin/sh");
    }

    log_status("no launch target found, using built-in emergency shell");
    emergency_shell();
}
