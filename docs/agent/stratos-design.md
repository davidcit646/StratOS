# StratOS system design — agent digest

**Authority:** Normative detail and rationale live in [../human/stratos-design.md](../human/stratos-design.md) (v0.4, long-form). If this digest disagrees with the human design doc, **follow the human doc** unless the repo has deliberately moved on—then update the human doc in the same change.

**Tighter contracts for paths and mounts:** [../human/runtime-persistence-contract.md](../human/runtime-persistence-contract.md), [../human/application-config-resolution.md](../human/application-config-resolution.md), [../human/efi-variables.md](../human/efi-variables.md).

---

## Non-negotiables (enforce on every change)


| Topic                      | Rule                                                                                                                                                                                                        |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Three layers**           | **SYSTEM** (EROFS slots A/B/C), **CONFIG** (persistent settings), **HOME** (user data). Any one may be lost; the others remain coherent per the guarantee (human sections **1.3**, **3.4**).                               |
| **Honest filesystem**      | No union/overlay on `/system`. No symlink tricks for `/bin` `/lib` `/etc`. Allowed: real **bind** mounts (e.g. `/system`→`/usr`, `/config/var`→`/var`) as in initramfs + contract.                          |
| **App config**             | Lookup order is **application-level**, not FS overlay: `/config/apps/…` → `/system/etc/…` → built-in defaults. See contract doc.                                                                            |
| **Bootloader sovereignty** | **StratBoot** runs at EFI; it may perform block-level slot work before Linux mounts.                                                                                                                        |
| **Update ownership**       | **StratMon** (user space) may **stage**, verify, and write **ESP manifest / EFI vars**. It must **not** write inactive **slot** partitions directly. **StratBoot** applies slot image updates when booting. |
| **Custom first**           | Prefer in-tree code; minimal new deps; align with [../human/coding-checklist.md](../human/coding-checklist.md).                                                                                             |


---

## Implemented boot / runtime chain (today)

1. **Firmware** → `BOOTX64.EFI` (**StratBoot**, `stratboot/`).
2. **Linux** + **initramfs** (`sysroot/initramfs-init.c`): mount `/system`, `/config`, `/apps`, `/home`, bind `/var`, exec `**/bin/stratman`**.
3. **stratman** (`stratman/`): PID 1, mounts, service manifests, optional `--network`.
4. **stratvm** (`stratvm/`, binary `stratwm`): wlroots compositor; **stratpanel** + **stratterm** as clients.

Short narrative: [../human/boot-stack.md](../human/boot-stack.md). Bootloader detail: [stratboot.md](stratboot.md). PID 1: [stratman.md](stratman.md).

---

## Human design doc → section map (grep targets)

Use the human doc for UI copy, menus, and future features; use this table to jump.


| Sec.  | Topic                                                          | Repo relevance                                                                                                                      |
| ----- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 1     | Philosophy / tenets                                            | Product constraints; “no nag”, honest FS.                                                                                           |
| 2     | Hardware requirements                                          | Docs only unless changing CI or hardware assumptions.                                                                                      |
| 3     | Architecture, kernel, **partitions**, **3.4–3.6** FS + config | **Must** match `initramfs-init.c`, disk scripts, persistence contract.                                                              |
| 4     | **StratBoot**, EFI schema, boot flow, recovery                 | `stratboot/`, [efi-variables.md](../human/efi-variables.md).                                                                        |
| 5     | Slots, **pinning**                                             | EFI vars + `stratboot` slot logic; user-facing rules.                                                                               |
| 6     | **Update system** (supervisor narrative)                       | **Implement split:** `stratmon/` staging vs `stratboot/` apply; human section **6.1** “supervisor on ESP” is aspirational—verify against code. |
| 7–8   | RAM, `.strat`                                                  | Mostly future / partial; grep crate `stratmon`, scripts before relying.                                                             |
| 9–11  | **stratvm**, panel, Spotlite                                   | `stratvm/`, `stratpanel/`; Spotlite largely in `stratterm/` today ([spotlite.md](spotlite.md)).                                     |
| 12–14 | Settings, terminal, default apps                               | `stratterm/`; checklist phases for gaps.                                                                                            |
| 15+   | Home recovery, AI pipeline, USB, branding                      | Mostly spec; cross-check checklist.                                                                                                 |


---

## Partition / path quick ref

GPT names and mount order: **human section 3.2** and **runtime persistence contract**. Scripts: `scripts/create-test-disk.sh`, `scripts/update-test-disk.sh` must stay aligned with **StratBoot** `root=` **PARTUUID** and initramfs.

Legacy name mapping (checklist vs runtime): `**/cache` → `/apps`**, `**/user` → under `/home**` (contract “Checklist naming resolution”).

---

## Component briefs (file-level)


| Area                         | Agent brief                                              |
| ---------------------------- | -------------------------------------------------------- |
| Bootloader                   | [stratboot.md](stratboot.md)                             |
| PID 1 / services             | [stratman.md](stratman.md)                               |
| Compositor                   | [stratvm.md](stratvm.md)                                 |
| Panel                        | [stratpanel.md](stratpanel.md)                           |
| Terminal / indexer           | [stratterm.md](stratterm.md), [spotlite.md](spotlite.md) |
| Update staging               | [stratmon.md](stratmon.md)                               |
| Initramfs / stratsup helpers | [stratsup-sysroot.md](stratsup-sysroot.md)               |


---

## Grep starters

```text
rg "mount_or_die|PARTUUID|/config|/system" sysroot stratman stratboot
rg "UPDATE\\.MAN|stage-update|FIEMAP" stratmon
rg "strat_slot_process_update_request|PINNED" stratboot
rg "layers_normal|stratvm.sock" stratvm stratpanel
```

---

## Human doc (full design)

[../human/stratos-design.md](../human/stratos-design.md)