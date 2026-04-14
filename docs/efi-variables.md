# StratOS EFI Variable Schema

Namespace GUID (StratOS):
`10731b6f-16b5-4aea-ab46-c62aa093c8e5`

All EFI variables are `uint8` unless noted. The namespace GUID is used for every StratOS variable.

## Slot State

- `STRAT_SLOT_A_STATUS`:
  - `0` staging
  - `1` confirmed
  - `2` bad
- `STRAT_SLOT_B_STATUS`:
  - `0` staging
  - `1` confirmed
  - `2` bad
  - `3` pinned
- `STRAT_SLOT_C_STATUS`:
  - `0` staging
  - `1` confirmed
  - `2` bad

## Active / Pinned

- `STRAT_ACTIVE_SLOT`:
  - `0` A
  - `1` B
  - `2` C
- `STRAT_PINNED_SLOT`:
  - `0` none
  - `1` A
  - `2` B
  - `3` C

## Reset Flags (bitmask)

- `STRAT_RESET_FLAGS` bitmask:
  - bit0: config
  - bit1: home
  - bit2: system
  - bit3: factory

## Boot Counter

- `STRAT_BOOT_COUNT`:
  - increments on boot
  - reset to `0` on confirmed

## Last Known Good

- `STRAT_LAST_GOOD_SLOT`:
  - `0` A
  - `1` B
  - `2` C

## Home State

- `STRAT_HOME_STATUS`:
  - `0` healthy
  - `1` degraded
  - `2` corrupt
