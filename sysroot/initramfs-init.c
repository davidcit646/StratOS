#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <stddef.h>
#include <string.h>
#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/ioctl.h>
#include <linux/loop.h>
#include <unistd.h>

/* Must match `xorriso -volid` in scripts/build-live-iso.sh (ISO9660 Primary Volume Descriptor). */
#define STRAT_LIVE_ISO_LABEL "STRATOS_LIVE"

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

static void read_cmdline_param(const char *param, char *out, size_t out_len, const char *fallback) {
    if (out == NULL || out_len == 0) {
        return;
    }

    snprintf(out, out_len, "%s", fallback);

    /* Require `key=` form so short prefixes cannot alias longer tokens (e.g. `ro` vs `root=`). */
    size_t plen = strlen(param);
    if (plen == 0 || param[plen - 1] != '=') {
        return;
    }

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
        if (strncmp(tok, param, strlen(param)) == 0 && tok[strlen(param)] != '\0') {
            snprintf(out, out_len, "%s", tok + strlen(param));
            return;
        }
        tok = strtok_r(NULL, " \t\r\n", &saveptr);
    }
}

static void read_root_device(char *out, size_t out_len) {
    /* Default when cmdline omits root= (legacy dev images); installed systems use PARTUUID from StratBoot. */
    read_cmdline_param("root=", out, out_len, "/dev/vda2");
}

static void read_config_device(char *out, size_t out_len) {
    read_cmdline_param("config=", out, out_len, "/dev/vda5");
}

static void read_apps_device(char *out, size_t out_len) {
    read_cmdline_param("apps=", out, out_len, "/dev/vda6");
}

static void read_home_device(char *out, size_t out_len) {
    read_cmdline_param("home=", out, out_len, "/dev/vda7");
}

static void read_live_erofs_path(char *out, size_t out_len) {
    read_cmdline_param("strat.live_erofs=", out, out_len, "slot-system.erofs");
}

static void read_live_iso_override_dev(char *out, size_t out_len) {
    read_cmdline_param("strat.live_iso_dev=", out, out_len, "");
}

/* Live tmpfs sizes (MiB). Defaults keep ~2.3G tmpfs caps; tune on low-RAM hosts via kernel cmdline. */
static unsigned read_cmdline_uint_mib(const char *param, unsigned fallback) {
    char val[32];
    read_cmdline_param(param, val, sizeof(val), "");
    if (val[0] == '\0') {
        return fallback;
    }
    char *end = NULL;
    unsigned long ul = strtoul(val, &end, 10);
    if (end == val || ul == 0 || ul > 131072UL) {
        return fallback;
    }
    return (unsigned)ul;
}

static int cmdline_is_live_iso(void) {
    int fd = open("/proc/cmdline", O_RDONLY);
    if (fd < 0) {
        return 0;
    }
    char buf[4096];
    ssize_t n = read(fd, buf, sizeof(buf) - 1);
    close(fd);
    if (n <= 0) {
        return 0;
    }
    buf[n] = '\0';
    return (strstr(buf, "strat.live=1") != NULL && strstr(buf, "strat.live_iso=1") != NULL);
}

static int try_mount_iso_volume(const char *dev) {
    if (access(dev, F_OK) != 0) {
        return -1;
    }
    if (mount(dev, "/live_iso", "iso9660", MS_RDONLY, NULL) == 0) {
        return 0;
    }
    if (mount(dev, "/live_iso", "udf", MS_RDONLY, NULL) == 0) {
        return 0;
    }
    return -1;
}

/* ECMA-119: Primary Volume Descriptor at logical sector 16 (2048-byte blocks). Volume ID bytes 40–71. */
static int iso9660_volume_id_matches(const char *dev, const char *expect) {
    int fd = open(dev, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return 0;
    }
    unsigned char sector[2048];
    ssize_t n = pread(fd, sector, sizeof(sector), (off_t)16 * 2048);
    close(fd);
    if (n != (ssize_t)sizeof(sector)) {
        return 0;
    }
    if (sector[0] != 0x01) {
        return 0;
    }
    if (memcmp(sector + 1, "CD001", 5) != 0) {
        return 0;
    }
    char vol[33];
    memcpy(vol, sector + 40, 32);
    vol[32] = '\0';
    size_t len = 32;
    while (len > 0 && vol[len - 1] == ' ') {
        vol[--len] = '\0';
    }
    return strcmp(vol, expect) == 0;
}

