# STRAT OS
### System Design Document v0.4
*"The system is the bedrock. Home is yours. Touch neither uninvited."*

---

## 1. PHILOSOPHY

Strat OS is a modern, immutable, self-healing Linux-based desktop operating system. It is a genuine daily driver. It is opinionated about defaults and completely open about everything else.

### 1.1 The Fairphone Principle

Strat OS is the Fairphone of operating systems.

Fairphone builds phones that the average person can use without thinking about the hardware, and that the advanced user can disassemble, modify, and rebuild from components. The phone breaks? Get the part. Fix it. Do it again. Zero lock-in. Zero artificial barriers.

Strat OS works the same way.

```
Average user (the squirrel)
    It works out of the box
    Updates itself when asked
    Looks and feels modern
    If something breaks — one screen explains it and offers to fix it
    Never sees a compiler
    Never edits a config file
    Never thinks about partitions

Advanced user
    Full source available and documented
    Custom kernel config
    Build everything from source
    Replace any component
    Edit config files directly
    Write your own .strat packages
    The entire system is yours to disassemble
    Nothing is hidden. Nothing is locked. Nothing fights you.

System breaks completely?
    Boot the live USB
    Diagnose from outside the broken system
    Repair the exact broken layer
    Or wipe and reinstall in 5 minutes
    Zero cost. Zero drama. Try again.
```

This is not a philosophy statement. It is an engineering requirement. Every design decision must serve both the squirrel and the power user without compromising either.

### 1.2 Core Tenets

- The system protects itself. The user never should have to.
- Zero nag. One message in. One message out.
- Power and simplicity are not opposites.
- If the user didn't ask for it, don't show it.
- Failures are handled automatically, reported plainly, never hidden.
- No symlinks pretending to be a filesystem. What you see is what's there.
- Every layer is independently destroyable and independently survivable.
- The bootloader is sovereign. It answers to no partition.
- Source first. Binaries when necessary. Never locked in.

### 1.3 The Three Layer Guarantee

The single most important architectural decision in Strat OS.

```
SYSTEM   (SLOT_A/B/C)
    The OS works.
    Read-only. Immutable. Self-healing.
    Lose it: system reinstalls from pinned slot.

CONFIG   (CONFIG partition)
    It looks and behaves like yours.
    Settings, preferences, SSH keys, shell history.
    Lose it: system boots to defaults. All data intact.
    Survives HOME destruction completely.

HOME     (HOME partition)
    Your files are there.
    Documents, media, projects, downloads.
    Lose it: painful but surgical. System and config untouched.
    Survives SYSTEM destruction completely.
```

Any single layer can be destroyed. The other two survive intact. This is not an accident. This is the design. Every architectural decision flows from this guarantee.

---

## 2. SYSTEM REQUIREMENTS

### Minimum
| Component | Requirement |
|---|---|
| RAM | 16GB |
| Storage | 256GB |
| CPU | x86_64, 4 cores, 2020 or newer |
| Boot | UEFI only. No legacy BIOS. Ever. |
| GPU | Any with modern Linux driver support |

### Recommended
| Component | Requirement |
|---|---|
| RAM | 32GB |
| Storage | 1TB NVMe |
| CPU | 8+ cores |
| GPU | Dedicated, Vulkan capable |

Strat OS is not for old hardware. It is not for low storage machines. It is not for people who want to run everything on a $200 laptop from 2015. There are other distros for that. This is not one of them.

---

## 3. ARCHITECTURE

### 3.1 Kernel

- Linux kernel (LTS base)
- **Minimal configuration** — only what Strat needs compiled in or available as modules
- **livepatch** — live kernel security and bug patches, no reboot required for 95% of updates
- **kexec** — major kernel version transitions, ~3 second handoff, no POST/BIOS cycle
- **CRIU** — process checkpoint/restore across kexec transitions

**What goes in:**
```
Core:        scheduler, memory manager, VFS, network stack, IPC
Filesystems: EROFS, Btrfs, XFS, ext4, FAT32, tmpfs, overlayfs
Hardware:    DRM/KMS, libinput, NVMe, SATA, USB, WiFi, Ethernet, ALSA
Security:    dm-verity, cgroups v2, namespaces, seccomp, EFI variables
Livepatch:   yes
kexec:       yes
CRIU:        yes
```

**What stays out:**
```
Virtualisation subsystems    — not a server
Legacy bus drivers           — ISA, parallel port, floppy
Debugging symbols            — production build, separate debug build for dev
Everything not explicitly needed
```

**Module strategy:**
```
Core subsystems    → compiled in (monolithic, always available)
Hardware drivers   → modular (loaded by udev on detection, flexible)
```

### 3.2 Partition Layout

Every partition has exactly one job. No partition does two jobs. No job is done by two partitions.

#### At 256GB (minimum)
```
ESP           512MB   FAT32    Bootloader, EFI variables, slot flags
SLOT_A         20GB   EROFS    Current system image (read-only, immutable)
SLOT_B         20GB   EROFS    Fallback / user-pinned image (read-only)
SLOT_C         20GB   EROFS    Update staging partition
CONFIG          4GB   ext4     User configuration — separate, sacred, always intact
HOME          ~191GB  Btrfs    User data — checksummed, compressed, snapshotted
```

#### At 1TB (recommended)
```
ESP           512MB   FAT32
SLOT_A         20GB   EROFS
SLOT_B         20GB   EROFS
SLOT_C         20GB   EROFS
CONFIG          4GB   ext4
STRAT_CACHE    50GB   XFS      Build cache for strat-build dependency compilation
HOME          ~885GB  Btrfs
```

### 3.3 Filesystem Choices — Justified

**ESP → FAT32**
UEFI specification requires it. No choice. No debate.

**SLOT_A/B/C → EROFS**
Read-only compressed filesystem. Used by Android for system partitions. Immutability is guaranteed at the filesystem level — not by a mount flag, not by policy, by the filesystem itself. You cannot write to EROFS even with root. Smaller system images due to compression. Faster reads due to layout optimization. Exactly right for system slots.

**CONFIG → ext4**
Small partition. Rarely written. Boring and stable. ext4 journaling protects the small amount of config data. No need for the complexity of Btrfs here. Stability is the only requirement.

