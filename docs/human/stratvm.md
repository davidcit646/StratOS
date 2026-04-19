# stratvm / stratwm (human guide)

**stratvm** is the in-tree **Wayland compositor**; the built binary is typically named `**stratwm`**. It uses **wlroots** for rendering and input, talks to the kernel via **libinput** and/or a **direct evdev** path when needed, and exposes a small **Unix socket IPC** for the panel and tools.

## User-visible behavior

- Tiled workspaces, floating windows, titlebar controls.
- **Layer shell** surfaces (e.g. the panel) live above normal XDG windows using separate scene layer trees.
- Autostarts `/bin/stratterm` and `/bin/stratpanel` with `WAYLAND_DISPLAY` set.

## Related reading

- Design doc windowing / desktop sections: [stratos-design.md](stratos-design.md).
- Checklist phases **10**, **24a**, **25**: [coding-checklist.md](coding-checklist.md).
- Agent brief: [../agent/stratvm.md](../agent/stratvm.md).