#define MAX_LIVE_ISO_CANDIDATES 96

static int block_name_skipped(const char *name) {
    if (name[0] == '.') {
        return 1;
    }
    if (strncmp(name, "loop", 4) == 0) {
        return 1;
    }
    if (strncmp(name, "ram", 3) == 0) {
        return 1;
    }
    if (strncmp(name, "dm-", 3) == 0) {
        return 1;
    }
    if (strncmp(name, "zram", 4) == 0) {
        return 1;
    }
    return 0;
}

static void live_iso_candidate_add(char paths[][256], size_t *n, const char *path) {
    size_t i;

    if (*n >= MAX_LIVE_ISO_CANDIDATES || path == NULL || path[0] == '\0') {
        return;
    }
    if (access(path, F_OK) != 0) {
        return;
    }
    for (i = 0; i < *n; i++) {
        if (strcmp(paths[i], path) == 0) {
            return;
        }
    }
    snprintf(paths[*n], 256, "%s", path);
    (*n)++;
}

static void live_iso_add_partition_variants(char paths[][256], size_t *n, const char *name) {
    char base[256];
    char extra[256];

    snprintf(base, sizeof(base), "/dev/%s", name);

    /* SD card: partition before whole block device. */
    if (strncmp(name, "mmcblk", 6) == 0 && strchr(name + 6, 'p') == NULL) {
        snprintf(extra, sizeof(extra), "/dev/%sp1", name);
        live_iso_candidate_add(paths, n, extra);
        live_iso_candidate_add(paths, n, base);
        return;
    }

    /* NVMe namespace (e.g. nvme0n1): first ISO partition, then whole namespace. */
    if (strncmp(name, "nvme", 4) == 0 && strchr(name, 'n') != NULL && strchr(name, 'p') == NULL) {
        snprintf(extra, sizeof(extra), "/dev/%sp1", name);
        live_iso_candidate_add(paths, n, extra);
        live_iso_candidate_add(paths, n, base);
        return;
    }

    /* Virtio / Xen SCSI: first partition (typical isohybrid USB), then whole disk. */
    if (strncmp(name, "sd", 2) == 0 && strlen(name) == 3 && name[2] >= 'a' && name[2] <= 'z') {
        snprintf(extra, sizeof(extra), "/dev/%s1", name);
        live_iso_candidate_add(paths, n, extra);
    }
    if (strncmp(name, "vd", 2) == 0 && strlen(name) == 3 && name[2] >= 'a' && name[2] <= 'z') {
        snprintf(extra, sizeof(extra), "/dev/%s1", name);
        live_iso_candidate_add(paths, n, extra);
    }
    if (strncmp(name, "xvd", 3) == 0 && strlen(name) == 4 && name[3] >= 'a' && name[3] <= 'z') {
        snprintf(extra, sizeof(extra), "/dev/%s1", name);
        live_iso_candidate_add(paths, n, extra);
    }
    live_iso_candidate_add(paths, n, base);
}

static int try_mount_live_iso_path(const char *path, int require_stratos_label) {
    if (require_stratos_label && !iso9660_volume_id_matches(path, STRAT_LIVE_ISO_LABEL)) {
        return -1;
    }
    return try_mount_iso_volume(path);
}

static int mount_live_iso_volume(void) {
    int i;
    char paths[MAX_LIVE_ISO_CANDIDATES][256];
    size_t np = 0;
    char ovr[256];
    DIR *bdir;
    struct dirent *ent;

    read_live_iso_override_dev(ovr, sizeof(ovr));
    if (ovr[0] != '\0') {
        if (try_mount_iso_volume(ovr) == 0) {
            return 0;
        }
    }

    /* Created by udev on some initramfs; often absent in devtmpfs-only images. */
    live_iso_candidate_add(paths, &np, "/dev/disk/by-label/" STRAT_LIVE_ISO_LABEL);

    /* Optical: match volume id first (internal / external drives). */
    for (i = 0; i < 32; i++) {
        char path[32];
        snprintf(path, sizeof(path), "/dev/sr%d", i);
        live_iso_candidate_add(paths, &np, path);
    }

    bdir = opendir("/sys/block");
    if (bdir != NULL) {
        while ((ent = readdir(bdir)) != NULL) {
            if (block_name_skipped(ent->d_name)) {
                continue;
            }
            /* sr* handled above in stable order. */
            if (strncmp(ent->d_name, "sr", 2) == 0) {
                continue;
            }
            live_iso_add_partition_variants(paths, &np, ent->d_name);
        }
        closedir(bdir);
    }

    for (i = 0; (size_t)i < np; i++) {
        if (try_mount_live_iso_path(paths[i], 1) == 0) {
            return 0;
        }
    }

    /* Unlabeled optical fallback (dev ISOs without matching PVD id). */
    for (i = 0; i < 32; i++) {
        char path[32];
        snprintf(path, sizeof(path), "/dev/sr%d", i);
        if (access(path, F_OK) != 0) {
            continue;
        }
        if (try_mount_iso_volume(path) == 0) {
            return 0;
        }
    }

    /* Last resort: any candidate that mounts as iso9660/udf even if PVD id mismatches
     * (guards against odd isohybrid layouts); skipped when empty. */
    for (i = 0; (size_t)i < np; i++) {
        if (strncmp(paths[i], "/dev/sr", 7) == 0) {
            continue;
        }
        if (try_mount_iso_volume(paths[i]) == 0) {
            return 0;
        }
    }
    return -1;
}

