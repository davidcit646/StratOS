# StratOS

**StratOS** is a custom-first, image-based desktop OS: immutable **EROFS** system slots (**A / B / C**), dedicated **CONFIG / apps / home** partitions, a **UEFI** bootloader (**StratBoot**), minimal **initramfs**, **stratman** as PID 1, and an in-house **Wayland** stack (**stratvm**, **stratpanel**, **stratterm**). Updates are designed around **StratMon** (staging, verify, manifest) and **StratBoot** (pre-boot slot surgery).

**Not** a Fedora/Arch remix: no traditional distro packaging model; the goal is a small, auditable stack you can reason about end-to-end.

---

## Documentation

Start with **[docs/README.md](docs/README.md)** for a full index. The main specs are:

- **[docs/human/stratos-design.md](docs/human/stratos-design.md)** — canonical architecture.
- **[docs/human/coding-checklist.md](docs/human/coding-checklist.md)** — phases and open work.
- **[docs/human/runtime-persistence-contract.md](docs/human/runtime-persistence-contract.md)** — mount and persistence rules.

---

## Build and run (QEMU)

End-to-end flow (kernel in `linux/`, outputs under `out/`):

```bash
./build-all-and-run.sh           # full build + disk update + QEMU
./build-all-and-run.sh -s      # skip kernel rebuild
./build-all-and-run.sh --recreate-disk   # new GPT image (needed after partition layout changes)
```

**Host tools (typical):** `cargo`, `gcc`, `make`, `flex`, `bison`, `cpio`, `gzip`, `mkfs.erofs`, `mkfs.ext4`, `mkfs.btrfs`, `sgdisk`, `losetup`, `kpartx`, `qemu-system-x86_64`, **gnu-efi** (for `stratboot`), **OVMF** vars at repo `ovmf_vars.fd` (see `scripts/run-qemu.sh`).

**Disk helpers** (called from the build script; GPT layout matches stratboot + `sysroot/initramfs-init.c`):

- `scripts/create-test-disk.sh`
- `scripts/update-test-disk.sh`
- `scripts/run-qemu.sh`

Logs: `ide-logs/qemu_strattest.txt`, `ide-logs/qemu-desktop-serial.txt`.

---

## Update path (summary)

- **StratMon** may download, verify, and stage payloads; it must not be the component that performs raw writes into inactive system slots.
- **StratBoot** runs in firmware context, reads EFI state (and manifests on the ESP when implemented), and owns block-level slot updates.

Details and invariants: **docs/human/stratos-design.md** § update / boot sections.

---

## Contributing

1. Use **docs/human/coding-checklist.md** for scope and ordering.
2. Prefer small, reviewable PRs; match existing style; avoid drive-by refactors.
3. Optional multi-agent prompt templates: **[docs/agent/ai-roles.md](docs/agent/ai-roles.md)**.
