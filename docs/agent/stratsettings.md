# stratsettings — agent brief

## Paths

- `stratsettings/src/lib.rs` — `StratSettings` load/save, `settings.toml` + `settings.d/*.toml` merge, legacy `panel.conf` overlay **only when `settings.toml` is absent**.
- `stratsettings/defaults/settings.default.toml` — embedded first-merge defaults.
- `stratsettings/src/bin/strat_ui_config.rs` — CLI (`strat-ui-config show|get|paths`).
- `stratsettings/src/bin/stratos_settings/` — Wayland `xdg-shell` settings UI (`stratos-settings` binary).

## Consumers

- **stratpanel** (`stratpanel/src/config.rs`) — reads merged settings via `StratSettings::load()` → `PanelConfig`.
- **stratterm** (`stratterm/src/main.rs`) — `StratSettings::load()` for terminal / file-explorer fields.
- **stratterm-indexer** (`stratterm/src/bin/stratterm-indexer.rs`) — applies `[spotlite.headless]` from merged settings, then overlays `/config/strat/indexer.conf` when present.
- **stratvm** — reads `/config/strat/stratvm-keybinds` (Spotlite + layout-cycle hotkeys). That file is written whenever **`StratSettings::save_to`** runs (not only from the settings UI): same path as `strat-ui-config export-keybinds` / `write_stratvm_keybind_file`.
- **stratos-settings** — calls `StratSettings::save_to` under `$STRAT_CONFIG_ROOT` or `/config/strat` (which **includes** writing `stratvm-keybinds`), then notifies **stratvm** over IPC (see below).
- **stratman** (`stratman --network`) — `network::load_network_config()` reads merged settings (`[network]` in `settings.toml` + `settings.d`), after a narrow legacy shim from `/config/strat/network.toml` (`interface` / `use_dhcp` only); `STRAT_NETWORK_INTERFACE` overrides the interface name last.

## IPC

- Socket: `/run/stratvm.sock` (same as panel IPC).
- **`StratSettings::save_to`** — persists merged settings and **`stratvm-keybinds`** from `[keyboard]`; any code path that saves settings should leave the compositor consistent (hotkeys or `strat-ui-config export-keybinds` if you edit TOML by hand).
- **`stratos-settings`** after save — sends `set panel autohide <true|false>` so the panel updates without restart, and **`reload_keybinds`** so stratvm re-reads `stratvm-keybinds` without rebooting.

## Human doc

- [../human/stratsettings.md](../human/stratsettings.md) — OS-level summary and binary table.
- [../human/coding-checklist.md](../human/coding-checklist.md) Phase 26 (full “control center” scope is still partial).
