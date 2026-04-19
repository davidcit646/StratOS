# Live bootable ISO (Milestone A)

StratOS can boot from a **hybrid UEFI ISO** that carries the same stack as an installed disk: **StratBoot** → **kernel + initramfs** → **EROFS root** → **stratman** → **stratwm / stratterm**.

This is **not** a generic rescue image: it reuses `out/phase7` artifacts from `./build-all-and-run.sh`.

---

## Architecture (live vs installed)

| Area | Installed disk (`scripts/create-test-disk.sh`) | Live ISO (this pipeline) |
| ---- | ---------------------------------------------- | ------------------------- |
| `/system` | EROFS on GPT `SLOT_A` | Same **EROFS image**, stored as `slot-system.erofs` on the **ISO9660** volume; initramfs **loop-mounts** it (read-only). |
| `/config`, `/apps`, `/home` | ext4 / ext4 / btrfs per GPT names | **tmpfs** for the session (no persistent layout on the medium). `/etc` and `/var` still follow the bind rules from `/config`. |
| StratBoot | Reads GPT PARTUUIDs, passes `root=` + `config=` + `apps=` + `home=` | If `EFI\\STRAT\\LIVE` exists on the FAT volume StratBoot booted from, StratBoot skips GPT data partitions and passes `strat.live=1 strat.live_iso=1` (kernel + initrd paths unchanged). |
| Honesty | Immutable slot semantics unchanged | Live does **not** make the slot image writable; tmpfs holds only CONFIG/APPS/HOME **semantics**, not a mutable EROFS slot. |

UI-led installer flow (menus, diagnostics, preserve-CONFIG options) in **stratos-design.md** section **17.2–17.8** is still ahead. A **command-line fresh install** is available from the live session (`strat-installer`).

---

## Milestone B — Install to a real disk (fresh wipe)

`bin/strat-installer` is copied into the phase7 rootfs (and therefore into `slot-system.erofs` and the ISO). It expects the **same payload files** the ISO places on the ISO9660 root (not only inside the embedded El Torito FAT):

| File | Role |
| ---- | ---- |
| `slot-system.erofs` | Raw copy to GPT `SLOT_A` |
| `vmlinuz.efi` | Kernel on ESP `EFI/STRAT/SLOT_A/` |
| `initramfs.img` | Initrd beside kernel (same bytes as `out/phase7/initramfs.cpio.gz`; filename is convention, not a different format) |
| `BOOTX64.EFI` | `EFI/BOOT/BOOTX64.EFI` |

`scripts/build-live-iso.sh` stages all four next to `slot-system.erofs` on the ISO9660 volume. The hybrid ISO volume identifier is **`STRATOS_LIVE`** (`xorriso -volid`), which **initramfs** and **`strat-installer`** use first when locating the medium among multiple optical devices (see below).

**Steps (from live session, as root):**

1. Identify the **internal** target disk (e.g. `/dev/nvme0n1` or `/dev/sda`). Do **not** choose the USB stick running the live image.
2. Optional: `lsblk -o NAME,SIZE,MODEL,MOUNTPOINTS`
3. Run:

   ```bash
   sudo strat-installer --disk /dev/nvme0n1
   ```

4. Type the exact confirmation phrase when prompted (full disk wipe).

The script creates the same **GPT layout** as `scripts/create-test-disk.sh`, sizes `SLOT_A` from the EROFS file (+ margin), gives **HOME** the remainder of the disk, formats partitions, `dd`s the system image, and copies StratBoot/kernel/initrd to the ESP **without** the live `EFI/STRAT/LIVE` marker (normal installed boot path). **StratBoot** initializes EFI variables on first boot when NVRAM is empty (`strat_maybe_init_vars`).

**Override payload path:** `--source-dir /path` if payloads are not on an auto-mounted optical device (e.g. NFS).

**Multiple optical drives:** Enumeration order of `/dev/sr*` is not guaranteed. **Label-first discovery** uses ISO9660 volume id **`STRATOS_LIVE`** (installer: `blkid -t LABEL=STRATOS_LIVE` when available; initramfs: Primary Volume Descriptor match before trying mounts). Fallback tries `/dev/sr0` … `/dev/sr31` if needed.

**Live-medium guard:** The installer refuses `--disk` if it resolves to the **same whole-disk device** as the block device backing your `--source-dir` (or the auto-mounted ISO on `/dev/sr0` / `/dev/sr1`). That blocks the common mistake of wiping the USB stick or optical drive you booted from. If you truly need to target that device, pass **`--allow-wipe-live-medium`** (dangerous).

**Dependencies:** `sgdisk`, `partprobe`, `mkfs.vfat`, `mkfs.ext4`, `mkfs.btrfs` — `build-all-and-run.sh` copies these from the **build host** into the rootfs for the installer; rebuild the system image after changing the script.

---

## Build

**Host tools:** `mkfs.vfat`, **mtools** (`mmd`, `mcopy`), **xorriso**, plus everything needed for `./build-all-and-run.sh`.

1. Produce artifacts (kernel, stratboot, initramfs, `slot-system.erofs`):

   ```bash
   ./build-all-and-run.sh -s
   ```

   Stop the script once artifacts are produced if you do not need a refreshed `test-disk.img`.

