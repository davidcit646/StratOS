# Runtime Persistence Mapping Contract

**This document is the single source of truth for all Phase 5 filesystem implementation.**

## Purpose

This contract defines the authoritative runtime filesystem mapping for StratOS. It resolves naming mismatches between the checklist and the actual runtime paths, and provides the complete path ownership table for Phase 5 implementation.

## Checklist Naming Resolution

The checklist uses legacy naming that does not match the actual runtime paths. This section explicitly maps the checklist terms to their runtime equivalents:

| Checklist Term | Runtime Path | Mount Source | Notes |
|---|---|---|---|
| `/cache` | `/apps` | STRAT_CACHE partition | Checklist "/cache" refers to the STRAT_CACHE partition, mounted at `/apps` at runtime |
| `/user` | `/home` | HOME partition | Checklist "/user" refers to the HOME partition, mounted at `/home` at runtime |

**Implementation note:** All Phase 5 scripts and documentation must use the runtime paths (`/apps`, `/home`), not the checklist terms.

## Runtime Path Ownership Table

| Path | Mount Source | Filesystem | Persistence | Mutability |
|---|---|---|---|---|
| `/system` | SLOT_A partition | EROFS | Immutable | Read-only (filesystem-level) |
| `/config` | CONFIG partition | ext4 | Persistent | Read-write |
| `/apps` | STRAT_CACHE partition | ext4 | Persistent | Read-write |
| `/home` | HOME partition | Btrfs | Persistent | Read-write |
| `/run` | tmpfs | tmpfs | Ephemeral | Read-write (RAM-only, lost on reboot) |
| `/var` | `/config/var` (bind mount) | ext4 (via /config) | Persistent | Read-write |
| `/usr` | `/system` (bind mount) | EROFS (via /system) | Immutable | Read-only |

## Direct Mounts

Partition → Mount Point mappings performed at initramfs time:

| Partition | Mount Point | Filesystem | Mount Flags |
|---|---|---|---|
| SLOT_A | `/system` | EROFS | MS_RDONLY |
| CONFIG | `/config` | ext4 | read-write |
| STRAT_CACHE | `/apps` | ext4 | read-write |
| HOME | `/home` | Btrfs | read-write |

## Bind Mounts

Source Path → Target Path mappings performed at initramfs time:

| Source | Target | Purpose |
|---|---|---|
| `/config/var` | `/var` | Persistent /var storage (created on first boot if missing) |
| `/system` | `/usr` | Legacy compatibility for hardcoded `/usr/*` paths |

## Path Categorization

### Immutable Paths
- `/system` — System binaries and libraries (EROFS, read-only at filesystem level)
- `/usr` — Bind mount of `/system`, inherits immutability

### Persistent Paths
- `/config` — User configuration, survives system destruction
- `/apps` — Build cache and .strat packages, survives system destruction
- `/home` — User data, survives system and config destruction
- `/var` — Persistent variable data (bind mount of `/config/var`)

### Ephemeral Paths
- `/run` — Runtime data, RAM-only, lost on reboot

## Three Layer Guarantee Mapping

This contract enforces the Three Layer Guarantee defined in [stratos-design.md](stratos-design.md):

1. **SYSTEM layer** (`/system`) — Read-only, immutable. Lose it: system reinstalls from pinned slot.
2. **CONFIG layer** (`/config`) — User settings and configuration. Lose it: system boots to defaults, all data intact.
3. **HOME layer** (`/home`) — User data. Lose it: painful but surgical. System and config untouched.

## References

- **Source of truth:** [stratos-design.md](stratos-design.md) section 3.4 "Honest Filesystem Structure"
- **Implementation reference:** sysroot/initramfs-init.c (mount operations at lines 129-175)

## Implementation Requirements

All Phase 5 implementation must:

1. Use runtime paths (`/apps`, `/home`), not checklist terms (`/cache`, `/user`)
2. Honor the persistence and mutability constraints defined above
3. Create `/config/var` on first boot if missing (as done in initramfs-init.c line 158)
4. Perform bind mounts exactly as specified in this contract
5. Never use symlinks for path aliasing (use bind mounts only)
6. Preserve the honest filesystem model — no union mounts, no overlayfs

## Version History

- 2026-04-16: Initial contract creation, resolved checklist naming mismatch