**STRAT_CACHE → XFS**
Large parallel writes. Build artifacts, compiled dependencies, package cache. XFS was designed for exactly this workload. High throughput, handles large files, scales well. Better choice than ext4 for this use case.

**HOME → Btrfs**
Your data deserves the best filesystem available.
- Per-block checksums — detects silent corruption before it ruins a file
- Transparent zstd compression — text, code, documents compress 3-5x, effectively more space
- Copy-on-write — snapshots are nearly free
- Scrub — proactive corruption detection, not reactive
- This is where irreplaceable files live. Btrfs protects them.

**Btrfs checksums catch what ext4 misses:**
```
ext4:   bit flips silently → file is corrupt → you find out later
Btrfs:  bit flips → checksum mismatch at read time → you find out immediately
        with RAID1: repairs automatically from second copy
        with scrub: finds corruption before you ever read the file
```

### 3.4 Honest Filesystem Structure

No symlinks. No union mounts. No duplication. One location for every type of file. What you see is what's there.

```
/system/          SLOT_A — EROFS, mounted read-only
    bin/          ALL system executables. One location.
    lib/          ALL system libraries. One location.
    etc/          Read-only system default configs
    share/        Static system data

/config/          CONFIG partition — ext4, mounted read-write
    system/       System-level user overrides
        etc/      Overrides for /system/etc defaults
        services/ Custom service configurations
    apps/         Per-app user configuration
    strat/        Strat OS component configs
        panel.conf
        wm.conf
        terminal.conf
        slots.conf
    user/         User dotfiles — SSH keys, GPG, shell history

/home/            HOME partition — Btrfs, mounted read-write
    [username]/   User data

/run/             tmpfs — RAM only, gone on reboot
/apps/            STRAT_CACHE — .strat packages, sandboxed
/var/             Bind mount from /config/var/
/usr/             Bind mount of /system — legacy compatibility, no symlink
```

**The /usr bind mount:**
Legacy software hardcodes paths like `/usr/bin/bash` and `/usr/lib/libfoo.so`. Rather than symlinks, Strat uses a single bind mount at initramfs time:

```
mount --bind /system /usr
```

One line. Real mount. No symlink. No indirection. `/usr/bin/bash` resolves. Scripts work. The filesystem stays honest.

**strat-build rewrites shebangs at compile time** for .strat packages — `#!/bin/bash` becomes `#!/system/bin/bash`. No runtime hacks needed for native packages.

### 3.5 Config Priority Stack

No merging. No symlinks. Strict priority. First match wins.

```
1. /config/apps/appname/     user customized — always wins
2. /system/etc/appname/      system defaults — fallback
3. App built-in defaults     last resort
```

### 3.6 No Symlinks Policy

Symlinks in a filesystem are almost always a sign of something that should have been designed differently. Strat is built from scratch. It is designed correctly.

```
Forbidden:   ln -s /system/bin /bin
             ln -s /system/lib /lib
             ln -s /system/lib /lib64
             Any symlink that aliases one path to another

Acceptable:  mount --bind /system /usr    (bind mount, real path)
             mount --bind /config/var /var (bind mount, real path)
```

---

## 4. BOOT SYSTEM

### 4.1 StratBoot — Custom UEFI Bootloader

Not GRUB. Not systemd-boot. Not limine. Ours.

Written in C against the UEFI specification using GNU-EFI. GOP (Graphics Output Protocol) for framebuffer rendering. Slot health flags stored in EFI variables. Lives in the ESP — outside and above every partition it manages.

**Why custom is non-negotiable:**

The bootloader is the only component that runs at EFI privilege level before any partition is mounted. At this level it has direct hardware access, can read and write any block device freely, and operates before the OS security model exists. This is the only safe location for partition-level operations — wipes, reflashes, resets — because nothing is mounted, nothing has open file handles, and nothing can fight the operation mid-execution.

GRUB and systemd-boot cannot do this. They hand off to the OS and trust it to manage itself. StratBoot owns the partition operations. The bootloader is sovereign.

**Estimated size:** ~1,500 lines of C. Clean, auditable, entirely ours.

**Responsibilities:**
- Read/write slot health flags in EFI variables
- Read/write partition reset flags in EFI variables
- Execute pending partition operations before any mount
- Select correct boot slot
- Render boot screen and all menus via GOP framebuffer
- Handle ESC interrupt and recovery menus
- Hand off to kernel + initramfs

### 4.2 EFI Variable Schema

```
STRAT_SLOT_A_STATUS   uint8   0=staging 1=confirmed 2=bad
STRAT_SLOT_B_STATUS   uint8   0=staging 1=confirmed 2=bad 3=pinned
STRAT_SLOT_C_STATUS   uint8   0=staging 1=confirmed 2=bad
STRAT_ACTIVE_SLOT     uint8   0=A 1=B 2=C
STRAT_PINNED_SLOT     uint8   0=none 1=A 2=B 3=C
STRAT_RESET_FLAGS     uint8   bitmask: bit0=config bit1=home bit2=system bit3=factory
STRAT_BOOT_COUNT      uint8   increments each boot, reset to 0 on confirmed
STRAT_LAST_GOOD_SLOT  uint8   last confirmed-good slot
STRAT_HOME_STATUS     uint8   0=healthy 1=degraded 2=corrupt
```

EFI variables are the source of truth for slot state. They survive partition wipes. They survive factory resets. They persist across every operation the bootloader can perform. Nothing else is trusted for slot state.

### 4.3 Normal Boot Flow

```
Power on
    ↓
StratBoot reads STRAT_RESET_FLAGS
Execute any pending partition operations (wipe/reflash/reset)
Clear reset flags
    ↓
Read STRAT_HOME_STATUS — if degraded/corrupt → home corruption screen
    ↓
Clean dark screen
Centered Strat mark — logo ring animation
Subtle spinner beneath
"STRAT OS" wordmark
Nothing else.
    ↓
Boot validation passes silently
    ↓
Login / desktop
```

No text. No progress bars. No verbose kernel messages. The mark and the spinner. That's it.

### 4.4 ESC Interrupt Menu