2. Build the ISO:

   ```bash
   ./scripts/build-live-iso.sh
   ```

**Output:** `out/live/stratos-live.iso` (typically hundreds of MB, dominated by `slot-system.erofs`).

Optional kernel cmdline knobs parsed in initramfs:

- `strat.live_erofs=` — path to the EROFS file **on the mounted ISO** (default: `slot-system.erofs` at the ISO root; can be `dir/file.erofs`).
- `strat.live_iso_dev=` — optional **explicit block device** for the ISO9660 layer (e.g. `/dev/sdc1` or `/dev/nvme0n1p1` when the hybrid image is on a USB stick). If omitted, initramfs tries this override first, then optical **`sr*`**, **`/dev/disk/by-label/STRATOS_LIVE`** when present, then scans **`/sys/block`** ( **`sd`/`vd`/`xvd`**: try `…1` before whole disk; **`nvme`/`mmcblk`**: `…p1` before namespace/card) and mounts the device whose **ISO9660 Primary Volume Descriptor** volume id is **STRATOS_LIVE** (same as `xorriso -volid` in `scripts/build-live-iso.sh`), then unlabeled optical, then a last-resort iso9660 mount without label match.
- `strat.live_config_mb=`, `strat.live_apps_mb=`, `strat.live_home_mb=` — tmpfs size caps for the live session (MiB); see **Live session RAM and tmpfs** below.

---

## Boot on hardware

Write **`out/live/stratos-live.iso`** to USB (see **Physical USB** below) or use firmware boot-from-optical if your machine exposes the medium that way. Bring-up is validated on **real UEFI hardware** (GPU, input, and firmware vary by machine).

---

## Secure Boot

The in-tree pipeline ships **unsigned** `BOOTX64.EFI` and an unsigned kernel. That is **not** a compile-time problem, but on many PCs it is a **boot-time** problem: firmware with **Secure Boot** enabled will refuse the chain until binaries are signed and loaded via **shim** (or similar). **For testing:** disable Secure Boot in firmware, or use hardware without it. **Before shipping to end users:** keep SB-off documented for StratOS live/install, or invest in a signing + shim pipeline (large effort). This repo does not provide signed artifacts yet.

---

## Live session RAM and tmpfs

The live path mounts **tmpfs** for `/config`, `/apps`, and `/home` with default **upper bounds** of roughly **512 MiB + 768 MiB + 1024 MiB** (see `sysroot/initramfs-init.c`). On **2–4 GiB** RAM hosts, a full desktop (compositor, panel, heavy apps) can **OOM** or thrash even when tmpfs caps are not fully used.

**Guidance:**

- Treat **4 GiB system RAM** as a practical **minimum** for a comfortable live desktop session; **8 GiB** is safer for browser-class workloads.
- Tune per-mount caps on the **kernel cmdline** (parsed in initramfs), without changing the image:

  - `strat.live_config_mb=` — `/config` tmpfs size in MiB (default **512**).
  - `strat.live_apps_mb=` — `/apps` (default **768**).
  - `strat.live_home_mb=` — `/home` (default **1024**).

Example (smaller caps on a 2 GiB machine — experiment; values that are too small break sessions):

```text
strat.live_config_mb=256 strat.live_apps_mb=256 strat.live_home_mb=384
```

---

## Physical USB

Write the hybrid ISO with `dd` or a graphical imager (payload is an ISO9660 + embedded FAT El Torito image; many tools can flash it).

```bash
sudo dd if=out/live/stratos-live.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

Replace `sdX` with your USB device and **double-check** the target. Unplug internal disks from the selector if your firmware allows mistaken choices.

**Ventoy / chain loaders:** **Unsupported** in-tree until explicitly tested and documented in a small compatibility matrix here. StratBoot + El Torito + hybrid MBR are intended to be firmware-friendly when the firmware boots the ISO directly; Ventoy and similar tools inject their own boot path and may not preserve StratBoot’s assumptions.

---

## Desktop UX (live session)

The live image runs the same **stratpanel** + **stratvm** stack as the dev disk. **Known limitation:** focused **XDG** windows can render **above** the layer-shell **panel** (scene-tree / Z-order), so the top bar may be visually covered until you focus the desktop or move the window. That is a **quality** issue for a “desktop” live ISO, not a boot failure; fixing it is compositor/panel layering work (see `docs/human/stratpanel.md` and Phase 24/25 notes in the coding checklist).

---

## Why not mkosi-only (yet)?

**In-tree today:** `scripts/build-live-iso.sh` — **xorriso** + **mtools** + embedded FAT ESP image (see script header for exact inputs from `out/phase3`, `out/phase4`, `out/phase7`).

**Design doc** section **17.9** still describes **mkosi** as the long-term packaging target (smaller image, reproducible profiles). A mkosi profile that consumes the same `out/` artifacts can replace the shell glue later without changing the StratBoot/initramfs contract.

---

## Related

- Agent task prompt: `docs/agent/prompts/live-iso.md`
- Coding checklist: Phase 22 (ISO pipeline) in `docs/human/coding-checklist.md`
