# Stratterm (human guide)

**Stratterm** is StratOS’s **Wayland terminal**: PTY, custom renderer, escape parser, optional file browser overlay, and companion tools (indexer, indexer settings CLI).

For a **feature-level changelog-style list**, see [../../stratterm/README.md](../../stratterm/README.md) in the crate.

## Documentation split

- **This page** — how Stratterm fits the OS (configs on `/config`, Wayland via `stratlayer`).
- **[file-explorer.md](file-explorer.md)** — “where is the file explorer?” → use Stratterm (`F7`).
- **[spotlite.md](spotlite.md)** — indexing / search roadmap vs code.
- **Agent brief:** [../agent/stratterm.md](../agent/stratterm.md).

## Config paths

- Indexer: `/config/strat/indexer.conf` (and `~/.config/...` fallback in tools).
- Panel is separate (`/config/strat/panel.conf`).
