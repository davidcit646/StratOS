# StratSup + sysroot — agent brief

## stratsup

- Path: `stratsup/src/`, `stratsup/Cargo.toml`.
- Notable: `efi_vars` module consumed by `stratmon` for runtime EFI writes on **Linux host** (not firmware).

## sysroot

- `sysroot/initramfs-init.c` — initramfs PID1; must stay aligned with `scripts/create-test-disk.sh` GPT layout and `stratboot` `root=` PARTUUID.
- `sysroot/Makefile` — builds auxiliary static binaries for rootfs where applicable.
- `sysroot/first-boot-provision.sh` — provisioning hook from rootfs build.

## Grep

`rg "execv\\(\"/bin/stratman\"|mount_or_die" sysroot`

## Human doc

[../human/stratsup-and-sysroot.md](../human/stratsup-and-sysroot.md)