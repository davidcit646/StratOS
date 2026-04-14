#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

static void wait_forever(void) {
    for (;;) {
        sleep(1);
    }
}

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

static void die_errno(const char *msg) {
    int saved = errno;
    fprintf(stderr, "init: %s: %s\n", msg, strerror(saved));
    wait_forever();
}

static void ensure_dir(const char *path) {
    if (mkdir(path, 0755) < 0 && errno != EEXIST) {
        die_errno(path);
    }
}

static void mount_or_die(
    const char *source,
    const char *target,
    const char *fstype,
    unsigned long flags,
    const char *data,
    const char *label
) {
    if (mount(source, target, fstype, flags, data) < 0) {
        die_errno(label);
    }
}

static void read_root_device(char *out, size_t out_len) {
    const char *fallback = "/dev/sda2";
    if (out == NULL || out_len == 0) {
        return;
    }

    snprintf(out, out_len, "%s", fallback);

    int fd = open("/proc/cmdline", O_RDONLY);
    if (fd < 0) {
        return;
    }

    char cmdline[2048];
    ssize_t n = read(fd, cmdline, sizeof(cmdline) - 1);
    close(fd);
    if (n <= 0) {
        return;
    }
    cmdline[n] = '\0';

    char *saveptr = NULL;
    char *tok = strtok_r(cmdline, " \t\r\n", &saveptr);
    while (tok != NULL) {
        if (strncmp(tok, "root=", 5) == 0 && tok[5] != '\0') {
            snprintf(out, out_len, "%s", tok + 5);
            return;
        }
        tok = strtok_r(NULL, " \t\r\n", &saveptr);
    }
}

static void log_status(const char *msg) {
    if (msg == NULL) {
        return;
    }
    fprintf(stderr, "init: %s\n", msg);
    int kmsg = open("/dev/kmsg", O_WRONLY);
    if (kmsg >= 0) {
        dprintf(kmsg, "<0>init: %s\n", msg);
        close(kmsg);
    }
}

int main(void) {
    log_status("start");
    ensure_dir("/proc");
    ensure_dir("/sys");
    ensure_dir("/dev");
    ensure_dir("/system");
    ensure_dir("/config");
    ensure_dir("/apps");
    ensure_dir("/home");
    ensure_dir("/var");
    ensure_dir("/run");
    ensure_dir("/usr");

    mount_or_die("proc", "/proc", "proc", 0, NULL, "mount /proc");
    log_status("mounted /proc");
    mount_or_die("sys", "/sys", "sysfs", 0, NULL, "mount /sys");
    log_status("mounted /sys");
    mount_or_die("dev", "/dev", "devtmpfs", 0, NULL, "mount /dev");
    attach_console_stdio();
    log_status("mounted /dev");

    char root_dev[128];
    read_root_device(root_dev, sizeof(root_dev));
    mount_or_die(root_dev, "/system", "erofs", MS_RDONLY, NULL, "mount /system");
    log_status("mounted /system");
    mount_or_die("/dev/sda5", "/config", "ext4", 0, NULL, "mount /config");
    log_status("mounted /config");
    mount_or_die("/dev/sda6", "/apps", "ext4", 0, NULL, "mount /apps");
    log_status("mounted /apps");
    mount_or_die("/dev/sda7", "/home", "btrfs", 0, NULL, "mount /home");
    log_status("mounted /home");

    /* Ensure /config/var exists — ext4 is empty on first boot */
    if (mkdir("/config/var", 0755) < 0 && errno != EEXIST) {
        die_errno("mkdir /config/var");
    }
    mount_or_die("/config/var", "/var", NULL, MS_BIND, NULL, "bind /var");
    log_status("bind-mounted /var");
    mount_or_die("tmpfs", "/run", "tmpfs", 0, "mode=755", "mount /run");
    log_status("mounted /run");
    mount_or_die("/system/usr", "/usr", NULL, MS_BIND, NULL, "bind /usr");
    log_status("bind-mounted /usr");

    mount_or_die("/dev", "/system/dev", NULL, MS_MOVE, NULL, "move /dev");
    mount_or_die("/proc", "/system/proc", NULL, MS_MOVE, NULL, "move /proc");
    mount_or_die("/sys", "/system/sys", NULL, MS_MOVE, NULL, "move /sys");
    mount_or_die("/run", "/system/run", NULL, MS_MOVE, NULL, "move /run");
    mount_or_die("/var", "/system/var", NULL, MS_MOVE, NULL, "move /var");
    mount_or_die("/config", "/system/config", NULL, MS_MOVE, NULL, "move /config");
    mount_or_die("/apps", "/system/apps", NULL, MS_MOVE, NULL, "move /apps");
    mount_or_die("/home", "/system/home", NULL, MS_MOVE, NULL, "move /home");
    mount_or_die("/usr", "/system/usr", NULL, MS_MOVE, NULL, "move /usr");

    log_status("chdir /system");
    if (chdir("/system") < 0) {
        die_errno("chdir /system");
    }
    log_status("move /system to /");
    mount_or_die(".", "/", NULL, MS_MOVE, NULL, "move /system to /");
    log_status("chroot /");
    if (chroot(".") < 0) {
        die_errno("chroot /");
    }
    if (chdir("/") < 0) {
        die_errno("chdir /");
    }

    char *const argv[] = {"/sbin/init", NULL};
    log_status("exec /sbin/init");
    execv(argv[0], argv);
    die_errno("exec /sbin/init");
    return 1;
}