static int mount_erofs_file_via_loop(const char *erofs_file) {
    int backing = open(erofs_file, O_RDONLY);
    if (backing < 0) {
        fprintf(stderr, "init: open erofs file %s: %s\n", erofs_file, strerror(errno));
        return -1;
    }

    int ctl = open("/dev/loop-control", O_RDWR | O_CLOEXEC);
    if (ctl < 0) {
        fprintf(stderr, "init: open /dev/loop-control: %s\n", strerror(errno));
        close(backing);
        return -1;
    }

    int dev_num = ioctl(ctl, LOOP_CTL_GET_FREE);
    close(ctl);
    if (dev_num < 0) {
        fprintf(stderr, "init: LOOP_CTL_GET_FREE failed: %s\n", strerror(errno));
        close(backing);
        return -1;
    }

    char loop_path[64];
    snprintf(loop_path, sizeof(loop_path), "/dev/loop%d", dev_num);

    int loop_fd = open(loop_path, O_RDWR | O_CLOEXEC);
    if (loop_fd < 0) {
        fprintf(stderr, "init: open %s: %s\n", loop_path, strerror(errno));
        close(backing);
        return -1;
    }

    if (ioctl(loop_fd, LOOP_SET_FD, (unsigned long)backing) != 0) {
        fprintf(stderr, "init: LOOP_SET_FD: %s\n", strerror(errno));
        close(loop_fd);
        close(backing);
        return -1;
    }

    close(backing);

    if (mount(loop_path, "/system", "erofs", MS_RDONLY, NULL) != 0) {
        fprintf(stderr, "init: mount erofs from %s: %s\n", loop_path, strerror(errno));
        (void)ioctl(loop_fd, LOOP_CLR_FD, 0);
        close(loop_fd);
        return -1;
    }
    close(loop_fd);
    return 0;
}

static void log_status(const char *msg) {
    if (msg == NULL) {
        return;
    }
    int kmsg = open("/dev/kmsg", O_WRONLY);
    if (kmsg >= 0) {
        dprintf(kmsg, "<0>init: %s\n", msg);
        close(kmsg);
    }
}

static int equals_ignore_case(const char *a, const char *b) {
    if (a == NULL || b == NULL) {
        return 0;
    }
    while (*a != '\0' && *b != '\0') {
        char ca = *a;
        char cb = *b;
        if (ca >= 'a' && ca <= 'z') {
            ca = (char)(ca - ('a' - 'A'));
        }
        if (cb >= 'a' && cb <= 'z') {
            cb = (char)(cb - ('a' - 'A'));
        }
        if (ca != cb) {
            return 0;
        }
        a++;
        b++;
    }
    return (*a == '\0' && *b == '\0');
}

