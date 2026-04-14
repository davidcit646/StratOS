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
