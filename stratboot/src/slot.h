#ifndef STRAT_SLOT_H
#define STRAT_SLOT_H

#include <efi.h>
#include <efilib.h>
#include "../efi/strat_efi_vars.h"

typedef enum {
    STRAT_SLOT_A = 0,
    STRAT_SLOT_B = 1,
    STRAT_SLOT_C = 2,
    STRAT_SLOT_NONE = 0xFF
} StratSlotId;

typedef enum {
    STRAT_SLOT_STATUS_STAGING = 0,
    STRAT_SLOT_STATUS_CONFIRMED = 1,
    STRAT_SLOT_STATUS_BAD = 2,
    STRAT_SLOT_STATUS_PINNED = 3
} StratSlotStatus;

typedef enum {
    STRAT_SLOT_DECISION_BOOT = 0,
    STRAT_SLOT_DECISION_RESET_PENDING = 1,
    STRAT_SLOT_DECISION_HALT = 2
} StratSlotDecisionKind;

typedef struct {
    UINT8 slot_a_status;
    UINT8 slot_b_status;
    UINT8 slot_c_status;
    UINT8 active_slot;
    UINT8 pinned_slot;
    UINT8 reset_flags;
} StratSlotState;

typedef struct {
    StratSlotDecisionKind kind;
    StratSlotId slot;
} StratSlotDecision;

EFI_STATUS strat_slot_read_state(EFI_RUNTIME_SERVICES *rt, StratSlotState *out);
EFI_STATUS strat_slot_select(const StratSlotState *state, StratSlotDecision *out);

#endif