```
[Screen fades. Spinner stops.]

  Hey. You interrupted boot.
  No worries. What do you need?

  ▶ Boot normally
    Continue booting Slot A — v1.3.1

  📍 Boot pinned image                          pinned
    Slot B — v1.2.4  ·  pinned 3 weeks ago

  ─────────────────────────────────────────

  🛡 Safe mode                                  minimal
    TTY only. Filesystem tools. Network up.

  🛟 Recovery options                  destructive inside
    Reset CONFIG, wipe HOME, factory reset, reflash system.

  ─────────────────────────────────────────

  ⚙  UEFI settings
  ↺  Reboot
  ⏻  Power off

  ↑↓ navigate   Enter select   Esc back
```

Keyboard navigable. Rendered via GOP framebuffer. No terminal aesthetic.

### 4.5 Recovery Options Menu

Accessible from ESC menu. All operations execute before any partition mounts.

```
  Recovery Options

  [ Reset CONFIG to defaults              ]
    Settings return to factory defaults.
    System and home data untouched.

  [ Wipe HOME                             ]
    All personal files deleted.
    System and config untouched.
    Requires CONFIRM. Cannot be undone.

  [ Reset CONFIG + Wipe HOME              ]
    Full user reset. System untouched.
    Boots to fresh user experience.
    Requires CONFIRM. Cannot be undone.

  [ Reflash system from pinned slot       ]
    Overwrites current system with pinned image.
    CONFIG and HOME untouched.

  [ Factory reset — everything            ]
    Wipes CONFIG and HOME.
    Reflashes system from pinned slot.
    Returns to first-boot state.
    Requires CONFIRM. Absolutely cannot be undone.

  [ Cancel                                ]
```

**CONFIRM screen for destructive actions:**
```
  ⚠  Wipe HOME

  Everything in your home folder will be deleted.
  Documents, photos, downloads — gone.
  System and settings are completely untouched.

  This cannot be undone.

  Type CONFIRM to proceed:
  [ ____________ ]

  [ Cancel ]
```

Type the word. Not click a button. Intentional friction for irreversible actions. Consistent across bootloader, settings, installer, and recovery terminal.

### 4.6 Safe Mode

```
Minimal init
No user services
No compositor
TTY only
Full filesystem access
Network up
Recovery tools available
```

Enough to fix something or pull a new image. Not a crippled embarrassment.

### 4.7 Boot Validation

systemd oneshot service. Runs in `sysinit.target` before user services. 30 second window.

**Checks:**
- /system mounted and read-only
- /config mounted and read-write
- /home mounted and accessible
- Critical system binaries present
- Strat WM binary present
- Network subsystem available

**Pass:** writes `STRAT_SLOT_X_STATUS = confirmed` to EFI variable. Silent.
**Fail:** writes `STRAT_SLOT_X_STATUS = bad`, reboots to fallback slot. Silent.

---

## 5. SLOT MANAGEMENT & PINNING

### 5.1 Pin System

The pin is sacred.

- User can pin any confirmed-good slot
- Pinned slot: **NEVER DELETED. EVER. FOR ANY REASON.**
- Supervisor cannot delete a pinned slot
- Update engine routes around pinned slots automatically
- Only the user can unpin
- Bootloader always offers pinned slot in interrupt menu with version and pin date
- Pinned slot is the source for system reflash in recovery

**If all slots pinned:**
```
"You've pinned everything.
 Unpin a slot to accept updates."

 [ Manage pins ]    [ Dismiss ]
```

### 5.2 Update Routing Logic

```
SLOT_A running + SLOT_B pinned  →  write update to SLOT_C
SLOT_A running + no pins        →  write update to SLOT_B
All slots pinned                →  notify user, do nothing
```

---

## 6. UPDATE SYSTEM

### 6.1 The Supervisor

Single statically-linked Rust binary. Zero runtime dependencies. Zero shared libraries. Lives in the ESP — neither system slot can touch it.

**Lifecycle:**
```
Normal operation:
    Dormant. Zero RAM. Zero CPU. Does not exist to the user.

User initiates update:
    Wakes.
    Downloads new system image to staging slot silently.
    User keeps working. Nothing changes.

Download complete:
    One notification: "Update ready. Switch when you want."
    Goes silent. Waits indefinitely.

User initiates switch:
    Executes live pivot.
    Health check runs silently.
    Pass → parks old slot, returns to dormant.
    Fail → pivots back, one notification, returns to dormant.
```

Never automatic. Never nags. Never prompts unprompted. If the user wants to update, they will.

### 6.2 Live Pivot

```
User confirms switch
    ↓
Screen fades to dark
Strat WM overlay — input consumed and discarded
Message:
    "Switching systems. Don't touch anything."
    "Back in about 5 minutes."
    "If you're in a hurry, bad timing."
    ↓
CRIU checkpoints running processes
pivot_root to new slot
Health check — 30 second window
    ↓
Pass:
    CRIU restores processes
    Fade back in
    Exactly where you were. No fanfare.

Fail:
    pivot_root back
    CRIU restores processes
    Fade back in
    "Something went wrong. You're back on stable."
```

### 6.3 Error Reporting

```
┌─────────────────────────────────────────┐
│  The update didn't apply.               │
│  You're back on your stable system.     │
│  Nothing is broken.                     │
│                                         │
│  [ Copy error to clipboard ]            │
│  [ View raw log ]                       │
│  [ Dismiss ]                            │
└─────────────────────────────────────────┘
```

**Clipboard format:**
```
[StratOS] Update failed — YYYY-MM-DD HH:MM

What happened:
[One plain English sentence]

Technical detail:
[Relevant log lines only — curated, not raw journald]

Paste this into Claude, Gemini, or ChatGPT for help.
```

AI-assisted debugging is a first-class OS feature.

---

## 7. RAM & PERFORMANCE

### 7.1 Memory Management Stack

```
ZRAM          50% of RAM — zstd compression — multiplies effective memory
ZSWAP         20% of RAM — zstd — compressed swap cache
uksmd         Userspace KSM — merges identical pages across processes
earlyoom      Kills largest non-system process at 10% free RAM — before kernel panics
cgroups v2    Hard per-app memory limits — one app cannot starve everything
```

