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
