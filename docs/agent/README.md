# Agent-readable documentation

Dense, **operational** briefs: file paths, entrypoints, invariants, and “do not” rules. Use these when driving an agent or searching the tree quickly. **Full system narrative** lives in [../human/stratos-design.md](../human/stratos-design.md); start agents with the digest below, then open the human doc when detail matters.

## System design (agent digest)

| Brief | Scope |
| ----- | ----- |
| [stratos-design.md](stratos-design.md) | Invariants, boot chain, update split, section map to human design doc, grep starters |

## Component briefs


| Brief                                      | Scope                                          |
| ------------------------------------------ | ---------------------------------------------- |
| [stratboot.md](stratboot.md)               | `stratboot/`, EFI vars, kernel paths on ESP    |
| [stratman.md](stratman.md)                 | `stratman/`, manifests, PID 1, `--network`     |
| [stratvm.md](stratvm.md)                   | `stratvm/`, wlroots, IPC, scene layers         |
| [stratpanel.md](stratpanel.md)             | `stratpanel/`, stratlayer, `/run/stratvm.sock` |
| [stratterm.md](stratterm.md)               | `stratterm/`, binaries, config paths           |
| [spotlite.md](spotlite.md)                 | Indexer, file browser, `/bin/spotlite` overlay   |
| [stratmon.md](stratmon.md)                 | `stratmon/`, `--stage-update`, UPDATE.MAN      |
| [stratsettings.md](stratsettings.md)       | `stratsettings/`, TOML merge, `stratos-settings` |
| [stratsup-sysroot.md](stratsup-sysroot.md) | `stratsup/`, `sysroot/`, initramfs             |


## Workflow


| Brief                      | Scope                        |
| -------------------------- | ---------------------------- |
| [ai-roles.md](ai-roles.md) | Optional multi-agent prompts |


## Task prompts


| Path                                                             | Use                                      |
| ---------------------------------------------------------------- | ---------------------------------------- |
| [prompts/panel-window-chrome.md](prompts/panel-window-chrome.md) | Panel / decorations follow-up          |
| [prompts/panel-flesh-out.md](prompts/panel-flesh-out.md)           | Workspace switcher, tray, pinned strip |
| [prompts/file-explorer.md](prompts/file-explorer.md)             | Stratterm file browser + indexer / Phase 12 |
| [prompts/live-iso.md](prompts/live-iso.md)                       | Live ISO / installer roadmap (`xorriso` pipeline in-tree; Phase 17 UI still open) |


When a brief disagrees with **[stratos-design.md](stratos-design.md)** or **[../human/stratos-design.md](../human/stratos-design.md)**, the **human** design doc wins unless the code has deliberately superseded it (then update the human doc).