# stratsettings

Modular StratOS UI configuration:

| Path | Role |
|------|------|
| `/config/strat/settings.toml` | Primary file (optional if only fragments + defaults) |
| `/config/strat/settings.d/*.toml` | Drop-in fragments; **sorted by name**; later files override |
| `/config/strat/panel.conf` | Legacy INI-style overlay for **`[panel]`** only (merged after TOML) |
| Embedded `defaults/settings.default.toml` | Shipped defaults when no files exist |

**Sections:** `[stratterm]`, `[panel]` (with `[panel.clock]`, `[panel.pinned]`, `[panel.tray]`), `[chrome]`, plus any **extra top-level table** (e.g. `[myproject]`) for third-party crates — captured in `StratSettings.extensions`.

**Types:** `StratSettings::load()`, `load_from(Path)`, optional `save_to(root)` for round-tripping.

**CLI:** `strat-ui-config show` | `strat-ui-config paths` | `strat-ui-config get <section> <key>`.

Booleans accept `true`/`false` or `1`/`0` in TOML.
