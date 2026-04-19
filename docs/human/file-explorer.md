# File explorer on StratOS

StratOS does **not** ship a separate “Files” or Nautilus-style application. If you want to **browse folders, preview files, and open or run things with confirmation**, that experience is built into **Stratterm** (the in-house terminal).

## What to use

1. Run **Stratterm** (it is the main terminal in a normal StratOS session).
2. The **title bar** and **`F7`** switch **focus** between the **explorer** (upper pane: list + preview) and the **shell** (lower pane). Both panes stay visible; only one receives keyboard input at a time. You can also click the upper or lower pane to move focus.
3. For search / indexing behind completions and paths, see **[spotlite.md](spotlite.md)** (indexer lives in the same crate as Stratterm today).

**Shortcuts and behavior** (authoritative, kept current with the code): **[../../stratterm/README.md](../../stratterm/README.md)** — read the “File browser” / overlay sections and the shortcut list.

**How Stratterm fits the OS** (mounts, `/config`, Wayland): **[stratterm.md](stratterm.md)**.

## For contributors / agents

Implementation work is scoped in the **[file explorer task prompt](../agent/prompts/file-explorer.md)**; code map: **[../agent/stratterm.md](../agent/stratterm.md)** and **[../agent/spotlite.md](../agent/spotlite.md)**.

A future **global** search launcher (design “Spotlite” overlay outside the terminal) is separate; see [spotlite.md](spotlite.md) and [coding-checklist.md](coding-checklist.md) Phase **12**.
