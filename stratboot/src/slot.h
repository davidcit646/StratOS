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
EFI_STATUS strat_slot_raw_copy(EFI_SYSTEM_TABLE *st, StratSlotId src_slot, StratSlotId dst_slot);
EFI_STATUS strat_slot_check_update_pending(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_rotate_to_b(EFI_SYSTEM_TABLE *st, EFI_RUNTIME_SERVICES *rt, const UINT8 *slot_b_hash);
EFI_STATUS strat_slot_clear_boot_success(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_increment_boot_attempts(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_reset_boot_attempts(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_handle_boot_state(EFI_SYSTEM_TABLE *st, EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_finalize_update(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_set_update_status(EFI_RUNTIME_SERVICES *rt, UINT8 status);
EFI_STATUS strat_slot_append_update_history(EFI_RUNTIME_SERVICES *rt, UINT8 status);
EFI_STATUS strat_slot_clear_update_request(EFI_RUNTIME_SERVICES *rt);
EFI_STATUS strat_slot_process_update_request(EFI_SYSTEM_TABLE *st, EFI_RUNTIME_SERVICES *rt);

#endif
