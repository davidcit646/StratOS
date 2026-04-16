# README.md

## StratOS: The Custom-First, Atomic Operating System

**StratOS** is a modern, reliable desktop operating system engineered from the ground up for power users who demand transparency, performance, and uncompromising stability. Built on an "honest" filesystem model with an immutable EROFS core, StratOS ensures system integrity through a custom-built, atomic A/B/C update architecture driven by EFI variables. By adhering to a strict **Custom First** philosophy, we reject the bloat of traditional desktop environments in favor of lightweight, purpose-built components—including our own Wayland compositor, terminal emulator, and bootloader. StratOS isn't just a platform; it is a refined tool for those who value a clean, dependency-minimal environment where the system remains exactly as you configured it.

---

## What is StratOS?

StratOS is an independent operating system built on a tuned Linux kernel, designed to bridge the gap between "experimental" hobbyist projects and "bloated" mainstream distributions. 

### What it IS:
* **Custom-First:** We build our own core components (StratTerm, Strat WM, Spotlite) rather than pulling in heavy external stacks.
* **Atomic & Immutable:** The system root is a read-only EROFS image. Updates are applied to inactive slots with instant rollback capability.
* **Structurally Honest:** We enforce a strict separation between `/system` (immutable), `/config` (persistent system settings), and `/user` (personal data).
* **Modern & Secure:** Written primarily in **Rust** for safety-critical user-space and **C** for low-level boot logic.

### What it IS NOT:
* **Not a Linux Distribution:** This is not a remix of Fedora, Ubuntu, or Arch. We do not use traditional package managers like `.deb` or `.rpm`.
* **Not a GNOME/KDE Wrapper:** We explicitly avoid GTK-heavy or Qt-heavy stacks. If you are looking for a traditional desktop environment, this isn't it.
* **Not a Research Toy:** While minimalist, every component is designed for real-world functional use on modern hardware.

---

## Key Features

* **StratBoot:** A custom UEFI bootloader and update surgeon.
* **Strat WM:** A fast, tiling Wayland compositor written from scratch for performance.
* **StratMon:** A secure update conductor that manages A/B/C slot transitions.
* **The .strat Format:** A signed, sandboxed package format designed for the StratOS ecosystem.
* **Spotlite:** An integrated, intelligent system search and launcher.

---

## Core Philosophy: Custom First

The most important rule of StratOS development is that **we build it ourselves.** We believe that pulling in a massive library to solve a small problem is a technical debt that compromises the entire system. 
* **No Excessive Dependencies:** Only add a library if it is truly minimal and cannot be reasonably replaced with custom code.
* **Language Precision:** We favor Rust for its memory safety in user-space and C for the bare-metal simplicity required at the boot level.

---

## Project Status

**Current Phase: Early Pre-Alpha**
The project is currently structured into approximately 20 phases of development. We are currently focusing on the core bootloader logic, the immutable filesystem layout, and the initial Wayland compositor protocols.

---

## How to Build & Test

The primary development target for StratOS is **QEMU** to ensure rapid iteration and safety during low-level development.

1.  **Clone the Repository:** `git clone https://github.com/stratos-project/stratos`
2.  **Toolchain Requirements:** Ensure you have the latest Rust stable and a cross-compilation C toolchain for UEFI.
3.  **Build the Image:** Run `./strat-build.sh` to generate the EROFS system image and ESP partition.
4.  **Run in QEMU:** ```bash
    ./strat-run.sh --qemu --enable-kvm
    ```

---

## How to Contribute

We welcome contributors who share our passion for minimalism and custom architecture.
1.  **Review the Coding Checklist:** Check `StratOS-Coding-Checklist.md` to find incomplete implementation tasks.
2.  **Follow the Rule:** Implement components yourself before suggesting an external dependency.
3.  **Submission:** Submit focused Pull Requests. Ensure all code adheres to the project's design principles.

---

# StratOS: Update Path Architecture (StratMon & StratBoot)

## I. Core Philosophy
StratOS implements a "Custom First" approach to system reliability. We minimize dependencies by building purpose-built components in **Rust** (User-space) and **C** (Low-level boot). The update system is designed to be atomic, immutable, and safe from filesystem-level corruption during the write process.

---

## II. The Conductor: StratMon
**StratMon** is the user-space conductor responsible for the "untrusted" phase of the update.

### Rules & Constraints
- **MAY**: Download update payloads, verify cryptographic signatures, and stage images.
- **MUST NOT**: Write directly to `/system` or any system slot (SLOT_A, SLOT_B, SLOT_C).
- **Format**: Updates are handled as **System images** (EROFS), never as ISOs.

### Implementation Logic (Rust)
1. **Acquisition**: Downloads the **Update payload** into `STRAT_CACHE`.
2. **Verification**: Validates the payload signature and hash.
3. **FIEMAP Mapping**: Since StratBoot lacks an XFS driver, StratMon uses `ioctl(FS_IOC_FIEMAP)` to identify the exact physical disk extents (block ranges) of the staged image file.
4. **Manifest Creation**: Writes a small, bootloader-readable **Update manifest** to the ESP. This file contains the physical block map of the source image.
5. **State Signaling**: Sets EFI variables (e.g., `Strat_Update_Pending`) to signal a requested transition to the **Target slot**.
6. **Trigger**: Initiates a reboot.

---

## III. The Surgeon: StratBoot
**StratBoot** is the low-level surgeon. It owns the hardware and performs the write in a pre-boot environment where the system is quiescent.

### Rules & Constraints
- **Ownership**: Owns the final system slot write.
- **Environment**: Runs before the normal OS mount.
- **Integrity**: Uses EFI variables as the source of truth and refuses unsafe or unverified targets.

### Implementation Logic (C)
1. **Detection**: On boot, checks EFI variables for an update request.
2. **Manifest Read**: Accesses the **Update manifest** from the ESP to find the source blocks in `STRAT_CACHE`.
3. **The Raw Copy**: Uses `EFI_BLOCK_IO_PROTOCOL` to perform a raw copy from the source physical extents directly into the **Target slot** (the inactive, non-pinned slot).
4. **Validation**: Calculates the hash of the bytes during the copy process; compares against the expected hash in the manifest.
5. **Slot Rotation**: 
   - If successful: Marks the **Target slot** as active.
   - If failed: Leaves the **Current slot** untouched and
