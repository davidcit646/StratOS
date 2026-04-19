# stratsettings (human guide)

**stratsettings** is the **modular configuration** crate: merged TOML under `/config/strat/` (`settings.toml`, optional `settings.d/*.toml`), embedded defaults, and a legacy **`panel.conf`** overlay only when no `settings.toml` exists.

## Binaries (rootfs)

| Binary | Role |
| ------ | ---- |
| **`stratos-settings`** | Wayland settings UI (`xdg-shell`); saves merged settings and notifies **stratvm** (`reload_keybinds`, panel autohide IPC). |
| **`strat-ui-config`** | CLI: `show`, `get`, `paths`, `export-keybinds` — inspect or persist without the GUI. |
| **`strat-settings`** (in **stratterm**) | Legacy CLI for flat **`/config/strat/indexer.conf`** workflows. |

## Consumers

**stratpanel**, **stratterm**, **stratterm-indexer**, **stratman** (`--network`), and compositor keybind files written via **`StratSettings::save_to`** (e.g. `/config/strat/stratvm-keybinds`). See the crate README for section names and merge rules.

## Related reading

- [stratsettings/README.md](../../stratsettings/README.md) — paths, sections, types.
- [../agent/stratsettings.md](../agent/stratsettings.md) — paths, IPC, invariants.
- [coding-checklist.md](coding-checklist.md) — Phases **14** and **26**.
