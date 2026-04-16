#include "strat_efi_vars.h"

EFI_GUID strat_efi_namespace_guid(void) {
    EFI_GUID guid = STRAT_EFI_NAMESPACE_GUID;
    return guid;
}

EFI_STATUS strat_efi_get_u8(
    EFI_RUNTIME_SERVICES *rt,
    CHAR16 *name,
    UINT8 *out_value
) {
    if (rt == NULL || name == NULL || out_value == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_GUID guid = strat_efi_namespace_guid();
    UINTN size = sizeof(UINT8);
    UINT32 attrs = 0;

    EFI_STATUS status = uefi_call_wrapper(rt->GetVariable, 5,
        name,
        &guid,
        &attrs,
        &size,
        out_value
    );

    if (status != EFI_SUCCESS) {
        return status;
    }

    if (size != sizeof(UINT8)) {
        return EFI_COMPROMISED_DATA;
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_efi_set_u8(
    EFI_RUNTIME_SERVICES *rt,
    CHAR16 *name,
    UINT8 value,
    UINT32 attrs
) {
    if (rt == NULL || name == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_GUID guid = strat_efi_namespace_guid();
    UINTN size = sizeof(UINT8);

    return uefi_call_wrapper(rt->SetVariable, 5,
        name,
        &guid,
        attrs,
        size,
        &value
    );
}

EFI_STATUS strat_efi_get_bytes(
    EFI_RUNTIME_SERVICES *rt,
    CHAR16 *name,
    UINT8 *out_value,
    UINTN size
) {
    if (rt == NULL || name == NULL || out_value == NULL || size == 0) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_GUID guid = strat_efi_namespace_guid();
    UINTN actual_size = size;
    UINT32 attrs = 0;

    EFI_STATUS status = uefi_call_wrapper(rt->GetVariable, 5,
        name,
        &guid,
        &attrs,
        &actual_size,
        out_value
    );

    if (status != EFI_SUCCESS) {
        return status;
    }

    if (actual_size != size) {
        return EFI_COMPROMISED_DATA;
    }

    return EFI_SUCCESS;
}

EFI_STATUS strat_efi_set_bytes(
    EFI_RUNTIME_SERVICES *rt,
    CHAR16 *name,
    UINT8 *value,
    UINTN size,
    UINT32 attrs
) {
    if (rt == NULL || name == NULL || value == NULL || size == 0) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_GUID guid = strat_efi_namespace_guid();

    return uefi_call_wrapper(rt->SetVariable, 5,
        name,
        &guid,
        attrs,
        size,
        value
    );
}

EFI_STATUS strat_efi_get_history(EFI_RUNTIME_SERVICES *rt, UINT8 *buffer, UINTN size) {
    if (rt == NULL || buffer == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (size != STRAT_UPDATE_HISTORY_SIZE) {
        return EFI_INVALID_PARAMETER;
    }

    return strat_efi_get_bytes(rt, STRAT_EFI_VAR_NAME_UPDATE_HISTORY, buffer, size);
}

EFI_STATUS strat_efi_set_history(EFI_RUNTIME_SERVICES *rt, UINT8 *buffer, UINTN size) {
    if (rt == NULL || buffer == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (size != STRAT_UPDATE_HISTORY_SIZE) {
        return EFI_INVALID_PARAMETER;
    }

    return strat_efi_set_bytes(rt, STRAT_EFI_VAR_NAME_UPDATE_HISTORY, buffer, size, STRAT_EFI_VAR_ATTRS);
}
