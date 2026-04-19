# Stratterm (human guide)

**Stratterm** is StratOS’s **Wayland terminal**: PTY, custom renderer, escape parser, optional file browser overlay, and companion tools (indexer, indexer settings CLI).

For a **feature-level changelog-style list**, see [../../stratterm/README.md](../../stratterm/README.md) in the crate.

## Documentation split

- **This page** — how Stratterm fits the OS (configs on `/config`, Wayland via `stratlayer`).
- **[file-explorer.md](file-explorer.md)** — “where is the file explorer?” → use Stratterm (`F7`).
- **[spotlite.md](spotlite.md)** — indexing / search roadmap vs code.
- **Agent brief:** [../agent/stratterm.md](../agent/stratterm.md).

## Config paths

- **Merged settings:** `/config/strat/settings.toml` and `settings.d/*.toml` (see [stratsettings.md](stratsettings.md)); Stratterm reads subsets for terminal / file-browser behavior.
- **Indexer:** `/config/strat/indexer.conf` (flat TOML; legacy **`strat-settings`** CLI still targets this file).
- **Panel:** configured through merged settings (or legacy `panel.conf` when no `settings.toml`); not Stratterm-owned.
