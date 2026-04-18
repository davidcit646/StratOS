# StratSup & sysroot (human guide)

## StratSup (`stratsup/`)

Rust crate historically used as a **supervisor**; today it still ships **host-side helpers** (for example **EFI variable access** used by `stratmon`). It is **not** PID 1 on the boot path—that is **stratman**.

## sysroot (`sysroot/`)

C code and makefiles for **early userspace**:

- `**initramfs-init.c`** — minimal PID1 in the initramfs: mount `/system` (EROFS), `/config`, `/apps`, `/home`, bind `/etc`/`/var`, then exec **stratman**.
- `**system-init.c`**, `**first-boot-provision.sh**` — first-boot helpers as referenced from the build scripts.

## Related

- Persistence: [runtime-persistence-contract.md](runtime-persistence-contract.md).
- Agent brief: [../agent/stratsup-sysroot.md](../agent/stratsup-sysroot.md).