# Human-readable documentation

These documents are written for **people**: onboarding, architecture, contracts, and history. For grep-friendly, constraint-heavy briefs aimed at **agents and tooling**, see [../agent/README.md](../agent/README.md).

## System design & contracts


| Document                                                             | Description                                                 |
| -------------------------------------------------------------------- | ----------------------------------------------------------- |
| [stratos-design.md](stratos-design.md)                               | Canonical long-form design (filesystems, updates, desktop). Agent digest: [../agent/stratos-design.md](../agent/stratos-design.md). |
| [boot-stack.md](boot-stack.md)                                       | Short story: firmware → initramfs → stratman → compositor.  |
| [runtime-persistence-contract.md](runtime-persistence-contract.md)   | What lives on which partition; mount order.                 |
| [application-config-resolution.md](application-config-resolution.md) | How apps resolve config without mutating `/system`.         |
| [etc-on-config.md](etc-on-config.md)                                 | Why `/etc` is bind-mounted from CONFIG.                     |
| [efi-variables.md](efi-variables.md)                                 | EFI names StratBoot reads/writes.                           |


## Process & tracking


| Document                                   | Description                               |
| ------------------------------------------ | ----------------------------------------- |
| [coding-checklist.md](coding-checklist.md) | Phase checklist (checkboxes only).        |
| [discussion-log.md](discussion-log.md)     | Append-only audit/builder/engineer lines. |


## Components (overview)


| Document                                           | What it covers                                     |
| -------------------------------------------------- | -------------------------------------------------- |
| [stratboot.md](stratboot.md)                       | UEFI bootloader, slots, kernel handoff.            |
| [stratman.md](stratman.md)                         | PID 1, services, maintenance, network child.       |
| [stratvm.md](stratvm.md)                           | Wayland compositor (stratwm), IPC, layers.         |
| [stratpanel.md](stratpanel.md)                     | Top panel, layer shell client, `panel.conf`.       |
| [stratterm.md](stratterm.md)                       | Terminal, Wayland client, PTY.                     |
| [file-explorer.md](file-explorer.md)             | File browsing → use Stratterm (`F7`); no separate app. |
| [spotlite.md](spotlite.md)                         | Search/indexer vision vs what ships today.         |
| [stratmon.md](stratmon.md)                         | Update staging, manifest, FIEMAP.                  |
| [stratsup-and-sysroot.md](stratsup-and-sysroot.md) | Legacy supervisor crate + initramfs/root skeleton. |


Start with **stratos-design.md** or **boot-stack.md**, then open the component page you are changing.