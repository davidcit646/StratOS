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
- [ ] Define minimal Kconfig for StratOS kernel
- [ ] Implement bootloader-to-kernel parameter passing
- [ ] Set up early serial console output

### Phase 4: Immutable Root (EROFS)
- [ ] Implement system image mounting logic
- [ ] Create `/system` mount as read-only EROFS
- [ ] Setup initial ramdisk (initramfs) with custom init

### Phase 5: Honest Filesystem Layout
- [ ] Implement separation of `/config`, `/cache`, and `/user`
- [ ] Build the overlayfs or bind-mount logic for persistent configs
- [ ] Script the creation of the system skeleton

### Phase 6: StratMon (Update Conductor)
- [ ] Implement the Update manifest parser (Rust)
- [ ] Build the FIEMAP extent mapping logic
- [ ] Create the EFI variable writer for update requests

### Phase 7: Slot Write Logic (Surgeon)
- [ ] Implement `EFI_BLOCK_IO_PROTOCOL` raw copy in StratBoot
- [ ] Build the pre-boot hash verification engine
- [ ] Implement slot rotation logic (A -> B transition)

### Phase 8: StratSup (Supervisor)
- [ ] Write custom PID 1 supervisor in Rust
- [ ] Implement service spawning and monitoring
- [ ] Create basic logging pipe for system services

### Phase 9: Wayland Foundation
- [ ] Implement custom Wayland protocol bindings
- [ ] Build minimalist shared memory (shm) allocator
- [ ] Create basic DRM/KMS backend for display output

### Phase 10: Strat WM (Compositor)
- [ ] Implement tiling engine logic
- [ ] Build custom GPU-accelerated rendering pipeline
- [ ] Create input handling for keyboard and mouse (libinput wrapper)

### Phase 11: StratTerm (Terminal)
- [ ] Implement custom PTY handling logic
- [ ] Build custom GPU text renderer (no heavy font libs)
- [ ] Implement VT100/Xterm escape sequence parser

### Phase 12: Spotlite (Search)
- [ ] Implement local filesystem indexing engine
- [ ] Build the instant-access UI overlay
- [ ] Create the application launcher logic

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
- [ ] Create the final unified build-and-deploy pipeline

### Phase 21: Update System Hardening
- [ ] Implement StratMon update conductor (Rust)
- [ ] Build FIEMAP extent mapping logic
- [ ] Create EFI variable writer for update requests

### Phase 22: Slot Write Logic Surgeon
- [ ] Implement EFI_BLOCK_IO_PROTOCOL raw copy in StratBoot
- [ ] Build pre-boot hash verification engine
- [ ] Implement slot activation logic via strat_slot_process_update_request

### Phase 23: Cleanup & Hardening
- [x] Target State Cleanup – APPROVED (deprecated legacy slot rotation, removed dead code paths)
- [ ] Minimize EFI variable surface area
- [ ] Introduce debug/build separation
