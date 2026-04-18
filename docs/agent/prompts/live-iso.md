# Agent prompt: live bootable ISO (and path to installer)

Copy everything below the line into a new agent chat (or Cursor agent) as the **user message**. Add a one-line preamble if you want **Milestone A only** (ISO boots to a usable live session, no install wizard) vs **Milestone B** (partitioning + copy StratBoot + first-boot on target disk).

---

## Mission

Deliver a **bootable x86_64 UEFI ISO** (and/or raw disk image suitable for `dd` / Ventoy) that runs **StratOS’s own stack**—not a generic rescue distro. **Milestone A** is success if the image boots to **Linux → initramfs → userspace** with Strat’s layout policy honored as much as practical for a live session. **Milestone B** (full install flow, diagnostic menus, typed CONFIRM) is specified in the human design but can land in follow-up PRs once A is stable.

---

## Authoritative specs

- **`docs/human/stratos-design.md` §17** — Live USB / installer / diagnostic philosophy, flow sketches, **§17.9 ISO build** (`mkosi`, &lt; ~2GB target, Ventoy-compatible).
- **`docs/human/runtime-persistence-contract.md`** — partition roles; live session must not “fake” `/system` mutability (no overlay on immutable system slot semantics; tmpfs or explicit live-only paths are OK if documented).
- **`docs/human/boot-stack.md`**, **`docs/agent/stratos-design.md`**, **`docs/agent/stratboot.md`**, **`docs/agent/stratsup-sysroot.md`**.

**Today’s dev path (must stay working):** `build-all-and-run.sh`, `scripts/create-test-disk.sh`, `scripts/update-test-disk.sh`, `sysroot/initramfs-init.c`, `stratboot/`, `stratman/`. Do **not** break QEMU-all-in-one unless the team explicitly accepts a migration; ISO work should **add** a pipeline (new scripts / `mkosi` config) alongside existing flow.

---

## Current facts (grep before designing)

| Area | Why it matters |
|------|----------------|
| `sysroot/initramfs-init.c` | Expects **GPT PARTUUID** `root=` and a **fixed** mount sequence for installed disks. |
| `stratboot/` | Loads kernel+initrd from **ESP** paths; EFI variables for slots. Live medium may use **different** ESP layout or a **live-only** boot entry—document and implement deliberately. |
| `build-all-and-run.sh` | Produces kernel, `BOOTX64.EFI`, initramfs, rootfs, EROFS, **test disk** refresh. ISO build should **reuse** these artifacts or call the same build stages. |
| `stratman/` | PID 1 assumptions (mounts, manifests). Live may need **`strat.live=1`** (or similar) cmdline + conditional paths in **one** place (`initramfs` and/or `stratman`)—avoid silent divergence. |

---

## Milestone A — Live ISO (definition of done)

1. **Build:** One documented command (e.g. `./scripts/build-live-iso.sh` or `mkosi …`) produces **`out/…/*.iso`** (or equivalent) from the existing tree without manual copy-paste of fifteen binaries.
2. **Contents (minimum):**  
   - **StratBoot** `BOOTX64.EFI` on the ISO’s ESP (or hybrid MBR/ESP if required for Ventoy).  
   - **Kernel** + **initramfs** that StratBoot can load (same naming conventions as QEMU unless you unify).  
   - **Live root:** e.g. EROFS/squashfs image containing the same userspace slice as phase7 rootfs **or** a documented slimmer subset—**must** boot to **stratman → stratwm** (or, if temporarily blocked, to a **clear** emergency shell with message; no silent hang).
3. **Hardware:** Boots on **QEMU OVMF** with the new ISO **and** one sentence in README on testing on **real USB** (Ventoy optional stretch).
4. **Docs:** New **`docs/human/live-iso.md`** (how to build, flash, boot) + **`docs/human/coding-checklist.md`** new items or Phase **22** bullets for “ISO pipeline” so status is honest.
5. **CI (optional but preferred):** extend `.github/workflows/` with an **artifact** or **smoke** job that builds ISO (may be heavy—use `workflow_dispatch` or cache if needed).

---

## Milestone B — Install (stretch; after A)

Align implementation with **§17.2–17.8**: target disk selection, destructive CONFIRM, GPT creation matching **`create-test-disk.sh`** semantics, writing slots + CONFIG + HOME, ESP StratBoot install, EFI vars. Likely a **new Rust or C binary** (`strat-installer` / live helper) plus StratBoot paths when booted from removable media. **Do not** implement full UI polish before Milestone A ISO boots.

---

## Constraints

- **Custom first:** prefer **`mkosi`** per design §17.9; if you must use `xorriso`/`grub-mkrescue`-style glue, justify in README and keep scripts small and auditable.
- **No duplicate magic numbers:** GPT offsets, partition names, and `PARTUUID` assumptions live in **one** truth (scripts + initramfs + StratBoot); ISO layout should **call** or **generate from** the same spec (tables, shared `.json`, or generated headers—pick one approach and document it).
- **Honest filesystem:** live session must not violate the **spirit** of §3.4 (no “pretend” mutable `/system` on the slot image). If you use tmpfs overlays for `/etc` or `/var` in live-only mode, document under `docs/human/live-iso.md`.
- **StratMon / StratBoot law unchanged:** installer may stage files; **block-level slot surgery** remains StratBoot’s job at reboot unless design is explicitly revised (then update human doc in the same change).

---

## Suggested execution order

1. Read **`docs/human/stratos-design.md` §17.9** and trace **`build-all-and-run.sh`** outputs (`out/` layout).
2. Prototype **ESP + kernel + initrd + one EROFS root** in a directory tree; boot in QEMU with **ISO attached** (not only `-drive file=test-disk.img`).
3. Add **`mkosi`** manifest(s) or wrapper that assemble that tree from existing build artifacts.
4. Add **`strat.live=1`** (or equivalent) end-to-end: kernel cmdline → initramfs → optional `stratman` branch; verify **panel + stratterm** or minimal shell.
5. Only then start **Milestone B** partitioning/installer binary.

Start with a **short architecture note** (live vs installed mount graph) in `docs/human/live-iso.md`, then implement Milestone A.

---

## Definition of done (PR summary)

- Commands to build ISO + expected artifact paths.
- QEMU one-liner (or `scripts/run-qemu-iso.sh`) proving boot.
- Checklist / human doc updated so “live ISO” is trackable, not tribal knowledge.

---

*End of prompt*