### 7.2 Never Reboot Design

```
livepatch     → 95% of kernel updates applied live, zero downtime
kexec + CRIU  → Major kernel versions, ~3 second transition, processes restored
ZRAM/ZSWAP    → RAM pressure handled gracefully, no OOM panics
cgroups v2    → No single process craters the system
earlyoom      → Graceful degradation instead of hard crash
```

---

## 8. PACKAGE SYSTEM — .strat

### 8.1 Philosophy

Dependencies live **inside** the .strat. Always. No shared library version conflicts. No dependency hell. Ever.

```
Traditional:
    app needs libfoo 2.3
    system has libfoo 2.1
    everything breaks
    user cries

.strat:
    app needs libfoo 2.3
    libfoo 2.3 is IN the .strat
    system is irrelevant
    nothing breaks
```

### 8.2 Source First Policy

Strat OS is opinionated about where software comes from.

```
.strat from source   BEST
    You compiled it on your hardware
    Optimized with -march=native -O2
    You know exactly what's in it
    Signed with your key
    Dependencies bundled, verified

Flatpak              GOOD — default for most users
    Sandboxed via bubblewrap
    Community governed, Flathub
    No central authority
    Doesn't touch host filesystem

AppImage             ACCEPTABLE
    Self-contained
    Drag, drop, run
    Verify the signature

Raw binary           LAST RESORT
    Advanced mode only
    You're on your own

Snap                 NOT SUPPORTED
    See section 8.7
```

**The two user models:**
```
Squirrel    Opens app store. Clicks install. Flatpak downloads. Done.
            Never thinks about compilers. It just works.

Power user  Opens terminal. strat-build install firefox.
            Makes coffee. Returns to hardware-optimized binary.
            Faster. Smaller. Fully auditable. Signed.
```

Strat never gets in either user's way. The surface is always clean. Power is always one level deeper.

### 8.3 strat-build Pipeline

```
User points at source (git repo, tarball, URL, or package name)
    ↓
strat-build reads source
Detects language (C, Rust, Go, Python, etc.)
Resolves all dependencies transitively
Downloads all dependency sources
    ↓
Compiles with native CPU flags:
    -march=native -O2           (C/C++)
    RUSTFLAGS="-C target-cpu=native"  (Rust)
    ↓
Links statically where possible
Bundles remainder
Rewrites shebangs (#!/bin/bash → #!/system/bin/bash)
Signs output with user's GPG key
    ↓
.strat — single self-contained binary
```

### 8.4 Build Manifest

```toml
[package]
name = "myapp"
version = "1.0.0"
source = "https://github.com/user/myapp"

[build]
lang = "rust"
optimize = "native"

[deps]
# strat-build resolves automatically
# override only if needed
```

### 8.5 Build Cache

STRAT_CACHE partition (XFS). Dependencies compiled once, cached by hash forever. libfoo compiled for one app is reused by every subsequent app that needs it. Never compiled twice.

Community cache (future): verified signed build artifacts from trusted sources. You verify the artifact matches what building from source would produce. Then use the cache.

### 8.6 Honest Compile Time

For large apps, strat-build shows an estimate and offers alternatives:

```
strat-build install libreoffice

Building LibreOffice from source will take approximately
90 minutes on your hardware.

[ Build anyway       ]
[ Flatpak — 3 min   ]
[ Cancel             ]
```

No judgment. User decides.

### 8.7 Snap — Not Supported

```
Snap is not included in Strat OS.
Snap is not in the strat-build package index.
Snap does not appear in the app store UI.
snapd does not run on Strat OS.

Reasons:
    snapd is always running — eating RAM and CPU constantly
    Snap Store is centralized and closed — Canonical controls it entirely
    Cannot run your own Snap store without Canonical's permission
    Mounts a squashfs loop device per snap — clutters mount table
    Phoning home to Canonical infrastructure — no opt-out

If you want snapd:
    Open recovery terminal
    Build it or find the binary
    Install it yourself
    It's your machine. Strat won't stop you.
    Strat won't help you either.
```

### 8.8 Permissions

```
App requests on first use only:
"This app wants access to your Documents folder."
[ Allow ]  [ Deny ]  [ Allow this once ]

Never asked again unless the app requests something new.
No wall of permissions on install that nobody reads.
```

### 8.9 Supported Formats — Final List

| Format | Support level | Default install path |
|---|---|---|
| .strat | Native, first class | strat-build |
| Flatpak | First class | App store default |
| AppImage | Supported | Drag, drop, run |
| Proton/Wine | Supported | For Windows apps/games |
| Raw binary | Advanced mode | /usr bind mount handles it |
| Snap | Not supported | User installable manually |

---

## 9. DESKTOP ENVIRONMENT — Strat WM

### 9.1 Compositor

**Strat WM** — built on wlroots directly. Not Sway. Not Hyprland. Ours.

Sway is rigid. Its configuration file is its UI. It was never designed for what Strat needs — per-window decoration control, live tiling/floating toggle from a settings UI, Cover Flow, panel with blur and auto-hide, SPOTLITE overlay rendering, and a clean IPC socket. Building on wlroots directly gives full control.

Wayland native. XWayland available for legacy apps. Not a core dependency.

**IPC socket at `/run/stratvm.sock`:**
```
settings app  →  "set panel autohide true"     → instant, no restart
SPOTLITE      →  "trigger_coverflow"           → instant
supervisor    →  "trigger_pivot_overlay show"  → instant
terminal      →  "float window 4"              → instant
```

No config file restart cycle. Ever.

### 9.2 Tiling & Windowing

Default: **Tiling**

Per-window override — right-click titlebar:
```
[ Float this window    ]
[ Maximize             ]
[ Move to workspace    ]
[ Always on top        ]
[ Close                ]
```

Global DE toggle in panel:
```
[ Tiling ▼ ] → [ Floating ▼ ]
```

Instant. Live. No restart. No config editing required.

**Tabbed mode:** Super+Shift+W stacks multiple windows behind one tile with clickable tab headers.

### 9.3 Window Decorations

Default: visible titlebar with close, minimize, fullscreen buttons. Buttons exist for new users. Power users remove them via right-click. Keyboard shortcuts replace them.

