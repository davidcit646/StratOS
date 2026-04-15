# TALKING-3.md

StratOS Project Status — 2026-04-14

---

## 🎉 MILESTONE: FULL BOOT TO GUI ACHIEVED

**StratOS successfully boots from UEFI to graphical terminal:**

```
UEFI → StratBoot → Linux 6.6.30 → Initramfs → EROFS → stratwm → foot terminal
```

### Current Working Stack

| Phase | Component | Status |
|-------|-------------|--------|
| 3 | **StratBoot** (UEFI bootloader) | ✅ Working — EFI var init, slot selection, kernel handoff |
| 4 | **Linux Kernel** 6.6.30 | ✅ Working — DRM/VirtIO GPU, all filesystems |
| 5 | **Boot Validation** | ✅ Service implemented (strat-validate-boot) |
| 6 | **Supervisor** (stratsup) | ✅ EFI vars, pivot, rollback, boot counter |
| 7 | **Initramfs + EROFS** | ✅ Working — static C init, pivot root, stratwm |
| 8 | **Window Manager** (stratwm) | ✅ **FIXED** — now renders windows properly |
| 9 | **Terminal** (foot) | ✅ Working — graphical terminal active |

---

## What Was Fixed

### Critical Bug: stratwm Not Rendering (2026-04-14)
**Problem:** stratwm created Wayland surfaces but windows were invisible. Wayland protocol was active but nothing rendered to screen.

**Root Cause:** Missing `wlr_output_schedule_frame()` call when new windows mapped. The scene was updated but output didn't repaint.

**Fix:** Added in `stratvm/src/main.c:view_map_notify()`:
```c
/* Damage all outputs to force repaint of new window */
struct stratwm_output *output;
wl_list_for_each(output, &server->outputs, link) {
    wlr_output_schedule_frame(output->wlr_output);
}
```

**Result:** Terminal window now appears with cyan border and titlebar.

---

## Project Architecture

### Boot Flow
1. **StratBoot** (C + GNU-EFI) — UEFI bootloader
   - `stratboot/src/stratboot.c` — main EFI entry
   - `stratboot/src/slot.c` — slot selection logic
   - `stratboot/src/reset.c` — reset/wipe operations
   - `stratboot/src/partition.c` — GPT partition operations

2. **Kernel** — Linux 6.6.30 LTS
   - Config: `stratos-kernel/stratos.config`
   - Build script: `scripts/phase4/build-kernel.sh`

3. **Initramfs** — static C init
   - Source: `sysroot/initramfs-init.c`
   - Mounts filesystems, pivots to EROFS

4. **Root Filesystem** (EROFS read-only)
   - Built by: `scripts/phase7/build-slot-erofs.sh`
   - Contains: stratwm, foot, libraries, system-init

5. **Window Manager** — stratwm (wlroots-based)
   - Source: `stratvm/src/main.c`
   - Wayland compositor with tiling, floating, titlebars

6. **Terminal** — foot (Wayland terminal)
   - Spawned by stratwm on startup

---

## Key Files

| Path | Purpose |
|------|---------|
| `out/phase3/BOOTX64.EFI` | UEFI bootloader binary |
| `out/phase4/vmlinuz` | Linux kernel |
| `out/phase7/initramfs.cpio.gz` | Initramfs image |
| `out/phase7/slot-system.erofs` | Root filesystem |
| `out/phase4/test-disk.vhd` | Full bootable disk image |
| `stratboot/src/stratboot.c` | Bootloader main |
| `stratvm/src/main.c` | Window manager |
| `sysroot/system-init.c` | PID 1 init (in EROFS) |

---

## Remaining Work

### Phase 3 (Bootloader) — Complete
- ✅ EFI variable initialization on first boot
- ✅ Slot selection (CONFIRMED/STAGING/PINNED)
- ✅ Home corruption detection screen
- ✅ Reset execution (CONFIG/HOME/System wipe)
- ✅ Kernel handoff

### Phase 4-7 — Complete
- ✅ Kernel build with GCC 15 compatibility
- ✅ Initramfs (static C, no busybox dependency)
- ✅ EROFS root filesystem
- ✅ Test disk creation

### Phase 8 (Window Manager) — Functional
- ✅ Basic tiling/floating windows
- ✅ Titlebars with buttons
- ✅ Window borders (focused/unfocused)
- ⚠️ **Phase 8.5+:** Multi-workspace switching, layout modes, animations

### Phase 9 (Strat Terminal) — Partial
- ✅ stratterm codebase (GTK4 + VTE)
- ✅ stratterm-indexer daemon
- ✅ strat-settings app
- ❌ **Not in EROFS yet** — needs build + staging

### Phase 10-14 — Not Started
- Session management
- App lifecycle
- Audio stack
- Networking

### Phase 15 (Polish) — Deferred
- Smooth boot animations
- GUI boot menu
- Advanced recovery tools

---

## Build Commands

```bash
# Full rebuild from scratch
cd /home/dcitarelli/StratOS
make clean all              # Build bootloader
scripts/phase4/build-kernel.sh --jobs 4
scripts/phase7/build-initramfs.sh --init-mode static
scripts/phase7/prepare-minimal-rootfs.sh
scripts/phase7/build-slot-erofs.sh --rootfs out/phase7/rootfs-minimal --output out/phase7/slot-system.erofs
scripts/phase4/create-test-disk.sh
qemu-img convert -f raw -O vpc out/phase4/test-disk.img out/phase4/test-disk.vhd

# Test boot
qemu-system-x86_64 \
  -m 1024 \
  -bios /usr/share/edk2/ovmf/OVMF_CODE.fd \
  -drive file=out/phase4/test-disk.vhd,format=vpc,if=ide \
  -vga std
```

---

## Next Steps (Priority Order)

1. **Add stratterm to EROFS** — build and stage the GTK terminal
2. **Multi-workspace support** — workspace switching in stratwm
3. **strat-settings integration** — settings app in rootfs
4. **Session persistence** — save/restore window state

---

## Handoff Protocol

- **This file (TALKING-3.md)** is the current active handoff log
- Previous logs: TALKING.md (Phase 1-8), TALKING-2.md (Phase 9 stratterm)
- Append new entries at bottom with date
- Include: what changed, file paths, build/test status

---

*Last updated: 2026-04-14 — Full GUI boot achieved*
