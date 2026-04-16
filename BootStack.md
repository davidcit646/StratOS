# StratOS: Triple-Slot Atomic Workstation

StratOS is a custom-engineered, image-based Linux architecture designed for total immutability and maintenance-driven stability. It discards the legacy GNU/Systemd filesystem hierarchy in favor of a Triple-Slot (A/B/C) system image model managed by the `stratboot` loader and `stratman` PID1.

## 1. The Boot & Execution Stack

StratOS operates via a vertical stack of custom-built binaries, ensuring the system state is validated before a single user process executes.

### stratboot (EFI Bootloader)
* The system entry point.
* Interfaces with EFI variables to manage Slot State (Staging, Confirmed, Bad).
* Implements A/B/C logic: Selects the highest-priority "Confirmed" or "Staging" image.

### initramfs (The Assembler)
* Discovers partitions (A, B, C, D, E) via UUID.
* Mounts the selected System Image directly to `/usr`.
* Mounts Partition D (Config) to `/config`.
* Executes `switch_root` into the `stratman` binary located on the System Image.

### stratman (PID 1 Orchestrator)
* Manages system initialization and service spawning.
* **Maintenance Mode**: Monitors user activity and background load to perform "Opportunistic Maintenance" (Library consolidation, integrity checks).
* **Namespace Guard**: Restricts user-space access to core partitions using mount namespaces.

---

## 2. Partition & Filesystem Architecture

StratOS eliminates legacy symlinks (`/bin`, `/lib`, `/etc`). The root scaffold is a minimal VFS mount table where system components are plugged in as discrete, immutable parts.

### Partition Mapping

| Partition | Role | Lifecycle | Mount Point |
| :--- | :--- | :--- | :--- |
| **A / B / C** | System Images | Read-Only (Swappable) | `/usr` |
| **D** | Config Truth | Transactional Read-Only | `/config` |
| **E** | User Data | Read-Write | `/home` |
| **RAM/Disk** | State | Volatile/Ephemeral | `/var` |

### Filesystem Tree

```text
/ (Root Scaffold - Partition X or Tmpfs)
├── usr/        <-- [SYSTEM IMAGE A, B, or C]
│   ├── bin/    # All executables (including stratman)
│   ├── lib/    # Shared libraries (.so files)
│   └── share/  # Static system data
├── config/     <-- [PARTITION D]
│   ├── network/ # Persistent network profiles
│   ├── hw/      # Hardware-specific rules
│   └── services/# Stratman service manifests
├── home/       <-- [PARTITION E]
│   └── user/    # Persistent personal data
└── var/        <-- [STATE]
    ├── log/     # Ephemeral logs
    └── run/     # Sockets and PIDs
