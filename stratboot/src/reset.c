#include "reset.h"

#define STRAT_RESET_ZERO_MEGABYTES 64

static StratSlotId slot_id_from_u8(UINT8 value) {
    switch (value) {
        case STRAT_SLOT_A: return STRAT_SLOT_A;
        case STRAT_SLOT_B: return STRAT_SLOT_B;
        case STRAT_SLOT_C: return STRAT_SLOT_C;
        default: return STRAT_SLOT_NONE;
    }
}

static const CHAR16 *slot_partition_name(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"SLOT_A";
        case STRAT_SLOT_B: return L"SLOT_B";
        case STRAT_SLOT_C: return L"SLOT_C";
        default:           return NULL;
    }
}

static EFI_STATUS zero_named_partition(EFI_SYSTEM_TABLE *st, const CHAR16 *name) {
    EFI_BLOCK_IO *bio = NULL;
    EFI_STATUS status = strat_find_partition_by_name(st, name, &bio);
    if (status != EFI_SUCCESS) {
        return status;
    }
    return strat_partition_zero_header(bio, STRAT_RESET_ZERO_MEGABYTES);
}

static EFI_STATUS copy_pinned_to_active_slot(EFI_SYSTEM_TABLE *st) {
    if (st == NULL || st->RuntimeServices == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 pinned_value = STRAT_SLOT_NONE;
    EFI_STATUS status = strat_efi_get_u8(st->RuntimeServices, STRAT_EFI_VAR_NAME_PINNED_SLOT, &pinned_value);
    if (status != EFI_SUCCESS) {
        return status;
    }

    UINT8 active_value = STRAT_SLOT_A;
    status = strat_efi_get_u8(st->RuntimeServices, STRAT_EFI_VAR_NAME_ACTIVE_SLOT, &active_value);
    if (status != EFI_SUCCESS) {
        return status;
    }

    StratSlotId pinned_slot = slot_id_from_u8(pinned_value);
    StratSlotId active_slot = slot_id_from_u8(active_value);
    if (pinned_slot == STRAT_SLOT_NONE || active_slot == STRAT_SLOT_NONE) {
        return EFI_NOT_FOUND;
    }
    if (pinned_slot == active_slot) {
        return EFI_SUCCESS;
    }

    const CHAR16 *src_name = slot_partition_name(pinned_slot);
    const CHAR16 *dst_name = slot_partition_name(active_slot);
    if (src_name == NULL || dst_name == NULL) {
        return EFI_NOT_FOUND;
    }

    EFI_BLOCK_IO *src_bio = NULL;
    EFI_BLOCK_IO *dst_bio = NULL;

    status = strat_find_partition_by_name(st, src_name, &src_bio);
    if (status != EFI_SUCCESS) {
        return status;
    }

    status = strat_find_partition_by_name(st, dst_name, &dst_bio);
    if (status != EFI_SUCCESS) {
        return status;
    }

    return strat_partition_copy(src_bio, dst_bio);
}

EFI_STATUS strat_reset_read(EFI_RUNTIME_SERVICES *rt, StratResetState *out) {
    if (rt == NULL || out == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINT8 value = 0;
    EFI_STATUS status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS, &value);
    out->flags = (status == EFI_SUCCESS) ? value : 0;
    return EFI_SUCCESS;
}

EFI_STATUS strat_reset_clear(EFI_RUNTIME_SERVICES *rt) {
    if (rt == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    return strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS, 0, STRAT_EFI_VAR_ATTRS);
}

EFI_STATUS strat_execute_resets(EFI_SYSTEM_TABLE *st, UINT8 flags) {
    if (st == NULL || st->RuntimeServices == NULL || st->BootServices == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    if (flags == 0) {
        return EFI_SUCCESS;
    }

    UINT8 effective_flags = flags;
    if ((flags & STRAT_RESET_FLAG_FACTORY) != 0) {
        effective_flags = (UINT8)(effective_flags |
                                  STRAT_RESET_FLAG_CONFIG |
                                  STRAT_RESET_FLAG_HOME |
                                  STRAT_RESET_FLAG_SYSTEM);
    }

    EFI_STATUS status = EFI_SUCCESS;

    if ((effective_flags & STRAT_RESET_FLAG_CONFIG) != 0) {
        status = zero_named_partition(st, L"CONFIG");
        if (status != EFI_SUCCESS) {
            return status;
        }
    }

    if ((effective_flags & STRAT_RESET_FLAG_HOME) != 0) {
        status = zero_named_partition(st, L"HOME");
        if (status != EFI_SUCCESS) {
            return status;
        }
    }

    if ((effective_flags & STRAT_RESET_FLAG_SYSTEM) != 0) {
        status = copy_pinned_to_active_slot(st);
        if (status != EFI_SUCCESS) {
            return status;
        }
    }

    return EFI_SUCCESS;
}

const CHAR8 *strat_reset_describe(UINT8 flags) {
    if ((flags & STRAT_RESET_FLAG_FACTORY) != 0) return "Factory reset scheduled";
    if ((flags & STRAT_RESET_FLAG_SYSTEM) != 0) return "System reset scheduled";
    if ((flags & STRAT_RESET_FLAG_HOME) != 0) return "Home wipe scheduled";
    if ((flags & STRAT_RESET_FLAG_CONFIG) != 0) return "Config reset scheduled";
    return "No reset pending";
}
