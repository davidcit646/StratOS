#include "slot.h"

static StratSlotId slot_id_from_u8(UINT8 value) {
    switch (value) {
        case STRAT_SLOT_A: return STRAT_SLOT_A;
        case STRAT_SLOT_B: return STRAT_SLOT_B;
        case STRAT_SLOT_C: return STRAT_SLOT_C;
        default: return STRAT_SLOT_NONE;
    }
}

static UINT8 slot_status_for(const StratSlotState *state, StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return state->slot_a_status;
        case STRAT_SLOT_B: return state->slot_b_status;
        case STRAT_SLOT_C: return state->slot_c_status;
        default: return STRAT_SLOT_STATUS_STAGING;
    }
}

EFI_STATUS strat_slot_read_state(EFI_RUNTIME_SERVICES *rt, StratSlotState *out) {
    if (rt == NULL || out == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 value = 0;
    EFI_STATUS status = EFI_SUCCESS;

    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_SLOT_A_STATUS, &value);
    out->slot_a_status = (status == EFI_SUCCESS) ? value : STRAT_SLOT_STATUS_STAGING;

    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_SLOT_B_STATUS, &value);
    out->slot_b_status = (status == EFI_SUCCESS) ? value : STRAT_SLOT_STATUS_STAGING;

    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_SLOT_C_STATUS, &value);
    out->slot_c_status = (status == EFI_SUCCESS) ? value : STRAT_SLOT_STATUS_STAGING;

    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_ACTIVE_SLOT, &value);
    out->active_slot = (status == EFI_SUCCESS) ? value : STRAT_SLOT_A;

    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_PINNED_SLOT, &value);
    out->pinned_slot = (status == EFI_SUCCESS) ? value : STRAT_SLOT_NONE;

    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS, &value);
    out->reset_flags = (status == EFI_SUCCESS) ? value : 0;

    return EFI_SUCCESS;
}

EFI_STATUS strat_slot_select(const StratSlotState *state, StratSlotDecision *out) {
    if (state == NULL || out == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (state->reset_flags != 0) {
        out->kind = STRAT_SLOT_DECISION_RESET_PENDING;
        out->slot = STRAT_SLOT_NONE;
        return EFI_SUCCESS;
    }

    StratSlotId active = slot_id_from_u8(state->active_slot);
    if (active != STRAT_SLOT_NONE &&
        slot_status_for(state, active) == STRAT_SLOT_STATUS_CONFIRMED) {
        out->kind = STRAT_SLOT_DECISION_BOOT;
        out->slot = active;
        return EFI_SUCCESS;
    }

    // Spec decision tree:
    // confirmed active -> next confirmed (A->B->C) -> pinned (if not BAD) -> halt
    // Fallback chain: A -> B -> C
    if (state->slot_a_status == STRAT_SLOT_STATUS_CONFIRMED) {
        out->kind = STRAT_SLOT_DECISION_BOOT;
        out->slot = STRAT_SLOT_A;
        return EFI_SUCCESS;
    }
    if (state->slot_b_status == STRAT_SLOT_STATUS_CONFIRMED) {
        out->kind = STRAT_SLOT_DECISION_BOOT;
        out->slot = STRAT_SLOT_B;
        return EFI_SUCCESS;
    }
    if (state->slot_c_status == STRAT_SLOT_STATUS_CONFIRMED) {
        out->kind = STRAT_SLOT_DECISION_BOOT;
        out->slot = STRAT_SLOT_C;
        return EFI_SUCCESS;
    }

    StratSlotId pinned = slot_id_from_u8(state->pinned_slot);
    if (pinned != STRAT_SLOT_NONE &&
        slot_status_for(state, pinned) != STRAT_SLOT_STATUS_BAD) {
        out->kind = STRAT_SLOT_DECISION_BOOT;
        out->slot = pinned;
        return EFI_SUCCESS;
    }

    out->kind = STRAT_SLOT_DECISION_HALT;
    out->slot = STRAT_SLOT_NONE;
    return EFI_SUCCESS;
}
