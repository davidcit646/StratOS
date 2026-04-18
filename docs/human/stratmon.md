# StratMon (human guide)

**StratMon** is the **update conductor** in user space: it understands staged **EROFS** images on disk, builds a small **binary manifest** for the bootloader, and flips **EFI variables** to request work on next reboot.

It is **not** allowed to poke raw system-slot partitions directly—that job belongs to **StratBoot** once the machine is back in firmware context.

## Code & CLI

- Crate: `stratmon/`.
- Typical development command: `--stage-update` (see `stratmon/src/main.rs`).

## Related

- Design: [stratos-design.md](stratos-design.md) (update pipeline).
- Checklist Phase **6** and **15** (HTTPS): [coding-checklist.md](coding-checklist.md).
- Agent: [../agent/stratmon.md](../agent/stratmon.md).