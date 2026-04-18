# Application Configuration Resolution Contract

**This document defines the required configuration lookup pattern for all StratOS-native applications.**

## Configuration Lookup Order

Applications MUST follow this strict priority order. First match wins. No merging.

1. **Primary:** `/config/apps/<appname>/` — User customized configuration (always wins)
2. **Fallback:** `/system/etc/<appname>/` — System default configuration
3. **Final fallback:** Application built-in defaults (last resort)

## Forbidden Mechanisms

The following mechanisms are explicitly forbidden for configuration resolution:

- **No filesystem-level overrides** — No bind mounts, no overlayfs, no union mounts
- **No symlinks** — Configuration paths must be real directories
- **No implicit merging** — First match wins; do not combine configs from multiple sources
- **No duplication** — Config files exist in exactly one location; the application resolves at runtime

## Required Application Behavior

All StratOS-native applications MUST:

1. Check `/config/apps/<appname>/` first
2. If configuration is not found, check `/system/etc/<appname>/`
3. If neither location contains the required configuration, use built-in defaults
4. Never merge configurations from multiple sources
5. Never assume filesystem-level overrides exist

## Example Lookup Flow

```
Application: myapp
Configuration file: settings.conf

Lookup sequence:
1. Check /config/apps/myapp/settings.conf
   - If exists: use this file (user override)
   - If not found: continue

2. Check /system/etc/myapp/settings.conf
   - If exists: use this file (system default)
   - If not found: continue

3. Use built-in defaults
   - Application provides default configuration
```

## Example Directory Structures

```
/config/apps/myapp/
    settings.conf          # User custom configuration
    keybindings.conf       # User custom keybindings

/system/etc/myapp/
    settings.conf          # System default configuration
    keybindings.conf       # System default keybindings

Runtime behavior:
- If /config/apps/myapp/settings.conf exists, it is used exclusively
- If not, /system/etc/myapp/settings.conf is used
- If neither exists, application uses built-in defaults
```

## Implementation Notes

- Configuration resolution is entirely application-level logic
- No initramfs or mount-time setup is required
- The filesystem remains honest — no indirection, no union mounts
- Applications are responsible for implementing this lookup pattern
- System services and daemons must follow the same pattern

## References

- **Source of truth:** [stratos-design.md](stratos-design.md) section 3.5 "Config Priority Stack"
- **Runtime contract:** [runtime-persistence-contract.md](runtime-persistence-contract.md) (defines path ownership)

## Applicability

This contract applies to:

- All StratOS-native applications
- System services and daemons
- Any component that reads configuration files
- Components built with strat-build

This pattern is required. Deviations violate StratOS architecture.

## Version History

- 2026-04-16: Initial contract creation, defined application-level config priority
