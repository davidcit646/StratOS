# StratOS documentation

All long-form specs, contracts, and the coding checklist live under `**docs/human/**`.  
Task-oriented, file-heavy briefs for automation and agents live under `**docs/agent/**`.


| Audience        | Path                               | Purpose                                                       |
| --------------- | ---------------------------------- | ------------------------------------------------------------- |
| **People**      | [human/README.md](human/README.md) | Index of narrative docs and contracts                         |
| **Agents / CI** | [agent/README.md](agent/README.md) | Index of operational briefs (paths, invariants, do-not rules) |


## Quick links

### Human-readable (specs & contracts)

- [stratos-design.md](human/stratos-design.md) — full system design (authoritative when detailed).
- [boot-stack.md](human/boot-stack.md) — short boot chain narrative.
- [coding-checklist.md](human/coding-checklist.md) — phased checklist (`[x]` / `[ ]`).
- [discussion-log.md](human/discussion-log.md) — append-only engineering log.
- [runtime-persistence-contract.md](human/runtime-persistence-contract.md) — `/system`, `/config`, `/apps`, `/home`.
- [live-iso.md](human/live-iso.md) — live UEFI ISO build, boot, and `strat.live` behavior.
- [application-config-resolution.md](human/application-config-resolution.md) — app config priority (no overlay on `/system`).
- [etc-on-config.md](human/etc-on-config.md) — `/etc` backed by CONFIG.
- [efi-variables.md](human/efi-variables.md) — StratBoot EFI variable schema.

### Human-readable (components)

- [stratboot.md](human/stratboot.md) · [stratman.md](human/stratman.md) · [stratvm.md](human/stratvm.md) · [stratpanel.md](human/stratpanel.md) · [stratsettings.md](human/stratsettings.md)  
- [stratterm.md](human/stratterm.md) · [file-explorer.md](human/file-explorer.md) (use Stratterm for browsing files) · [spotlite.md](human/spotlite.md) · [stratmon.md](human/stratmon.md) · [stratsup-and-sysroot.md](human/stratsup-and-sysroot.md)

### Agent-readable (components + roles)

- [agent/README.md](agent/README.md) — index of all `docs/agent/*.md`.
- [agent/stratos-design.md](agent/stratos-design.md) — system design digest for agents (points here for full prose).
- [agent/ai-roles.md](agent/ai-roles.md) — optional Auditor / Builder / coordinator prompts.
- [agent/prompts/panel-window-chrome.md](agent/prompts/panel-window-chrome.md) — panel / decorations task prompt.
- [agent/prompts/panel-flesh-out.md](agent/prompts/panel-flesh-out.md) — workspace switcher, system tray, pinned apps.
- [agent/prompts/file-explorer.md](agent/prompts/file-explorer.md) — stratterm file browser / indexer task prompt.
- [agent/prompts/live-iso.md](agent/prompts/live-iso.md) — live ISO milestone prompt (`scripts/build-live-iso.sh` today; `mkosi` optional future per design).

### Elsewhere in the repo

- [README.md](../README.md) — clone, build, bare-metal images.
- [.github/workflows/stratos-ci.yml](../.github/workflows/stratos-ci.yml) — CI: full `./build-all-and-run.sh` + `./scripts/build-live-iso.sh` on push/PR.
- [stratos-kernel/README.md](../stratos-kernel/README.md) — kernel config fragments.
- [stratsettings/README.md](../stratsettings/README.md) — merged settings schema; see [human/stratsettings.md](human/stratsettings.md) for the OS-level summary.
- [stratterm/README.md](../stratterm/README.md) — terminal feature list; see [human/stratterm.md](human/stratterm.md) for OS-level docs.