Right-click titlebar:
```
[ Float this window    ]
[ Remove titlebar      ]
[ Remove buttons       ]
[ Remove borders       ]
[ Restore defaults     ]
[ Apply to all windows ]
```

Live. Immediate. Reversible. No restart.

**Decoration settings (in Settings app):**
```
Corner radius:   [ slider 0–12px ]
Border width:    [ slider 0–4px  ]
Button style:    [ Minimal ] [ Round ] [ Square ]
Button position: [ Left ] [ Right ]
Color:           Follows system accent
```

### 9.4 Keybinds

```
Super + W              Close window
Super + M              Minimize
Super + F              Fullscreen
Super + Shift + Space  Float ↔ Tile toggle
Super + Shift + W      Tabbed mode
Super + Tab            Cover Flow forward
Super + Shift + Tab    Cover Flow reverse
Super + Space          SPOTLITE
Super + `              Panel summon / dismiss
Super + 1/2/3...       Switch workspace
Super + Arrow          Move focus
Super + Shift + Arrow  Move window
```

### 9.5 Cover Flow — Super + Tab

```
        ╔═══════════╗
  ▓▓▓▓▓ ║           ║ ▓▓▓▓▓
  ▓ 2 ▓ ║   App 3   ║ ▓ 4 ▓
  ▓▓▓▓▓ ║  (focus)  ║ ▓▓▓▓▓
        ╚═══════════╝

  Terminal  Chromium  OnlyOffice
```

- Live window textures via wlroots capture
- Perspective transform via OpenGL/Vulkan
- 200ms smooth animation, cubic-bezier easing
- Center: full, sharp, focused
- Sides: receding, dimmed, perspective-tilted
- App name beneath each
- Release Super → focus selected window
- Configurable: all workspaces or current only

### 9.6 Workspaces

```
[ 1 ][ 2 ][ 3 ][ + ]

Click + → new workspace
Drag window to number → moves it
Each workspace has independent tiling layout
```

---

## 10. PANEL

### 10.1 Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  🌐 📦 📄 🎵 🎮 >>>  |  [ 1 ][ 2 ][ 3 ][ + ]  |  ▲ 📶 🔊 🔋 4:32│
└──────────────────────────────────────────────────────────────────┘
Left:    Pinned app launchers — scrollable, infinite, no overflow UI
Center:  Workspace switcher
Right:   System tray — updates, network, volume, battery, clock
```

Position: top. Always.

### 10.2 Launcher Bar — Scrollable

Infinite scroll. No overflow menus. No second row. No arrows. Just scroll.

```
Trackpad:    Two-finger swipe left/right while hovering
Scroll wheel: Scroll up/down while hovering
Touch:        Swipe directly
Momentum:     Yes — flick and it coasts naturally
```

Pinned apps anchor left. Scroll right → recently opened → all installed apps. One continuous strip. No folders. No categories.

Right-click any launcher item:
```
[ Pin to launcher ]  or  [ Unpin ]
[ Open                ]
```

### 10.3 Auto-hide

```
Mouse leaves top of screen → panel slides up (150ms)
Mouse hits top edge        → panel slides down
Super + `                  → panel toggle
```

### 10.4 Tray Interactions

Hover and scroll. No clicking required for quick adjustments.

| Item | Hover shows | Scroll does |
|---|---|---|
| 🔊 Volume | Current % | Adjust volume |
| 📶 Network | Connected network | Cycle saved networks |
| 🔋 Battery | % + time remaining | — |
| 🔆 Brightness | Current % | Adjust brightness |
| 🕐 Clock | Full date + next event | — |

Click any tray item → full detail panel.

### 10.5 Config

Config file: `/config/strat/panel.conf`

```toml
[panel]
position = "top"
autohide = true
summon_key = "super+grave"
size = 28
opacity = 0.85
blur = true

[clock]
format = "12hr"
show_date = false

[pinned]
apps = ["chromium", "onlyoffice", "strat-terminal", "vlc"]

[tray]
show_network = true
show_volume = true
show_updates = true
show_battery = true
```

Settings UI and config file are always in sync. Edit either. Both work.

---

## 11. SPOTLITE

### 11.1 Philosophy

SPOTLITE finds things on your computer. That is its entire job.

```
No web results. Ever.
No ads. Ever.
No store promotions. Ever.
No telemetry. What you search stays on your machine. Period.
```

Windows Search forgot this in 2015. SPOTLITE never will.

### 11.2 Invocation

Super + Space. Instant. No lag. No loading spinner.

### 11.3 Index Coverage

```
Apps              every .strat, flatpak, appimage installed
/home/            everything — documents, images, video, music, code
/config/          your configuration files
Settings          every panel, every control, deep linked
Commands          plain English → real command mappings
Bookmarks         Chromium bookmarks, live indexed
Favorites         user-starred items
Recent            last 20 opened files/apps
Email             subject, sender, body preview, attachments
Calendar          events, dates, locations, people
Live system info  RAM, CPU, storage, uptime — answered inline
Math              calculated inline
Unit conversion   answered inline
```

### 11.4 Indexer

```
Dormant background service
Indexes on events only — never full rescans:
    File created/modified → inotify on /home/ and /config/
    App installed         → package manager notification
    Bookmark added        → Chromium hook
    Setting changed       → settings daemon notification
    Email received        → mail daemon notification
    Calendar updated      → calendar daemon notification

Index: SQLite FTS5 in STRAT_CACHE
Always current. Never stale. Tiny RAM footprint at idle.
```

### 11.5 Result Types & Actions

| Type | Enter action |
|---|---|
| App | Launch |
| File | Open in default app |
| Folder | Open in Strat Terminal |
| Setting | Deep link to exact control |
| Bookmark | Open in Chromium |
| Email | Open in mail client |
| Calendar event | Open in calendar |
| Command | Execute (confirm if destructive) |
| Math/conversion | Display answer inline |
| Live info | Expand inline |

### 11.6 Division of Responsibility

```
Panel launcher    browse and open     "I know roughly where it is"
SPOTLITE          find precisely      "I know exactly what I want"
```

Two surfaces. Zero overlap.

---

## 12. SETTINGS

### 12.1 Philosophy

Search is the front door. The icon grid is the map. The user never needs to know where something lives.

### 12.2 Structure

```
🔍  Search settings...