static int resolve_partuuid_device(const char *partuuid_spec, char *out, size_t out_len) {
    if (partuuid_spec == NULL || out == NULL || out_len == 0) {
        return -1;
    }
    const char *prefix = "PARTUUID=";
    size_t prefix_len = strlen(prefix);
    if (strncmp(partuuid_spec, prefix, prefix_len) != 0) {
        snprintf(out, out_len, "%s", partuuid_spec);
        return 0;
    }

    const char *target = partuuid_spec + prefix_len;
    DIR *dir = opendir("/sys/class/block");
    if (dir == NULL) {
        return -1;
    }

    struct dirent *entry = NULL;
    char path[256];
    char line[256];
    char value[128];
    int found = 0;

    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') {
            continue;
        }

        snprintf(path, sizeof(path), "/sys/class/block/%s/uevent", entry->d_name);
        FILE *f = fopen(path, "r");
        if (f == NULL) {
            continue;
        }

        while (fgets(line, sizeof(line), f) != NULL) {
            if (strncmp(line, "PARTUUID=", 9) != 0) {
                continue;
            }
            snprintf(value, sizeof(value), "%s", line + 9);
            size_t len = strlen(value);
            while (len > 0 && (value[len - 1] == '\n' || value[len - 1] == '\r')) {
                value[len - 1] = '\0';
                len--;
            }
            if (equals_ignore_case(value, target)) {
                snprintf(out, out_len, "/dev/%s", entry->d_name);
                found = 1;
                break;
            }
        }

        fclose(f);
        if (found) {
            break;
        }
    }

    closedir(dir);
    return found ? 0 : -1;
}

static void resolve_or_fallback_data_dev(
    const char *raw,
    const char *fallback_vda,
    const char *fallback_sda,
    char *out,
    size_t out_len,
    const char *what
) {
    if (resolve_partuuid_device(raw, out, out_len) == 0) {
        return;
    }
    if (access(fallback_vda, F_OK) == 0) {
        snprintf(out, out_len, "%s", fallback_vda);
        fprintf(stderr, "init: %s: using fallback %s\n", what, fallback_vda);
        return;
    }
    if (access(fallback_sda, F_OK) == 0) {
        snprintf(out, out_len, "%s", fallback_sda);
        fprintf(stderr, "init: %s: using fallback %s\n", what, fallback_sda);
        return;
    }
    fprintf(stderr, "init: could not resolve %s (%s)\n", what, raw);
    wait_forever();
}

