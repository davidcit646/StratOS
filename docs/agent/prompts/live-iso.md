# Agent prompt: live bootable ISO (and path to installer)

Copy everything below the line into a new agent chat (or Cursor agent) as the **user message**. Add a one-line preamble if you want **follow-up work only** (this repo already ships a **Milestone A** xorriso pipeline) vs **green-field ISO design**.

---

## Mission

StratOS targets a **bootable x86_64 UEFI hybrid ISO** that runs the in-tree stack (not a generic rescue image). **Milestone A** (live session boots end-to-end) is implemented via **`./scripts/build-live-iso.sh`** → `out/live/stratos-live.iso` and **`docs/human/live-iso.md`**. **Milestone B** (graphical installer, preserve-CONFIG flows) remains design work in **stratos-design.md** section **17**; a **destructive CLI** install exists as **`scripts/strat-installer.sh`** → `/bin/strat-installer` in the phase7 rootfs / ISO payloads.

---

## Authoritative specs

- **`docs/human/stratos-design.md`** — section **17** (live USB / installer / diagnostic philosophy; section **17.9** still describes **mkosi** as a long-term packaging preference).
- **`docs/human/live-iso.md`** — **canonical in-repo** live vs installed behavior, `strat.live` / `strat.live_iso`, xorriso inputs, USB and bare-metal notes.
- **`docs/human/runtime-persistence-contract.md`** — partition roles; live must not pretend `/system` is mutable (tmpfs for CONFIG/APPS/HOME semantics is documented).
- **`docs/human/boot-stack.md`**, **`docs/agent/stratos-design.md`**, **`docs/agent/stratboot.md`**, **`docs/agent/stratsup-sysroot.md`**.

**Dev paths (do not break without an explicit migration):** `build-all-and-run.sh`, `scripts/create-test-disk.sh`, `scripts/update-test-disk.sh`, `sysroot/initramfs-init.c`, `stratboot/`, `stratman/`. ISO tooling **adds** alongside the disk image workflow.

---

## Already in the tree (grep these before proposing duplicates)

| Path | Role |
|------|------|
| `scripts/build-live-iso.sh` | **xorriso** + **mtools** + embedded FAT ESP; reads `out/phase7/slot-system.erofs`, `out/phase7/initramfs.cpio.gz`, `out/phase4/vmlinuz`, `out/phase3/BOOTX64.EFI`; copies initrd as **`initramfs.img`** (same blob, conventional name); **`-volid STRATOS_LIVE`** for label discovery; writes `EFI/STRAT/LIVE` marker on ESP. |
| `scripts/strat-installer.sh` | Fresh wipe → GPT layout matching `create-test-disk.sh`; finds ISO via **`LABEL=STRATOS_LIVE`** (`blkid`) then `/dev/sr0`…`sr31`; copies payloads from mount or `--source-dir`. |
| `sysroot/initramfs-init.c` | Live cmdline branches (`strat.live`, `strat.live_iso`) vs GPT `root=` install path; live ISO mount prefers **PVD volume id `STRATOS_LIVE`** then block-device scan. |
| `stratboot/src/stratboot.c` | Detects live medium; passes `strat.live=1 strat.live_iso=1` when `EFI/STRAT/LIVE` exists on the boot FAT. |

---

## Open / follow-up work (pick one slice per PR)

1. **Packaging:** Optional **mkosi** (or similar) profile that consumes the same `out/` artifacts — design section **17.9** target; today the shell script is the source of truth.
2. **Installer UX:** Menus, diagnostics, preserve-CONFIG options per design section **17.2–17.8** (today: typed-phrase CLI only in `strat-installer`).
3. **CI:** Optional workflow to build ISO (heavy; may stay `workflow_dispatch`).
4. **Ventoy / hardware matrix:** **Unsupported** in-tree until tested; add a short matrix to `live-iso.md` when reports exist.

---

## Constraints

- **Custom first:** the **in-tree** ISO path is **`xorriso`** + small helpers; **`mkosi`** is an admissible **replacement layer** if it stays reproducible and documented—do not leave two divergent ISO stories.
- **Single source of partition truth:** GPT names, sizes, and `PARTUUID` flow through `scripts/create-test-disk.sh`, `initramfs-init.c`, and StratBoot; installer and ISO must stay aligned.
- **Honest filesystem:** live session must not violate the spirit of **stratos-design** filesystem honesty (section **3.4**); tmpfs live policy is spelled out in `live-iso.md`.
- **StratMon / StratBoot:** block-level slot surgery remains bootloader-owned at reboot; `strat-installer` does partition + `dd` + ESP file copies, not StratMon raw slot writes.

---

## Suggested execution order (for new work)

1. Read **`docs/human/live-iso.md`** and run **`./scripts/build-live-iso.sh`** after a successful **`./build-all-and-run.sh -s`**.
2. Validate on **real UEFI hardware** (USB or optical); confirm **`stratman` → stratvm → stratpanel/stratterm** or note the failure mode in the human doc.
3. Only then layer **mkosi**, **CI**, or **rich installer UI**—each as its own reviewable change.

---

## Definition of done (when changing this area)

- Commands + artifact paths updated in **`docs/human/live-iso.md`** and **`docs/human/coding-checklist.md`** Phase **22** if behavior shifts.
- No stale references claiming ISO is “future only” while `build-live-iso.sh` exists.

---

*End of prompt*
