> **StratOS coding checklist.** Testing lives elsewhere. **Custom first:** prefer in-tree code over new dependencies.  
> **Phase IDs are stable** for logs and PRs. **Body order** matches boot dependency: **1–7**, then **12b**, then **8–20**, **22–23**, **24a–26**. Phases **21** and **27+** are unused / reserved.  
> *Each line below is a checkbox item only (`[x]` done, `[ ]` not done). Path hints sit in the item text.*

---

## Phase index (jump links — not completion state)

1. [Phase 1 — Toolchain](#p1)
2. [Phase 2 — StratBoot](#p2)
3. [Phase 3 — Kernel](#p3)
4. [Phase 4 — Initramfs / EROFS root](#p4)
5. [Phase 5 — Filesystem layout](#p5)
6. [Phase 6 — StratMon](#p6)
7. [Phase 7 — Slot writes](#p7)
8. [Phase 12b — stratman](#p12b)
9. [Phase 8 — StratSup](#p8)
10. [Phase 9 — stratlayer](#p9)
11. [Phase 10 — stratvm](#p10)
12. [Phase 11 — Stratterm](#p11)
13. [Phase 12 — Spotlite](#p12)
14. [Phase 13 — `.strat](#p13)`
15. [Phase 14 — Config API](#p14)
16. [Phase 15 — Network](#p15)
17. [Phase 16 — Session](#p16)
18. [Phase 17 — Audio](#p17)
19. [Phase 18 — Security](#p18)
20. [Phase 19 — Recovery](#p19)
21. [Phase 20 — Integration](#p20)
22. [Phase 22 — Build](#p22)
23. [Phase 23 — Hardening](#p23)
24. [Phase 24a — Compositor IPC](#p24a)
25. [Phase 24 — Panel](#p24)
26. [Phase 25 — Windows](#p25)
27. [Phase 26 — Settings](#p26)

**Dependency hint:** After **12b**, compositor **10** autostarts **11** + **24** from `stratvm/src/main.c`. Updates **6–7** and network **15** can run in parallel with desktop work.

---

### Part A - Boot and updates (1-7)

## Phase 1: Toolchain & build system

- EROFS system image in build pipeline (`build-all-and-run.sh` → `mkfs.erofs`, `out/phase7/`)
- Rust workspace components (`stratman/`, `stratpanel/`, `stratterm/`, `stratmon/`, `stratlayer/`, `stratsup/`)
- GPT test disk helpers (`scripts/create-test-disk.sh`, `scripts/update-test-disk.sh`)

## Phase 2: StratBoot (UEFI)

- UEFI entry and UI (`stratboot/src/stratboot.c`)
- EFI variable slot state (`stratboot/efi/strat_efi_vars.`*, `stratboot/src/slot.c`)
- Slot selection + kernel handoff (`stratboot/src/stratboot.c` — `start_kernel_efi`, PARTUUID cmdline)

## Phase 3: Kernel tuning & handoff

- StratOS Kconfig merge (`stratos-kernel/stratos.config` on `linux/` tree)
- Bootloader → kernel cmdline / initrd on ESP (`\EFI\STRAT\SLOT_`*; `scripts/update-test-disk.sh`)
- Serial / early console (`stratos.config`, StratBoot `console=ttyS0`)

## Phase 4: Immutable root (EROFS)

- Initramfs PID1 mounts (`sysroot/initramfs-init.c` — `root=` EROFS, `config=`/`apps=`/`home=`)
- Read-only `/system`, pivot into rootfs (`initramfs-init.c` → `execv("/bin/stratman")`)
- Initramfs image build (`build-all-and-run.sh` cpio + static `init`)

## Phase 5: Honest filesystem layout

- Separate config / apps / home from immutable system (`initramfs-init.c`; [runtime-persistence-contract.md](runtime-persistence-contract.md))
- Bind mounts for `/etc` and `/var` (`/config/etc`, `/config/var` in `initramfs-init.c`; `stratman/src/main.rs` best-effort repeat)
- Rootfs skeleton + provisioning (`build-all-and-run.sh` rootfs stage, `sysroot/first-boot-provision.sh`)

## Phase 6: StratMon (update conductor)

- Binary update manifest (`stratmon/src/manifest.rs`; staging path in `stratmon/src/main.rs` e.g. `/EFI/STRAT/UPDATE.MAN`)
- FIEMAP extent mapping (`stratmon/src/fiemap.rs`)
- EFI variable signaling for updates (`stratmon/src/main.rs` via `stratsup::efi_vars` on host)

## Phase 7: Slot write logic (StratBoot surgeon)

- Raw block I/O in StratBoot (`stratboot/src/` — `strat_slot_process_update_request` et al.)
- Pre-boot hash verification (`stratboot/src/sha256.c` + call sites)
- Slot rotation / state machine (`stratboot/src/slot.c`)
- StratBoot applies staged **UPDATE.MAN** on all production update paths (audit `stratboot.c` vs `stratmon` writer)

---

### Part A (cont.) - PID 1 before session clients

## Phase 12b: stratman (PID 1 orchestrator)

- PID 1 entry (`stratman/src/main.rs` — mounts, env, `service::load_and_run_all`)
- TOML service manifests (`stratman/manifests/*.toml`, `stratman/src/service.rs`)
- Maintenance / idle queue (`stratman/src/maint.rs`)
- Initramfs exec target (`sysroot/initramfs-init.c` → `/bin/stratman`)
- Per-service namespaces (`stratman/src/service.rs`)

---

### Part B - Legacy supervisor and Wayland clients (8-12)

## Phase 8: StratSup (supervisor) - legacy

- Rust `stratsup/` crate (host EFI helpers for `stratmon`; not PID 1 on target)
- Service-style modules in `stratsup/src/`
- validate-boot / logging helpers pulled into rootfs (`build-all-and-run.sh`)

## Phase 9: Wayland foundation (stratlayer)

- Hand-rolled client protocol core (`stratlayer/src/`)
- SHM pools / buffers (`stratlayer/src/shm/`)
- Consumers wired (`stratpanel`, `stratterm` depend on `stratlayer` in `Cargo.toml`)

## Phase 10: stratvm (compositor)

- Tiling / workspaces (`stratvm/src/main.c`, `stratvm/src/server.h`)
- wlroots scene + backends (`stratvm/Makefile`, `stratvm/src/main.c`)
- Keyboard + pointer (libinput + direct evdev hotplug in `stratvm/src/main.c`)

## Phase 11: Stratterm (terminal)

- PTY + Wayland front end (`stratterm/src/pty.rs`, `wayland.rs`, `main.rs`)
- Renderer + font path (`stratterm/src/renderer.rs`, `font.rs`)
- Escape parser (`stratterm/src/parser.rs`)

## Phase 12: Spotlite

- Filesystem indexer + SQLite (`stratterm/src/bin/stratterm-indexer.rs`; `/config/strat/indexer.conf`)
- In-terminal file browser overlay (`stratterm/src/file_browser.rs`, `stratterm/src/main.rs` — F7 / browser flows)
- Dedicated global Spotlite launcher (search-first overlay across apps per design)

---

### Part C - Platform and policy (13-20)

## Phase 13: `.strat` package format

- Define `.strat` structure (archive + manifest)
- Cryptographic signing and verification
- Sandboxed execution runner

## Phase 14: System configuration API

- Hand-rolled TOML for panel (`stratpanel/src/config.rs`, `/config/strat/panel.conf`)
- Hand-rolled TOML for indexer CLI (`stratterm/src/bin/strat-settings.rs`, `/config/strat/indexer.conf`)
- Atomic system-wide config API for all subsystems under `/config`
- General system preferences CLI (not indexer-only)

## Phase 15: Network stack (minimal)

- `strat-network` stratman child (`stratman/src/network.rs`, `stratman/manifests/strat-network.toml`, `--network`)
- DHCP client in-tree (`stratman/src/network.rs`)
- StratMon HTTPS fetch + trust store for update payloads
- Kernel drivers in `stratos-kernel/stratos.config` (virtio / common NICs as enabled)

## Phase 16: User session management

- Login / auth module
- Session environment injection beyond stratman static env
- Logout and cgroup / session cleanup

## Phase 17: Audio (minimal)

- ALSA / PipeWire minimal interface
- Volume control integrated with panel tray (not config-only)
- Notification sound trigger

## Phase 18: Security & sandboxing

- Landlock or seccomp for `.strat` apps
- Capability-based permissions
- Kernel hardening sysctl defaults

## Phase 19: Recovery environment

- Safe Mode boot target
- Factory reset flow (CONFIG / HOME per [runtime-persistence-contract.md](runtime-persistence-contract.md))
- Slot repair / recovery CLI for StratBoot

## Phase 20: Final system integration

- System-wide theme / asset manager
- First-boot setup wizard (custom UI)

---

### Part D - Build, hardening, and desktop (22-26)

## Phase 22: Build system

- `build-all-and-run.sh` (kernel, `stratboot`, `stratvm`, Rust bins, initramfs, rootfs, EROFS, disk, QEMU)
- Default dev path uses `scripts/*.sh` only (no `scripts/phaseN/` tree required)
- GCC15 kernel build shim when needed (`build-all-and-run.sh`)
- QEMU logs `ide-logs/qemu_strattest.txt` + serial (`scripts/run-qemu.sh`, `SERIAL_LOG_PATH`)

## Phase 23: Cleanup & hardening

- Deprecated slot / dead paths removed (current `stratboot` tree)
- EFI variable surface reduction (`stratboot/efi/strat_efi_vars.h`)
- StratBoot `DEBUG` gated logging (`stratboot/Makefile`, `stratboot.c`)

## Phase 24a: Compositor prerequisites

- `wlr_layer_shell_v1` (`stratvm/src/main.c` — `server_new_layer_surface_notify`)
- Unix IPC `/run/stratvm.sock` (`stratvm/src/main.c`)

## Phase 24: Panel

- Panel binary + `LAYER_TOP` (`stratpanel/src/main.rs`; `stratvm/src/main.c` `spawn_autostart("/bin/stratpanel", …)` — not a `stratman` manifest)
- Pinned app strip UI (`pinned.apps` in `main.rs`: scroll wheel + click launch absolute paths)
- Workspace switcher + IPC (`stratpanel/src/ipc.rs`, compositor commands in `stratvm`)
- Clock shown in panel (`stratpanel/src/main.rs`, `clock.rs`)
- [~~] Tray area in `main.rs`: stub cells **N/V/U/B** (`~~`= not wired,`-` = hidden via config); real volume/network/etc. still open
- IPC `set panel autohide` + compositor flag (`stratpanel/src/ipc.rs`, `stratvm/src/main.c` `panel_autohide`)
- Auto-hide in `stratpanel`: `set_size`/`set_exclusive_zone` peek bar + debounced collapse on pointer leave (`wl_pointer.leave` + `stratlayer`); expand on enter/motion/click
- Panel IPC client (`stratpanel/src/ipc.rs`)
- `/config/strat/panel.conf` reader (`stratpanel/src/config.rs`)

## Phase 25: Window management

- Titlebar + close + float/max toggle (`stratvm/src/main.c` — scene rects, buttons)
- Minimize from titlebar (min button currently `wlr_xdg_toplevel_send_close` — needs real iconify/minimize)
- Interactive move / drag (`stratwm_apply_move_grab`, `grabbed_view`, titlebar + `request_move` in `stratvm/src/main.c`)
- Layer Z-order: separate `wlr_scene_tree` layers; XDG under `layers_normal` (`stratvm/src/server.h`, `main.c`)
- Titlebar right-click on empty titleband (`stratvm/src/main.c`): plain = toggle decorations; Super = move to next workspace; Shift = float toggle (no client-side menu yet)
- Tiling / floating toggle (`toggle_float` in `stratvm/src/main.c`)
- Cover Flow window switcher (Super+Tab)
- Tabbed mode (Super+Shift+W)
- Decoration sizing from `/config/strat/stratvm.conf`: `titlebar_height=`, `border_pad=` (see `stratwm_load_deco_config`); radius / button shapes still hardcoded

## Phase 26: Settings

- Indexer settings CLI (`stratterm/src/bin/strat-settings.rs` — `/config/strat/indexer.conf`)
- Settings app (search-first UI) + full panels (display, sound, network, power, input, software, security, about)
- Appearance panel (theme, decorations, fonts) wired to compositor
- Slots / updates UI (EFI + StratMon state)
- Recovery UI (CONFIG / HOME / factory reset)
- Settings IPC to stratvm + stratman (panel uses stratvm socket only today)