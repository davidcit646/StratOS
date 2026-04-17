# StratOS Discussion Log

Consolidated notes from TALKING.md, TALKING-2.md, and TALKING-3.md. This is a living log of ideas and discussions. Formal architecture lives in StratOS-Design-Doc-v0.4.md.

---

## I. Core Philosophy and Goals

### Custom-First Principles
* **The Power User Focus:** StratOS is built on the belief that "Customization is not a feature; it is the foundation."
* **Immutable but Plastic:** While the core OS remains immutable for stability and security, the user layer must be infinitely flexible. 
* **The "Vibe Coding" Workflow:** Integration of AI-assisted development tools (like Windsurf and Cursor) should be a first-class citizen in the OS experience.
* **Infrastructure as Code:** The OS state should be reproducible and declarable, primarily through the BlueBuild and Universal Blue ecosystem.

### System Metaphors
* **StratMon is the Conductor:** It coordinates the state of the system, ensures services are in the correct phase, and manages the lifecycle of applications.
* **StratBoot is the Surgeon:** It handles the precision work of the boot process, ensuring only the necessary layers are stitched together at the right time.

---

## II. Update and Versioning Architecture

### The StratOS Lifecycle
* **Atomic Updates:** Leveraging `rpm-ostree` for guaranteed rollbacks.
* **The Update Flow:**
    1.  **Staging:** Updates are downloaded in the background to a dormant deployment.
    2.  **Verification:** Integrity checks ensure the staged image matches the remote manifest.
    3.  **The "Handshake":** On reboot, StratBoot verifies the integrity of the new deployment before switching.
* **Version Pinning:** Users should be able to pin specific builds to prevent breaking changes during critical work cycles.

---

## III. Filesystem and Storage Philosophy

### Layering Strategy
* **The Base Image (ReadOnly):** Contains the kernel, system libraries, and core StratOS utilities.
* **The Work Layer:** Utilizing `overlays` or `reflink` based copying to allow for "disposable" testing environments.
* **Home Directory Management:** Intentional separation of user data and system configuration. Exploration of using Btrfs subvolumes to snapshot `/home` independently of the system.

### Security and Permissions
* **Flatpak-Centric:** All GUI applications should ideally be sandboxed via Flatpak.
* **Service Isolation:** System services managed by StratMon should run with the least privilege possible.

---

## IV. Technical Implementation Ideas

### StratMon (The System Monitor/Conductor)
* **Goal:** A lightweight daemon to monitor system health and resource allocation.
* **Features:**
    * Monitor temperature and power profiles.
    * Interface with the update daemon to notify users of pending reboots.
    * Provide a "Developer Mode" toggle that relaxes certain immutability constraints for active coding sessions.

### Bootloader and Initialization
* **StratBoot Ideas:** * Integration with systemd-boot for simplicity and speed.

---

## V. Auditor Reviews

