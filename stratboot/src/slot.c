#include "slot.h"
#include "partition.h"
#include "sha256.h"

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

static const CHAR16 *slot_name_from_id(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"SLOT_A";
        case STRAT_SLOT_B: return L"SLOT_B";
        case STRAT_SLOT_C: return L"SLOT_C";
        default: return NULL;
    }
}

EFI_STATUS strat_slot_check_update_pending(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 value = 0;
    EFI_STATUS status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_STRAT_UPDATE_PENDING, &value);
    if (status != EFI_SUCCESS) {
        return status;
    }

    if (value == 0) {
        return EFI_NOT_FOUND;
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_slot_clear_boot_success(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    return strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_BOOT_SUCCESS, 0, STRAT_EFI_VAR_ATTRS);
}

EFI_STATUS strat_slot_increment_boot_attempts(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 value = 0;
    EFI_STATUS status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_BOOT_ATTEMPTS, &value);
    if (status == EFI_NOT_FOUND) {
        value = 0;
    } else if (status != EFI_SUCCESS) {
        return status;
    }

    value++;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_BOOT_ATTEMPTS, value, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) {
        return status;
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_slot_reset_boot_attempts(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    return strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_BOOT_ATTEMPTS, 0, STRAT_EFI_VAR_ATTRS);
}

EFI_STATUS strat_slot_handle_boot_state(EFI_SYSTEM_TABLE *st, EFI_RUNTIME_SERVICES *rt) {
    if (st == NULL || st->BootServices == NULL || rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 boot_success = 0;
    EFI_STATUS status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_BOOT_SUCCESS, &boot_success);
    if (status != EFI_SUCCESS) {
        boot_success = 0;
    }

    if (boot_success == 1) {
        status = strat_slot_reset_boot_attempts(rt);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_BOOT_SUCCESS, 0, STRAT_EFI_VAR_ATTRS);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_set_update_status(rt, 2);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_append_update_history(rt, 2);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_finalize_update(rt);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_set_update_status(rt, 4);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_append_update_history(rt, 4);
        if (status != EFI_SUCCESS) {
            return status;
        }
        return EFI_SUCCESS;
    }

    status = strat_slot_increment_boot_attempts(rt);
    if (status != EFI_SUCCESS) {
        return status;
    }

    UINT8 attempts = 0;
    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_BOOT_ATTEMPTS, &attempts);
    if (status != EFI_SUCCESS) {
        return status;
    }

    if (attempts > 3) {
        status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_ACTIVE_SLOT, STRAT_SLOT_A, STRAT_EFI_VAR_ATTRS);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_set_update_status(rt, 3);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_append_update_history(rt, 3);
        if (status != EFI_SUCCESS) {
            return status;
        }
        status = strat_slot_reset_boot_attempts(rt);
        if (status != EFI_SUCCESS) {
            return status;
        }
        return EFI_ABORTED;
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_slot_finalize_update(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    return strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_STRAT_UPDATE_PENDING, 0, STRAT_EFI_VAR_ATTRS);
}

EFI_STATUS strat_slot_set_update_status(EFI_RUNTIME_SERVICES *rt, UINT8 status) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    return strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_LAST_UPDATE_STATUS, status, STRAT_EFI_VAR_ATTRS);
}

EFI_STATUS strat_slot_append_update_history(EFI_RUNTIME_SERVICES *rt, UINT8 status) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 history[STRAT_UPDATE_HISTORY_SIZE];
    EFI_STATUS efi_status = strat_efi_get_bytes(rt, STRAT_EFI_VAR_NAME_UPDATE_HISTORY, history, STRAT_UPDATE_HISTORY_SIZE);

    if (efi_status != EFI_SUCCESS) {
        if (efi_status == EFI_NOT_FOUND) {
            for (UINTN i = 0; i < STRAT_UPDATE_HISTORY_SIZE; i++) {
                history[i] = 0;
            }
        } else {
            return efi_status;
        }
    }

    for (INTN i = 0; i < (STRAT_UPDATE_HISTORY_SIZE - 1); i++) {
        history[i] = history[i + 1];
    }

    history[STRAT_UPDATE_HISTORY_SIZE - 1] = status;

    return strat_efi_set_bytes(rt, STRAT_EFI_VAR_NAME_UPDATE_HISTORY, history, STRAT_UPDATE_HISTORY_SIZE, STRAT_EFI_VAR_ATTRS);
}

EFI_STATUS strat_slot_clear_update_request(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_STATUS status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_TARGET_SLOT, 0, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) {
        return status;
    }

    UINT8 zero_hash[32];
    for (UINTN i = 0; i < 32; i++) {
        zero_hash[i] = 0;
    }

    status = strat_efi_set_bytes(rt, STRAT_EFI_VAR_NAME_TARGET_HASH, zero_hash, 32, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) {
        return status;
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_slot_process_update_request(EFI_SYSTEM_TABLE *st, EFI_RUNTIME_SERVICES *rt) {
    if (st == NULL || st->BootServices == NULL || rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 update_pending = 0;
    EFI_STATUS status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_STRAT_UPDATE_PENDING, &update_pending);
    if (status != EFI_SUCCESS || update_pending == 0) {
        return EFI_NOT_READY;
    }

    UINT8 target_slot = 0;
    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_TARGET_SLOT, &target_slot);
    if (status != EFI_SUCCESS) {
        return status;
    }

    if (target_slot != STRAT_SLOT_A && target_slot != STRAT_SLOT_B && target_slot != STRAT_SLOT_C) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 target_hash[32];
    status = strat_efi_get_bytes(rt, STRAT_EFI_VAR_NAME_TARGET_HASH, target_hash, 32);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_slot_verify_hash(st, target_slot, target_hash);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_ACTIVE_SLOT, target_slot, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_slot_clear_boot_success(rt);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_slot_set_update_status(rt, 1);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_slot_append_update_history(rt, 1);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_slot_clear_update_request(rt);
    if (status != EFI_SUCCESS) {
        return status;
    }

    return EFI_SUCCESS;
}
