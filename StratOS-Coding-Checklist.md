> **This is the clean coding checklist for StratOS. Testing and validation are handled in a separate workflow. Follow the 'Custom First' rule: implement components ourselves before adding external dependencies.**

### Phase 1: Toolchain & Build System
- [x] Implement custom build script for EROFS image generation
- [x] Configure Rust toolchain for custom target specs
- [x] Create initial disk image layout utility (Partitioning A/B/C slots)

### Phase 2: StratBoot (Low-Level)
- [x] Implement basic UEFI entry point in C
- [x] Build EFI variable reader for slot selection
- [x] Implement EFI variable read/write for slot selection

### Phase 3: Kernel Tuning & Handoff
- [x] Define minimal Kconfig for StratOS kernel
- [x] Implement bootloader-to-kernel parameter passing
- [x] Set up early serial console output

### Phase 4: Immutable Root (EROFS)
- [x] Implement system image mounting logic
- [x] Create `/system` mount as read-only EROFS
- [x] Setup initial ramdisk (initramfs) with custom init

### Phase 5: Honest Filesystem Layout
- [x] Implement separation of `/config`, `/cache`, and `/user`
- [x] Build the overlayfs or bind-mount logic for persistent configs
- [x] Script the creation of the system skeleton

### Phase 6: StratMon (Update Conductor)
- [x] Implement the Update manifest parser (Rust)
- [x] Build the FIEMAP extent mapping logic
- [x] Create the EFI variable writer for update requests

### Phase 7: Slot Write Logic (Surgeon)
- [x] Implement `EFI_BLOCK_IO_PROTOCOL` raw copy in StratBoot
- [x] Build the pre-boot hash verification engine
- [x] Implement slot rotation logic (A -> B transition)

### Phase 8: StratSup (Supervisor)
- [x] Write custom PID 1 supervisor in Rust
- [x] Implement service spawning and monitoring
- [x] Create basic logging pipe for system services

### Phase 9: Wayland Foundation
- [x] Implement custom Wayland protocol bindings
- [x] Build minimalist shared memory (shm) allocator
- [x] Create basic DRM/KMS backend for display output

### Phase 10: stratvm (Compositor)
- [x] Implement tiling engine logic
- [x] Build custom GPU-accelerated rendering pipeline
- [x] Create input handling for keyboard and mouse (libinput wrapper)

### Phase 11: StratTerm (Terminal)
- [x] Implement custom PTY handling logic
- [x] Build custom GPU text renderer (no heavy font libs)
- [x] Implement VT100/Xterm escape sequence parser

### Phase 12: Spotlite (Search)
- [ ] Implement local filesystem indexing engine
- [ ] Build the instant-access UI overlay
- [ ] Create the application launcher logic

### Phase 12b: stratman (PID 1 Orchestrator)
- [x] Implement stratman as PID 1 (replace stratsup)
- [x] Build service spawning and lifecycle management
- [x] Implement maintenance window logic (idle detection + deferred updates)
- [x] Wire stratman into initramfs handoff (switch_root into stratman)
- [x] Implement namespace guard

### Phase 24a: Compositor Prerequisites (Panel & Settings)
- [x] Add wlr_layer_shell_v1 support to stratvm compositor
- [x] Implement IPC socket server in stratvm (/run/stratvm.sock) with command parser

### Phase 24: Panel
- [x] Implement panel binary (top bar, always-on-top Wayland layer surface)
- [ ] Build pinned app launcher with scrollable strip
- [ ] Build workspace switcher (click to switch, drag window to workspace)
- [ ] Build system tray (clock, volume, network, battery, brightness, updates)
- [ ] Implement auto-hide (slide up on mouse leave, slide down on top edge)
- [ ] Wire panel IPC to stratvm socket (/run/stratvm.sock)
- [x] Implement panel.conf TOML config reader (/config/strat/panel.conf)

### Phase 25: Window Management
- [x] Implement window decorations (titlebar, close/minimize/maximize buttons)
- [ ] Build right-click titlebar context menu (float, remove titlebar, move to workspace)
- [x] Implement per-window tiling/floating toggle
- [ ] Build Cover Flow window switcher (Super+Tab, live textures, perspective transform)
- [ ] Implement tabbed mode (Super+Shift+W, stacked windows with tab headers)
- [ ] Implement configurable decoration rendering (corner radius, border width, button style/position)

### Phase 26: Settings
- [ ] Implement settings binary with search-first UI
- [ ] Build display, sound, network, power, input, software, default apps, security, and about panels
- [ ] Build appearance panel (theme, window decorations, fonts)
- [ ] Build slots panel (view slot state, pin/unpin, manage updates)
- [ ] Build recovery panel (reset CONFIG, schedule HOME wipe, factory reset)
- [ ] Wire settings IPC to stratvm and stratman sockets

### Phase 13: .strat Package Format
- [ ] Define `.strat` file structure (compressed archive + manifest)
- [ ] Implement cryptographic signing and verification
- [ ] Build the sandboxed execution runner

### Phase 14: System Configuration API
- [ ] Create a custom TOML-based system configuration parser
- [ ] Implement atomic config writes for `/config`
- [ ] Build CLI tool for system-wide preference management

### Phase 15: Network Stack Minimalist
- [ ] Implement custom wrapper for network interface management
- [ ] Build minimalist DHCP client
- [ ] Create StratMon integration for secure HTTPS downloads

### Phase 16: User Session Management
- [ ] Implement custom login/auth module
- [ ] Build session environment variable injector
- [ ] Create user-space process group cleanup on logout

### Phase 17: Audio Subsystem (Minimal)
- [ ] Implement basic ALSA/Pipewire minimal interface
- [ ] Build custom volume control logic
- [ ] Create system notification sound trigger

### Phase 18: Security & Sandboxing
- [ ] Implement Landlock or Seccomp profiles for `.strat` apps
- [ ] Build capability-based permission system
- [ ] Create kernel-level hardening sysctl defaults

### Phase 19: Recovery Environment
- [ ] Build minimalist "Safe Mode" boot target
- [ ] Implement "Factory Reset" (Wipe `/config` and `/cache`)
- [ ] Create manual slot-repair tool for StratBoot

### Phase 20: Final System Integration
- [ ] Implement system-wide theme/asset manager
- [ ] Build the first-boot setup wizard (Custom UI)

### Phase 23: Cleanup & Hardening
- [x] Target State Cleanup – APPROVED (deprecated legacy slot rotation, removed dead code paths)
- [x] Minimize EFI variable surface area
- [x] Introduce debug/build separation
