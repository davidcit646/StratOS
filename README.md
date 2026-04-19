# StratOS

**StratOS** is a custom-first, image-based desktop OS: immutable **EROFS** system slots (**A / B / C**), dedicated **CONFIG / apps / home** partitions, a **UEFI** bootloader (**StratBoot**), minimal **initramfs**, **stratman** as PID 1, and an in-house **Wayland** stack (**stratvm**, **stratpanel**, **stratterm**, **stratsettings** for merged UI config and **`stratos-settings`**). Updates are designed around **StratMon** (staging, verify, manifest) and **StratBoot** (pre-boot slot surgery).

**Not** a Fedora/Arch remix: no traditional distro packaging model; the goal is a small, auditable stack you can reason about end-to-end.

---

## Documentation

Start with **[docs/README.md](docs/README.md)** for a full index. The main specs are:

- **[docs/human/stratos-design.md](docs/human/stratos-design.md)** — canonical architecture.
- **[docs/human/coding-checklist.md](docs/human/coding-checklist.md)** — phases and open work.
- **[docs/human/runtime-persistence-contract.md](docs/human/runtime-persistence-contract.md)** — mount and persistence rules.

---

## Build

End-to-end flow (kernel in `linux/`, outputs under `out/`). The script **builds everything and refreshes the GPT test disk image** (`out/phase4/test-disk.img`) for flashing or attaching on **bare metal**.

```bash
./build-all-and-run.sh              # full build + disk update
./build-all-and-run.sh -s           # skip kernel rebuild
./build-all-and-run.sh --recreate-disk   # new GPT image (needed after partition layout changes)
```

**Host tools (typical):** `cargo`, `gcc`, `make`, `flex`, `bison`, `cpio`, `gzip`, `mkfs.erofs`, `mkfs.ext4`, `mkfs.btrfs`, `sgdisk`, `losetup`, `kpartx`, **pkg-config**, **xorriso** + **mtools** (live ISO only).

**Per-component packages (names differ by distro):**

| Area | Fedora / RHEL-style | Debian / Ubuntu-style |
|------|---------------------|------------------------|
| **stratboot** (UEFI) | `gnu-efi-devel` | `gnu-efi` |
| **stratvm** (Wayland) | `wlroots-devel`, `wayland-devel`, `wayland-protocols-devel`, `libxkbcommon-devel`, `pixman-devel`, `libinput-devel`, `libevdev-devel` | `libwlroots-dev`, `libwayland-dev`, `wayland-protocols`, `libxkbcommon-dev`, `libpixman-1-dev`, `libinput-dev`, `libevdev-dev` |
| **sysroot initramfs** | `cpio` (usually `cpio` package) | `cpio` |
| **Kernel build** (`linux/`) | `flex`, `bison`, `openssl-devel`, `elfutils-libelf-devel`, `bc`, `openssl`, `perl` | `flex`, `bison`, `libssl-dev`, `libelf-dev`, `bc`, `openssl`, `perl` |

Install the matching row before building that subtree; `./build-all-and-run.sh` expects **cpio**, **gzip**, and **erofs** userspace (`erofs-utils` / `mkfs.erofs`) up front.

**Disk helpers** (called from the build script; GPT layout matches StratBoot + `sysroot/initramfs-init.c`):

- `scripts/create-test-disk.sh`
- `scripts/update-test-disk.sh`

**Live ISO (optional):** after a full build, `./scripts/build-live-iso.sh` writes `out/live/stratos-live.iso`. Flash the ISO to USB with `dd` or another imager (verify the block device), boot on **UEFI hardware**, then run **`strat-installer`** as root to wipe a **whole internal disk** and install GPT/EROFS/ESP (see [docs/human/live-iso.md](docs/human/live-iso.md)).

**Secure Boot:** StratBoot and the kernel are **unsigned**—builds succeed, but many PCs will not boot until **Secure Boot is disabled** in firmware (or a future signing/shim path exists). Same constraint for the test disk and the live ISO; details in [docs/human/live-iso.md](docs/human/live-iso.md#secure-boot).

---

## Update path (summary)

- **StratMon** may download, verify, and stage payloads; it must not be the component that performs raw writes into inactive system slots.
- **StratBoot** runs in firmware context, reads EFI state (and manifests on the ESP when implemented), and owns block-level slot updates.

Details and invariants: **docs/human/stratos-design.md** (update system and boot chain sections).

---

## Contributing

1. Use **docs/human/coding-checklist.md** for scope and ordering.
2. Prefer small, reviewable PRs; match existing style; avoid drive-by refactors.
3. Optional multi-agent prompt templates: **[docs/agent/ai-roles.md](docs/agent/ai-roles.md)**.
