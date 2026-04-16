# StratOS

**StratOS is a modern, reliable desktop operating system engineered from the ground up on a "Custom First" philosophy.** Built for power users who demand transparency and absolute control, StratOS rejects the bloat of traditional desktop environments in favor of purpose-built, lightweight components. By leveraging a custom-first Rust and C stack, an atomic A/B/C update system, and an "Honest" immutable filesystem architecture, StratOS provides a rock-solid foundation that is as safe as it is performant.

## What is StratOS?

StratOS is a focused effort to build a cohesive computing environment where every component—from the bootloader to the compositor—is designed to work together without the baggage of excessive dependencies. 

### What it IS:
* **Atomic & Safe:** Utilizing an A/B/C slot system driven by EFI variables for seamless, rollback-capable updates.
* **Honest Filesystem:** A strict separation of concerns with an immutable `/system` (EROFS) and clearly defined paths for config, cache, and user data.
* **Custom-First:** We build our own solutions (e.g., StratWM, StratTerm, Spotlite) rather than pulling in heavy frameworks like GNOME or GTK.
* **Performance-Oriented:** Written in Rust for memory-safe user-space tools and C for low-level boot logic.

### What it IS NOT:
* **Not a Distribution:** StratOS is not a remix of Fedora, Ubuntu, or any existing Linux distro.
* **Not a Wrapper:** It is NOT a shell for GNOME, KDE, or Xfce. 
* **Not a Toy:** While minimalist, every component is built for real-world functionality and reliability.
* **Not Legacy-Bound:** We do not aim for compatibility with `.deb` or `.rpm` ecosystems; we use our own signed, sandboxed `.strat` format.

## Core Philosophy: "Custom First"

We believe that true reliability comes from understanding and owning the stack. 
1.  **Minimize Dependencies:** Only add a dependency when it is truly necessary and cannot be reasonably implemented as custom code.
2.  **Purpose-Built:** We prefer direct Wayland clients and custom GPU text rendering over generic, heavy-duty libraries.
3.  **Language Discipline:** C for the boot-level components (Stratboot); Rust for everything else to ensure memory safety without sacrificing speed.

## Key Features

* **Stratboot:** A custom UEFI bootloader and validation service.
* **Stratsup:** A custom supervisor for system initialization and process management.
* **Strat WM:** A fast, minimalist tiling Wayland compositor written from scratch.
* **StratTerm:** A high-performance terminal emulator with custom GPU-accelerated rendering.
* **Spotlite:** An integrated, intelligent system-wide search and launcher.
* **Atomic Updates:** Rolling back from a failed update is as simple as switching an EFI variable.

## Project Status

StratOS is currently in **Early Pre-Alpha**. The project is divided into approximately 20 development phases, moving from low-level boot protocols to high-level user-space tools. We are currently building out the core system services and the initial compositor logic.

## How to Build & Test

The primary development target for StratOS is **QEMU (x86_64)**. 

1.  **Prerequisites:** Rust (Nightly), `gcc`, `make`, `qemu-system-x86_64`, and `ovmf`.
2.  **Clone the Repo:** `git clone https://github.com/stratos-project/stratos.git`
3.  **Build:** Run the custom build script to compile the kernel, bootloader, and user-space tools.
4.  **Run:** ```bash
    make qemu
    ```
    *Note: This will launch StratOS in a virtualized environment with EFI support.*

## How to Contribute

We welcome contributors who share our passion for minimalism and "Custom First" engineering.

* **Follow the Codex:** All code must adhere to `CODEX_PROTOCOL.md` and the design principles in `StratOS-Design-Doc-v0.4.md`.
* **Pick a Task:** Check the `StratOS-Coding-Checklist.md` for current implementation needs.
* **Custom First:** Before suggesting a new library, evaluate if we can implement the required logic ourselves.
* **Submit PRs:** Keep pull requests focused, atomic, and well-documented.

## Links

* **Design Document:** `docs/StratOS-Design-Doc-v0.4.md`
* **Coding Standards:** `docs/CODEX_PROTOCOL.md`
* **Issue Tracker:** GitHub Issues