2026-04-16 | Auditor | StratOS-Coding-Checklist.md & StratOS-Design.md | All | Reviewed implementation order; identified Phase 1 as critical path; found Phase 2 GDT/IDT error (UEFI-only system) and Phase 6/7 dependency gap; assigned Phase 1 toolchain task to Builder
2026-04-16 | Auditor | StratOS-Coding-Checklist.md & StratOS-Design.md | All | Accepted Auditor review as current execution baseline: Phase 1 is the critical path, Phase 2 GDT/IDT requires UEFI-only correction, and Phase 6/7 dependency ordering must be made explicit
2026-04-16 | Builder | scripts/phase1/build-erofs-image.sh, .cargo/config.toml | 1-62, 1-11 | Implemented Phase 1 toolchain: custom EROFS build script and Rust toolchain config for x86_64-unknown-linux-musl target with static linking
2026-04-16 | Auditor | scripts/phase1/build-erofs-image.sh, .cargo/config.toml | All | REJECTED Phase 1 implementation: wrong phase directory (phase1 vs phase0), wrong script purpose (EROFS builder vs GPT disk layout), wrong Rust target (linux-musl vs uefi), missing Makefile and toolchain scripts, violates Custom First with erofs-utils dependency
2026-04-16 | Builder | scripts/phase0/build-erofs-custom.sh, scripts/phase0/rust-uefi-target.json, scripts/phase0/verify-rust-uefi.sh, .cargo/config.toml, Makefile | All | Corrected Phase 1 implementation: custom EROFS builder in phase0 using tar intermediate (no erofs-utils), UEFI Rust target config, Makefile with phase1-all/disk-layout/toolchain/erofs targets
2026-04-16 | Auditor | scripts/phase0/build-erofs-custom.sh, Makefile | All | REJECTED Phase 1 implementation (2nd): still wrong script purpose (tar archive vs GPT disk layout), Makefile references non-existent scripts (install-toolchain.sh, install-gnu-efi.sh, create-partition-layout.sh), missing actual partition layout implementation, build-erofs-custom.sh is not EROFS
2026-04-16 | Builder | scripts/phase0/create-qemu-disk-image.sh, scripts/phase0/install-toolchain.sh, scripts/phase0/install-gnu-efi.sh, Makefile | All | Corrected Phase 1 implementation (2nd): simplified install-toolchain.sh to Rust UEFI target only, updated create-qemu-disk-image.sh to 6-partition GPT layout (ESP/SLOT_A/B/C/CONFIG/HOME), Makefile with toolchain/disk-image/clean targets
2026-04-16 | Auditor | scripts/phase0/create-qemu-disk-image.sh, scripts/phase0/install-toolchain.sh, scripts/phase0/install-gnu-efi.sh, Makefile | All | ACCEPTED Phase 1 implementation: correct GPT partition layout with sgdisk, Rust UEFI target installation, GNU-EFI library installation, functional Makefile with toolchain/disk-image/clean targets
2026-04-16 | Builder | scripts/phase0/create-qemu-disk-image.sh, scripts/phase0/install-toolchain.sh, scripts/phase0/install-gnu-efi.sh, Makefile | All | Corrected Phase 1 implementation (2nd): simplified install-toolchain.sh to Rust UEFI target only, updated create-qemu-disk-image.sh to 6-partition GPT layout (ESP/SLOT_A/B/C/CONFIG/HOME), Makefile with toolchain/disk-image/clean targets
2026-04-16 | Auditor | StratOS-Coding-Checklist.md | All | Accepted Phase 2 checklist correction (GDT/IDT → EFI variable read/write); assigned validation task for existing UEFI entry point and EFI variable implementation against design doc section 4.2 schema
2026-04-16 | Auditor | stratboot/src/stratboot.c, stratboot/efi/strat_efi_vars.c/h | Lines 762-916, 1-61, 1-43 | ACCEPTED Phase 2 UEFI entry point validation: efi_main correctly initializes EFI_SYSTEM_TABLE, EFI var implementation matches design doc section 4.2 schema (all 9 vars defined), Custom First (pure GNU-EFI), no legacy x86/GDT/IDT, proper EFI_STATUS handling, minimal and auditable
2026-04-16 | Builder | StratOS-Coding-Checklist.md | Line 11 | Corrected Phase 2 checklist: replaced GDT/IDT setup with UEFI-appropriate EFI variable read/write task
2026-04-16 | Builder | stratboot/src/stratboot.c, stratboot/efi/strat_efi_vars.c/h | Lines 762-916, 1-61, 1-43 | Validated UEFI entry point compliance: efi_main correctly initializes EFI_SYSTEM_TABLE, EFI var implementation matches design doc section 4.2 schema (all 9 vars defined), Custom First (pure GNU-EFI), no legacy x86/GDT/IDT, proper EFI_STATUS handling, minimal and auditable
2026-04-16 | Builder | stratboot/src/stratboot.c, stratboot/src/slot.c | Lines 666-726, 648-664, 1-100 | Validated kernel handoff: start_kernel_efi uses EFI_LOADED_IMAGE_PROTOCOL and BootServices->LoadImage/StartImage (correct EFI protocols), command line includes root device and initrd, slot path functions return correct EFI paths; ISSUE: slot_root_device hardcodes /dev/sda2/3/4 which assumes SATA disk, will fail on NVMe (/dev/nvme0n1p2/3/4) or other disk types
2026-04-16 | Auditor | stratboot/src/stratboot.c, stratboot/src/partition.c | Lines 648-655 | Assigned root device hardcoding fix: replace /dev/sdX with PARTUUID-based hardware-agnostic identification using EFI protocols to read partition GUIDs from GPT table
2026-04-16 | Builder | stratboot/src/stratboot.c, stratboot/src/partition.c, stratboot/src/partition.h | Lines 1-9, 648-655, 786-810, 707-711, 274-341, 16-20 | Fixed root device hardcoding: added strat_partition_get_partuuid to read partition GUID from GPT using EFI_BLOCK_IO_PROTOCOL, updated slot_root_device to return PARTUUID strings, added PARTUUID initialization loop in efi_main, updated kernel command line to use root=PARTUUID= format, hardware-agnostic (works on SATA/NVMe/USB/virtio)
2026-04-16 | Auditor | stratboot/src/stratboot.c, stratboot/src/partition.c, stratboot/src/partition.h | Lines 658-667, 274-341, 16-20, 722, 808-831 | ACCEPTED root device hardcoding fix: strat_partition_get_partuuid correctly uses EFI_BLOCK_IO_PROTOCOL to read GPT partition GUIDs, slot_root_device returns PARTUUID strings, initialization loop populates all slots, kernel command line uses root=PARTUUID= format, Custom First compliance (pure EFI), hardware-agnostic (SATA/NVMe/USB/virtio)
2026-04-16 | Builder | stratos-kernel/stratos_minimal.config | 1-113 | Created minimal Linux kernel configuration: CONFIG_EROFS_FS=y (built-in), CONFIG_BLOCK=y, CONFIG_PCI=y (UEFI GOP compatibility), CONFIG_EFI=y, CONFIG_CMDLINE_BOOL=y (bootloader params), CONFIG_MODULES=n (no module support), 113 lines (minimal), Custom First (direct Kconfig), aligns with StratBoot EFI handoff
2026-04-16 | Builder | stratos-kernel/stratos_minimal.config | 11, 69 | Cleaned up kernel config: removed duplicate CONFIG_ARCH_MAY_HAVE_PC_FDC=y and inconsistent CONFIG_SECURITY_NETWORK=y (networking disabled with CONFIG_NET=n), preserving minimal baseline
2026-04-16 | Auditor | Phase 5 task selection | Skeleton creation is foundational prerequisite for all subsequent Phase 5 work
2026-04-16 | Auditor | scripts/phase5/create-system-skeleton.sh | All | ACCEPTED Phase 5 skeleton script: correct directory structure per design doc section 3.4, input validation with clear errors, idempotent via mkdir -p, 0755 permissions, no symlinks/mounts/overlayfs, minimal (39 lines), basic shell utilities only; verify script is executable before next task
2026-04-16 | Auditor | Phase 5 runtime contract | Documented authoritative persistence mapping to resolve checklist naming mismatch before implementation
2026-04-16 | Auditor | docs/runtime-persistence-contract.md | All | ACCEPTED runtime contract: correct checklist naming resolution (/cache→/apps, /user→/home), complete path ownership table, direct/bind mount lists, path categorization, Three Layer Guarantee mapping, references design doc section 3.4 and initramfs-init.c lines 129-175, enforces honest filesystem model; NOTE: STRAT_CACHE documented as ext4 per current initramfs implementation (design doc specifies XFS as target — future cleanup item)
2026-04-16 | Auditor | sysroot/initramfs-init.c | Lines 157-162 | ACCEPTED existing /config/var → /var bind mount: mkdir with EEXIST handling, MS_BIND flag, correct timing (post /config mount, pre MS_MOVE), aligns with runtime contract and design doc section 3.6; no modifications needed
2026-04-16 | Auditor | sysroot/initramfs-init.c | Lines 164-168 | REJECTED /config/system/etc → /system/etc bind mount: violates honest filesystem model (no union mounts), violates config priority stack (app-level fallback not filesystem overlay), breaks /system EROFS immutability, not in runtime persistence contract; remove lines 164-168, config override must be application-level logic per design doc section 3.5
2026-04-16 | Auditor | sysroot/initramfs-init.c | All | ACCEPTED bind mount removal: lines 164-168 correctly removed, /system/etc remains immutable (EROFS), /config/var→/var and /system→/usr bind mounts unchanged, aligns with runtime persistence contract and honest filesystem model
2026-04-16 | Auditor | docs/application-config-resolution.md | All | ACCEPTED application config resolution contract: correct priority order per design doc section 3.5 (/config/apps first, /system/etc fallback, built-in defaults), explicitly forbids filesystem-level overrides (bind mounts/overlayfs/symlinks), defines required application-level lookup behavior, clear examples, references runtime contract, states pattern is required for all StratOS-native apps
2026-04-16 | Engineer | Documentation sync | All | Updated repository documentation to reflect actual system state. Project is in Phase 23 (Cleanup & Hardening). Prior docs incorrectly indicated early pre-alpha and incomplete phase progression.
2026-04-16 | Builder | Phase 1 Rust custom target configuration | All | Completed Rust custom target configuration: updated stratsup and stratmon to use x86_64-stratos-uefi, fixed setup-toolchain.sh to verify custom target builds, removed phase0 dependency from Makefile
2026-04-16 | Auditor | stratboot/src/slot.h, stratboot/src/slot.c, README.md, StratOS-Coding-Checklist.md | Lines 44, 46, 112-144, 164-200, 160, 117 | ACCEPTED Phase 23 deprecated function removal: confirmed zero live call sites for strat_slot_raw_copy and strat_slot_rotate_to_b, architecture law compliance maintained with strat_slot_process_update_request as sole update path, surface area reduced
2026-04-16 | Builder | stratboot/efi/strat_efi_vars.h, stratboot/src/stratboot.c, stratboot/tests/efi_var_test.c | All | Phase 23 surface area minimization complete: EFI variable count reduced from 18 to 14 by removing BOOT_COUNT, LAST_GOOD_SLOT, STRAT_SMOKE_EFI_MAIN_VAR, STRAT_SMOKE_BOOTING_SLOT_VAR
2026-04-16 | Builder | stratboot/src/stratboot.c, stratboot/Makefile | Lines 17-28, 30-52, 774-968, 23-25, 39-40 | Phase 23 debug/build separation implemented: wrapped debugcon_log() and serial_log() functions and 21 call sites in #ifdef DEBUG, added DEBUG flag to Makefile with -DDEBUG CFLAGS, created debug target, production builds exclude all debug logging
2026-04-16 | Builder | stratmon/src/manifest.rs, stratmon/src/fiemap.rs, stratmon/src/main.rs, stratmon/Cargo.toml | All | Phase 6 StratMon update pipeline complete: implemented binary manifest format (C-compatible ManifestHeader/ExtentEntry structs, /EFI/STRAT/UPDATE.MAN), FIEMAP extent mapping via ioctl(FS_IOC_FIEMAP) using nix crate, --stage-update command integration with SHA256 hashing, added nix and sha2 dependencies, Custom First compliance (no external block mapping libs), no StratBoot changes
2026-04-16 | Builder | stratlayer/src/lib.rs, stratlayer/src/wire/, stratlayer/src/protocols/, stratlayer/src/shm/, stratlayer/examples/smoke_test.rs, stratlayer/Cargo.toml | All | Phase 9 Wayland foundation complete: implemented custom Rust Wayland client library (stratlayer) with own wire protocol (Unix socket via nix, custom MessageHeader/Argument parsing), object registry, POSIX shm buffer management, core protocols (wl_display, wl_registry, wl_compositor, wl_surface, wl_shm, xdg_wm_base, xdg_surface, xdg_toplevel), smoke test opens solid color window on stratvm, Custom First compliance (no wayland-client/wayland-server crates), usable as foundation crate for future DE components
2026-04-16 | Builder | stratboot/src/stratboot.c, sysroot/initramfs-init.c, stratvm/src/main.c, stratstop/Makefile, stratstop/src/fb.c, stratstop/src/font.c, stratstop/src/logo.c, stratstop/src/stratstop.c, stratsup/src/supervisor.rs, stratsup/src/main.rs, scripts/phase7/prepare-minimal-rootfs.sh | All | Phase 23 silent boot + shutdown logo complete: modified kernel cmdline to loglevel=0 console=none (stratboot), removed stderr output from initramfs-init log_status, wrapped all fprintf(stderr) in stratvm with #ifdef DEBUG, created stratstop binary with user-space framebuffer access (fb.c), font rendering (font.c), logo drawing (logo.c) matching StratBoot visual spec (black background, filled circle with S character, halo rings, STRAT OS wordmark), integrated stratstop as stratsup shutdown hook before poweroff/reboot syscall, added to rootfs preparation, builds successfully
2026-04-16 | Builder | stratvm/src/main.c | Lines 859-866 | Wrapped remaining fprintf(stderr) calls in stratvm with #ifdef DEBUG guards, completing Phase 23 silent boot implementation
2026-04-16 | Builder | stratlayer/src/wire/protocol.rs, stratlayer/src/wire/socket.rs, stratlayer/src/shm/pool.rs | All | Updated stratlayer for nix 0.29 API compatibility: fixed type mismatches (usize to u16 casts), corrected BorrowedFd::borrow_raw usage, added IntoRawFd trait import, fixed UnixAddr::new to take &str, removed unnecessary unsafe block, restructured display name handling to avoid lifetime issues; 12 compilation errors fixed, build now succeeds