static void pivot_into_stratman(void) {
    /* Ensure /config/etc exists — persistent fs is empty on first boot; tmpfs is empty every boot */
    if (mkdir("/config/etc", 0755) < 0 && errno != EEXIST) {
        die_errno("mkdir /config/etc");
    }
    mount_or_die("/config/etc", "/etc", NULL, MS_BIND, NULL, "bind /etc");
    log_status("bind-mounted /etc");

    /* Ensure /config/var exists — same */
    if (mkdir("/config/var", 0755) < 0 && errno != EEXIST) {
        die_errno("mkdir /config/var");
    }
    mount_or_die("/config/var", "/var", NULL, MS_BIND, NULL, "bind /var");
    log_status("bind-mounted /var");
    mount_or_die("tmpfs", "/run", "tmpfs", 0, "mode=755", "mount /run");
    log_status("mounted /run");
    mount_or_die("/system", "/usr", NULL, MS_BIND, NULL, "bind /usr");
    log_status("bind-mounted /usr");

    mount_or_die("/dev", "/system/dev", NULL, MS_MOVE, NULL, "move /dev");
    mount_or_die("/proc", "/system/proc", NULL, MS_MOVE, NULL, "move /proc");
    mount_or_die("/sys", "/system/sys", NULL, MS_MOVE, NULL, "move /sys");
    mount_or_die("/run", "/system/run", NULL, MS_MOVE, NULL, "move /run");
    mount_or_die("/var", "/system/var", NULL, MS_MOVE, NULL, "move /var");
    mount_or_die("/config", "/system/config", NULL, MS_MOVE, NULL, "move /config");
    mount_or_die("/apps", "/system/apps", NULL, MS_MOVE, NULL, "move /apps");
    mount_or_die("/home", "/system/home", NULL, MS_MOVE, NULL, "move /home");
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

    char *const argv[] = {"/bin/stratman", NULL};
    log_status("exec /bin/stratman");
    fprintf(stderr, "init: starting /bin/stratman\n");
    (void)fflush(stderr);
    execv(argv[0], argv);
    die_errno("exec /bin/stratman");
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
    ensure_dir("/etc");
    ensure_dir("/live_iso");

    mount_or_die("proc", "/proc", "proc", 0, NULL, "mount /proc");
    log_status("mounted /proc");
    mount_or_die("sys", "/sys", "sysfs", 0, NULL, "mount /sys");
    log_status("mounted /sys");
    mount_or_die("dev", "/dev", "devtmpfs", 0, NULL, "mount /dev");
    attach_console_stdio();
    log_status("mounted /dev");

    if (cmdline_is_live_iso()) {
        if (mount_live_iso_volume() != 0) {
            fprintf(
                stderr,
                "init: live ISO: could not mount iso9660/udf (check USB path vs CD-ROM, or "
                "strat.live_iso_dev=); expected volume id " STRAT_LIVE_ISO_LABEL "\n"
            );
            wait_forever();
        }
        log_status("mounted live ISO volume on /live_iso");

        char rel[256];
        char full[512];
        read_live_erofs_path(rel, sizeof(rel));
        if (rel[0] == '/') {
            snprintf(full, sizeof(full), "/live_iso%s", rel);
        } else {
            snprintf(full, sizeof(full), "/live_iso/%s", rel);
        }

        if (mount_erofs_file_via_loop(full) != 0) {
            fprintf(stderr, "init: live: could not mount EROFS at %s\n", full);
            wait_forever();
        }
        log_status("mounted /system (live EROFS)");

        {
            unsigned cfg_mb = read_cmdline_uint_mib("strat.live_config_mb=", 512);
            unsigned apps_mb = read_cmdline_uint_mib("strat.live_apps_mb=", 768);
            unsigned home_mb = read_cmdline_uint_mib("strat.live_home_mb=", 1024);
            char data[96];
            snprintf(data, sizeof(data), "mode=755,size=%uM", cfg_mb);
            mount_or_die("tmpfs", "/config", "tmpfs", 0, data, "tmpfs /config");
            log_status("mounted tmpfs /config (live)");
            snprintf(data, sizeof(data), "mode=755,size=%uM", apps_mb);
            mount_or_die("tmpfs", "/apps", "tmpfs", 0, data, "tmpfs /apps");
            log_status("mounted tmpfs /apps (live)");
            snprintf(data, sizeof(data), "mode=755,size=%uM", home_mb);
            mount_or_die("tmpfs", "/home", "tmpfs", 0, data, "tmpfs /home");
            log_status("mounted tmpfs /home (live)");
        }

        pivot_into_stratman();
    }

    {
        char root_dev[128];
        char root_dev_resolved[128];
        read_root_device(root_dev, sizeof(root_dev));
        if (resolve_partuuid_device(root_dev, root_dev_resolved, sizeof(root_dev_resolved)) != 0) {
            if (strncmp(root_dev, "PARTUUID=", 9) == 0) {
                if (access("/dev/vda2", F_OK) == 0) {
                    snprintf(root_dev_resolved, sizeof(root_dev_resolved), "/dev/vda2");
                } else if (access("/dev/sda2", F_OK) == 0) {
                    snprintf(root_dev_resolved, sizeof(root_dev_resolved), "/dev/sda2");
                } else {
                    fprintf(stderr, "init: resolve root device failed: %s\n", root_dev);
                    wait_forever();
                }
                fprintf(stderr, "init: PARTUUID unresolved, falling back to %s\n", root_dev_resolved);
            } else {
                snprintf(root_dev_resolved, sizeof(root_dev_resolved), "%s", root_dev);
            }
        }
        mount_or_die(root_dev_resolved, "/system", "erofs", MS_RDONLY, NULL, "mount /system");
        log_status("mounted /system");

        char config_dev[128];
        char config_resolved[128];
        read_config_device(config_dev, sizeof(config_dev));
        resolve_or_fallback_data_dev(
            config_dev, "/dev/vda5", "/dev/sda5", config_resolved, sizeof(config_resolved), "config"
        );
        mount_or_die(config_resolved, "/config", "ext4", 0, NULL, "mount /config");
        log_status("mounted /config");

        char apps_dev[128];
        char apps_resolved[128];
        read_apps_device(apps_dev, sizeof(apps_dev));
        resolve_or_fallback_data_dev(
            apps_dev, "/dev/vda6", "/dev/sda6", apps_resolved, sizeof(apps_resolved), "apps"
        );
        mount_or_die(apps_resolved, "/apps", "ext4", 0, NULL, "mount /apps");
        log_status("mounted /apps");

        char home_dev[128];
        char home_resolved[128];
        read_home_device(home_dev, sizeof(home_dev));
        resolve_or_fallback_data_dev(
            home_dev, "/dev/vda7", "/dev/sda7", home_resolved, sizeof(home_resolved), "home"
        );
        mount_or_die(home_resolved, "/home", "btrfs", 0, NULL, "mount /home");
        log_status("mounted /home");

        pivot_into_stratman();
    }

    return 1;
}