── System ─────────────────────────────
🖥 Display   🔊 Sound   📶 Network
🔋 Power    ⌨️  Input    🖨 Print

── Appearance ──────────────────────────
🎨 Theme    🪟 Windows   🔤 Fonts

── Apps & Files ────────────────────────
📦 Software   🗂 Default Apps

── Updates & Security ──────────────────
🔄 Updates   🔒 Security   📍 Slots

── Recovery ────────────────────────────
🛟 Reset Options

── About ───────────────────────────────
ℹ️  System Info
```

### 12.3 Behavior

**Search:** type → results instant → each result is a direct deep link to the exact control. Highlights the control, not just the category.

**Scroll:** top = icon grid prominent (Leopard-style). Scroll down = grid condenses to grouped list, search bar pins to top. Scroll up = grid expands back. The page breathes.

**Individual panels:** one back button, one clear title, sliders over dropdowns where possible, plain English labels.

### 12.4 Recovery Panel

Non-destructive options execute immediately. Destructive options set an EFI flag and require reboot — bootloader executes them before any partition mounts.

```
Reset CONFIG to defaults
    Executes immediately. System and home untouched.
    [ Reset CONFIG ]

Wipe HOME
    Sets EFI flag. Requires reboot. Cannot be undone.
    CONFIRM required.
    [ Schedule HOME wipe ]

Factory reset
    Sets EFI flags. Requires reboot. Cannot be undone.
    CONFIRM required.
    [ Schedule factory reset ]
```

---

## 13. STRAT TERMINAL

### 13.1 Philosophy

The shell is a tool. Not a flex. Not a rite of passage. It should be the most powerful and the most approachable thing on the system simultaneously. Both users — the squirrel and the power user — live here comfortably.

### 13.2 Default Interface

```
StratOS Command Line
────────────────────────────────────────
Working in: /home/Dave/Documents/Projects/

📁 ..  (go up)
📁 StratOS/
📁 Archive/
📄 notes.txt
📄 budget.xlsx
────────────────────────────────────────
[ Help ]  [ Docs ]  [ User Guide ]

> _
```

### 13.3 File Browser

| Target | Single click | Double click |
|---|---|---|
| 📁 Folder | Preview inline | Navigate into it |
| 📄 File | Preview first lines | Open in default app |
| 📄 Script | Show what it does | Run it |
| 📄 Config | Plain English summary | Edit it |

### 13.4 Breadcrumb

```
Working in: /home/ Dave/ Documents/ Projects/
                    ↑       ↑          ↑
                 all segments clickable
```

`cd ../../` never needs to be typed again.

### 13.5 Tree View

Toggle flat/tree with one click. Expand inline. No flags.

### 13.6 Ghost Completion

The terminal whispers the rest of the command as you type.

```
User types:    cd down
Ghost:         cd Downloads/
               ──────────↑ dimmed, not typed yet

Tab or →       accepts ghost
ESC            dismisses ghost
Keep typing    ghost updates live
```

**Ranking:**
1. Most recently visited match (frecency)
2. Most frequently visited match
3. Closest string match in current directory
4. Closest match anywhere in /home/
5. System paths

**Case insensitive always. Abbreviation matching:**
```
cd dl    →  Downloads/      (abbreviation)
cd DOWN  →  Downloads/      (case insensitive)
```

**Full path ghosting:**
```
cd ~/doc/pro/str  →  cd ~/Documents/Projects/StratOS/
```

**Command ghosting:**
```
git     →  git commit -m "    (last git command you ran)
sudo    →  sudo systemctl restart
```

### 13.7 cd -s — Smart Directory Shortcut

Type the first letter of each directory segment. Strat expands the full path.

```
cd -s dpr
     d = Downloads
     p = Projects
     r = Red
→  ~/Downloads/Projects/Red/
```

Ghost shows expansion before commit. Tab cycles ambiguous alternatives. Frecency determines default.

```
cd -s dp/StratOS/src  →  ~/Downloads/Projects/StratOS/src/
```

Mix abbreviations and exact paths freely.

### 13.8 Help System

```
> help

What do you want to do?

[ Manage files ]      [ Install software ]
[ System info  ]      [ Network          ]
[ Manage updates ]    [ Advanced         ]

Or just type what you want in plain English.
```

### 13.9 Advanced Mode

```
> advanced

Dropping to full shell. You know what you're doing.
Type 'exit' to come back.

$
```

No judgment. No warnings. Full shell.

### 13.10 What's Stripped

Username. Hostname. Git branch. Conda/venv indicators. All of it. Gone by default. Context appears only when useful.

### 13.11 Shell Stack

```
Strat Terminal surface   friendly UI, file browser, ghosting, natural language
fish underneath          autosuggestions, zero config, syntax highlighting
nushell available        structured data pipelines for power users
bash always accessible   POSIX compatibility for scripts
```

---

## 14. DEFAULT APPLICATION STACK

| Role | Application |
|---|---|
| Browser | Ungoogled Chromium |
| Office | OnlyOffice (Community Edition) |
| Terminal | Strat Terminal (ours) |
| Image viewer | Strat Viewer (ours) |
| Video & Audio | VLC |
| Print | CUPS + vendor drivers |
| Text editing | Strat Terminal built-in |
| Email | Native IMAP client |
| Calendar | Native CalDAV client |

### 14.1 Strat Viewer

```
┌──────────────────────────────────────────┐
│  filename.jpg                🗕  🗖  ✕  │
├──────────────────────────────────────────┤
│            [ image here ]               │
├──────────────────────────────────────────┤
│  ◀  ▶  🔍+  🔍-  ⛶   │ Set as BG │ Copy │
└──────────────────────────────────────────┘
```

**Set as BG:** click → inline overlay on image → Fill / Fit / Center / Tile → done. No settings app.
**Fullscreen:** clean, no chrome, toolbar fades in on mouse move, fades out on stop. ESC exits.
**Copy:** full image to clipboard. Paste anywhere.

Estimated: ~700 lines of C or Rust.

---

## 15. HOME CORRUPTION DETECTION & RECOVERY

### 15.1 Detection

Boot validation checks /home mount integrity before desktop loads. On failure — journal error, inode corruption, Btrfs checksum failures, dirty bit — boot halts and the corruption screen is presented. The user never reaches a broken desktop.

### 15.2 Proactive Detection via Btrfs

```
Btrfs checksums every block on write and read.
Silent corruption → checksum mismatch at read time → logged immediately.
Monthly scrub → walks every block proactively → finds corruption before you read the file.

