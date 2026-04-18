# /etc on CONFIG: Honest Filesystem Pattern

**This document defines the architecture for mutable system configuration in StratOS.**

## Problem

Traditional Linux systems use `/etc` for mutable system configuration (network, DNS, users, services). However, StratOS's Honest Filesystem Model has:

- `/system` (SLOT_A): Immutable EROFS — cannot write
- `/` (root): tmpfs or initramfs — ephemeral, lost on reboot
- `/config`: ext4 on CONFIG partition — persistent, survives reboots

Apps and libraries (libc, curl, OpenSSL) hardcode `/etc/resolv.conf` for DNS and expect `/etc` to exist.

## Solution

Create `/config/etc/` as the **persistent backing store** and bind mount it to `/etc` at boot time.

```
Persistent storage:  /config/etc/resolv.conf
                     /config/etc/hostname
                     /config/etc/hosts
                     
Runtime view:        /etc/resolv.conf  (bind mount)
                     /etc/hostname     (bind mount)
                     /etc/hosts        (bind mount)
```

## Architecture

### Boot-Time Setup (initramfs-init.c)

```c
// After mounting CONFIG partition to /config
mkdir("/config/etc", 0755);           // Ensure backing directory exists
mkdir("/etc", 0755);                  // Create mount point
mount("/config/etc", "/etc", "", MS_BIND, NULL);  // Bind mount
```

### Runtime Behavior

| Action | Path Used | Actual Location | Persistence |
|--------|-----------|-------------------|-------------|
| Read DNS | `/etc/resolv.conf` | `/config/etc/resolv.conf` | ✅ Survives reboot |
| Write DNS | `/etc/resolv.conf` | `/config/etc/resolv.conf` | ✅ Survives reboot |
| Read hostname | `/etc/hostname` | `/config/etc/hostname` | ✅ Survives reboot |

### Applications

Apps use standard paths (`/etc/resolv.conf`) and work without modification. The bind mount is transparent.

**StratOS-native apps** should prefer direct `/config/etc/` paths when they need persistence guarantees.

## Stratman Integration

### Initialization

stratman's `mount_filesystems()` creates the bind mount after CONFIG partition is available:

```rust
unsafe fn mount_filesystems() {
    // ... existing mounts ...
    
    // Ensure /config/etc exists (create on first boot)
    ensure_dir(b"/config/etc\0");
    
    // Create /etc mount point
    ensure_dir(b"/etc\0");
    
    // Bind mount /config/etc to /etc
    mount_best_effort(b"/config/etc\0", b"/etc\0", b"", libc::MS_BIND, core::ptr::null());
}
```

### Network Manager

strat-network writes DNS configuration directly to `/etc/resolv.conf` (which writes through to `/config/etc/resolv.conf`):

```rust
fn write_dns_config(dns_servers: &[[u8; 4]]) -> Result<(), String> {
    // /etc is bind-mounted to /config/etc
    // This persists across reboots
    let mut file = std::fs::File::create("/etc/resolv.conf")?;
    // ... write DNS servers ...
}
```

## Filesystem Contract

This pattern follows the **Runtime Persistence Contract** (`runtime-persistence-contract.md`):

- **Mutable data** lives under `/config`
- **Ephemeral runtime view** at standard paths (`/etc`)
- **Transparent to applications** via bind mount
- **Honest filesystem** — no symlinks, no overlayfs, no union mounts

## Comparison to /var Pattern

| | `/var` | `/etc` |
|---|---|---|
| **Backing store** | `/config/var` | `/config/etc` |
| **Runtime mount** | bind to `/var` | bind to `/etc` |
| **Purpose** | Variable data (logs, caches) | System config (network, DNS) |
| **Created by** | initramfs-init.c | initramfs-init.c + stratman |

## Implementation Checklist

- [x] CONFIG partition mounted at `/config` (ext4, persistent)
- [x] `/config/var` → `/var` bind mount (initramfs-init.c)
- [ ] `/config/etc` → `/etc` bind mount (initramfs-init.c + stratman)
- [ ] DNS writes to `/etc/resolv.conf` (strat-network)
- [ ] Service manifests in `/config/etc/stratman/services/` (override support)

## Security Considerations

- `/config` is user-writable — any app can modify `/config/etc/resolv.conf`
- DNS hijacking possible if untrusted apps run
- Mitigation: Service sandboxing (future), read-only bind mounts for specific files

## References

- `docs/runtime-persistence-contract.md` — Path ownership and persistence rules
- `docs/application-config-resolution.md` — App config lookup patterns
- `sysroot/initramfs-init.c` — Boot-time mount operations
- `stratman/src/main.rs` — Service manager initialization

## Version History

- 2026-04-17: Initial documentation, defined /etc on CONFIG pattern
