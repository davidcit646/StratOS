# StratBoot (human guide)

**StratBoot** is StratOS’s **UEFI bootloader**: it runs before Linux, draws a minimal splash / status UI, reads **EFI variables** for A/B/C slot state, resolves **GPT partition names** (e.g. `SLOT_A`, `CONFIG`), and loads the **kernel + initramfs** from the ESP under `\EFI\STRAT\SLOT_`*.

## What to read next

- Full boot/update rules: [stratos-design.md](stratos-design.md) (boot and StratMon / StratBoot sections).
- Variable names and semantics: [efi-variables.md](efi-variables.md).
- Shorter stack diagram: [boot-stack.md](boot-stack.md).

## How it fits the machine

1. Firmware hands off to `BOOTX64.EFI` (StratBoot) on the ESP.
2. StratBoot picks a slot, builds `root=PARTUUID=…` for the **EROFS** system partition, and starts the Linux image.
3. Linux runs **initramfs** (`sysroot/initramfs-init.c`), which mounts the rest and execs **stratman**.

Disk images for QEMU are built with `scripts/create-test-disk.sh` and populated by `scripts/update-test-disk.sh` (see root [README.md](../../README.md)).

## Agent-oriented companion

For file-level maps and invariants (no UI prose), use [../agent/stratboot.md](../agent/stratboot.md).