Scrub result:
    Clean  → silent, nothing shown
    Errors → one notification:
             "Filesystem check found issues in /home.
              [ View details ]  [ Copy report ]  [ Dismiss ]"
```

Early warning before boot failure. Before data loss. Built into the filesystem.

### 15.3 Btrfs Snapshots

```
Before any HOME wipe:
    btrfs subvolume snapshot /home /home/.strat-snapshots/pre-wipe-[timestamp]
    Takes seconds. Costs almost no space (CoW).

Retention: 24 hours, then auto-deleted.

Recovery screen offers: "We took a snapshot before wiping.
                         Files may be recoverable for 24 hours."
[ Restore from snapshot ]
```

A permanent destructive action becomes a recoverable one.

### 15.4 Home Corruption Screen

Full screen. No desktop. No partial boot. The entire display is the recovery UI.

```
[pulsing amber dot]  Strat OS — Boot Alert — Home Partition

Hey.

So this sucks, but your home directory is gone.

Whatever happened last time corrupted your user data.
Your system is completely fine. Your configs are completely fine.
But your home partition? We can't mount it cleanly.

[error box — exact filesystem error message]

Your options:

[ Attempt Boot          safe        ]
  Maybe the OS just needs another shot.
  Journal errors sometimes clear on a clean mount.
  Can't hurt. Nothing changes.

[ Reset / Wipe Home     destructive ]
  Wipe home clean. Your configs survive this.
  Settings, prefs, SSH keys — still there.
  Your files are gone. CONFIRM required.

[ Attempt Hard Recovery  technical  ]
  We'll dig in and try to fix it.
  If we can't — we'll tell you exactly what's fucked
  and give you a clipboard report for AI-assisted help.

[ Open Recovery Terminal  you know what you're doing ]
  Full shell. fsck, btrfs, testdisk, photorec, smartctl.
  /system and /config read-only.
  /home mounted degraded.
  No hand holding.

System partition: healthy  ·  Config partition: healthy
This is isolated to /home only.
```

### 15.5 Recovery Terminal

```
Strat OS Recovery Shell — v1.3.1
/home mounted degraded — proceed with caution
─────────────────────────────────────────
Tools: fsck  e2fsck  btrfs  debugfs  mount  dd
       rsync  tar  smartctl  testdisk  photorec
─────────────────────────────────────────
Mounts:
  /system   → sda2   ro   healthy
  /config   → sda4   ro   healthy
  /home     → sda5   rw   degraded
─────────────────────────────────────────
Type 'exit' to return to recovery menu.

# _
```

**The one guardrail:**
```
# mkfs.ext4 /dev/sda5

⚠ This will destroy /home completely.
  Type CONFIRM to proceed, or Ctrl+C to cancel:
```

Everything else — no guardrails. You opened the terminal. You know what you're doing.

---

## 16. AI BUILD PIPELINE

### 16.1 Overview

```
Codex (code generation)
    ↓
QEMU automated boot + test run
    ↓
Opus (reasoning / spec validation)
    ↓
loop
```

### 16.2 Applications

- StratBoot slot logic and reset operations
- Strat WM compositor subsystems
- strat-build dependency resolution
- Supervisor pivot orchestration
- Boot validation service
- System image spec validation
- Crash log curation

### 16.3 Ground Truth

QEMU with serial output piped to text. No component ships without a passing QEMU boot run. Automated regression suite. If QEMU doesn't boot, the code is wrong.

---

## 17. LIVE USB — INSTALLER & DIAGNOSTIC

### 17.1 Philosophy

The live USB is not just an installer. It is a diagnostic and repair tool. It boots outside the installed system and has full access to every partition before any of them are mounted. This is the same EFI privilege advantage that StratBoot uses — direct hardware access, no OS security model to navigate, clean sequential block operations.

### 17.2 Boot Detection

StratBoot on the live USB scans for existing Strat OS installations:

```
Blank disk or non-Strat disk  →  go straight to installer
Existing Strat install found  →  show choice:
                                 [ Install fresh    ]
                                 [ Diagnose & repair ]
```

### 17.3 Installer Flow

```
Select target disk
    ↓
Disk info shown (model, size, serial)
CONFIRM required before any write
    ↓
Partition and format disk
Write SLOT_A (current) and SLOT_B (LTS, auto-pinned)
Initialize CONFIG partition
Initialize HOME (Btrfs subvolume)
Install StratBoot to ESP
Set EFI variables
    ↓
Done. Remove USB. Reboot. Strat OS boots.
```

### 17.4 Diagnostic Report

When an existing install is detected, StratBoot reads EFI variables and partition state before presenting the diagnostic screen:

```
We found a Strat OS install on /dev/nvme0n1.

  ESP           ✓ healthy
  SLOT_A        ✗ bad flag — failed boot validation
  SLOT_B        ✓ confirmed — v1.2.4 (pinned)
  SLOT_C        ○ empty
  CONFIG        ✓ readable
  HOME          ✗ journal checksum error — degraded

Pinned slot is healthy. Config is intact. Home is damaged.
We can fix this.

[ Auto-repair      ]  let Strat figure it out
[ Walk me through  ]  show me each step
[ Manual terminal  ]  open diagnostic shell
[ Fresh install    ]  choose what to preserve
```

### 17.5 Auto-repair Logic

```
SLOT_A bad + SLOT_B healthy
    → set SLOT_B active, clear SLOT_A bad flag

HOME degraded (journal error)
    → run e2fsck / btrfs check
    → repairable → fix, verify, report
    → not repairable → offer snapshot restore or wipe

HOME corrupt (Btrfs checksum failures)
    → run btrfs scrub
    → repairable → fix and verify
    → not repairable → offer snapshot restore or wipe

CONFIG corrupt
    → restore from /system/etc defaults
    → report what was reset

ESP corrupt
    → reinstall StratBoot from live USB
    → reinitialize EFI variables
    → verify boot

All slots bad, no pinned slot
    → cannot auto-repair
    → escalate to walk-through or manual terminal
```

### 17.6 Walk Me Through Mode

One issue per screen. Plain English. One action per step.

```
Step 1 of 3

SLOT_A has a bad flag.
It failed boot validation last time it was used.

SLOT_B is healthy and pinned.
We can set SLOT_B as your active boot slot.
Your configs and home data are untouched.

[ Do it ]  [ Skip ]  [ Explain more ]
```

### 17.7 Live Diagnostic Terminal

All detected partitions pre-mounted at labelled paths:

```
/mnt/esp      ESP
/mnt/slota    SLOT_A  (ro)
/mnt/slotb    SLOT_B  (ro, pinned)
/mnt/slotc    SLOT_C  (ro)
/mnt/config   CONFIG  (ro)
/mnt/home     HOME    (degraded)
```

Full tool suite. Same CONFIRM guard on mkfs commands. `exit` returns to diagnostic menu.

### 17.8 Fresh Install Options From Diagnostic Mode

```
[ Preserve CONFIG + HOME, reinstall system only ]
    Fastest. System broken, data and settings fine.
    Only SLOT_A, SLOT_B, SLOT_C, ESP touched.

[ Preserve CONFIG, wipe HOME, reinstall system  ]
    Settings survive. Files gone.
    Btrfs snapshot taken before HOME wipe.

[ Full fresh install — wipe everything          ]
    Clean slate. 5 minutes. Done.
    CONFIRM required.
```

The system-only reinstall is unique to Strat OS. No other OS can offer it because no other OS separates system, config, and data this cleanly.

### 17.9 ISO Build

Built with **mkosi**. Target size under 2GB. Ventoy compatible.

```
Contents:
    StratBoot
    Minimal Linux kernel
    Strat Installer binary
    Strat Diagnostic binary
    Recovery tool suite (fsck, btrfs, e2fsck, testdisk, photorec, smartctl)
    Nothing else
```

---

## 18. VISUAL IDENTITY

*To be defined — requires: wordmark, system typeface, accent color, icon language, boot screen mark.*

**Tone:** Calm. Confident. The OS knows what it's doing and doesn't need to prove it. Clean without being cold. Modern without being corporate. Nothing macOS. Nothing Windows. Entirely its own thing.

---

## APPENDIX A — What Strat OS Is Not

- Not for legacy BIOS hardware
- Not for machines under 256GB storage
- Not for users who want Windows compatibility as a first-class goal
- Not a hobbyist experiment — a genuine daily driver with a defined architecture
- Not built on symlink forests or union mount tricks — what you see is what's there
- Not something that nags you, prompts you, or decides things for you
- Not Snap-compatible by default — and that's a feature
- Not Aurora, not NixOS, not Fedora Silverblue — though it shares their immutability philosophy

---

## APPENDIX B — The Three Layer Guarantee

```
Destroy SYSTEM  →  CONFIG and HOME survive intact
Destroy CONFIG  →  SYSTEM and HOME survive intact
Destroy HOME    →  SYSTEM and CONFIG survive intact

Destroy any two →  the third survives
Destroy all three → you asked for it explicitly with CONFIRM
```

Every reset operation — from CONFIG wipe to full factory reset — executes in StratBoot before any partition mounts. This is the only safe location. The bootloader is sovereign. It cannot be broken by the state of any partition it manages.

---

## APPENDIX C — Filesystem Summary

| Partition | Filesystem | Why |
|---|---|---|
| ESP | FAT32 | UEFI spec requires it |
| SLOT_A/B/C | EROFS | Immutable by design, compressed, Android-proven |
| CONFIG | ext4 | Stable, boring, perfect for tiny config partition |
| STRAT_CACHE | XFS | Parallel build writes, large files, built for this |
| HOME | Btrfs | Checksums, compression, snapshots, your data deserves it |

---

## APPENDIX D — Key Innovations

1. **Three-layer stratified architecture** — SYSTEM, CONFIG, HOME independently survivable
2. **Bootloader-executed resets** — partition operations before mount, the only safe way
3. **Honest filesystem** — no symlinks, no duplication, one location for everything, /usr bind mount for compat
4. **EROFS system partitions** — immutable at filesystem level, not just mount policy
5. **Btrfs HOME** — checksums catch silent corruption, scrub finds it proactively, snapshots make wipes recoverable
6. **Live rootfs pivot** — update without rebooting, automatic rollback on failure
7. **User-pinned slots** — sacred, untouchable, always bootable, source for system reflash
8. **strat-build** — source-first, compile from scratch, bundle dependencies, hardware-optimized
9. **Source-first philosophy** — squirrels use Flatpak, power users compile, Snap not supported
10. **CONFIG partition** — configuration survives HOME destruction, HOME wipe is surgical not catastrophic
11. **Strat Terminal** — unified file browser and shell, ghost completion, cd -s abbreviation expansion
12. **SPOTLITE** — system-wide search, zero web results, zero ads, zero telemetry
13. **Cover Flow window switcher** — live window previews, smooth perspective animation
14. **Per-window tiling/floating** — no religious commitment required
15. **Strat WM** — wlroots compositor, clean IPC, built for Strat's requirements
16. **AI-assisted error reporting** — clipboard-ready diagnostic output, first-class feature
17. **Home corruption detection** — caught at boot, never reaches broken desktop
18. **Recovery terminal** — full shell access, one guardrail, everything else open
19. **Live USB diagnostic** — boots outside broken system, diagnoses and repairs exact broken layer
20. **System-only reinstall** — replace broken system, preserve data and config, unique to Strat's architecture
21. **Fairphone principle** — average user never sees complexity, power user has full access, nothing hidden

---

*Strat OS — v0.4 Design Document*
*Status: Pre-implementation spec*
*"The system is the bedrock. Home is yours. Touch neither uninvited."*
