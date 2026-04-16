#include <efi.h>
#include <efilib.h>
#include "../efi/strat_efi_vars.h"

static const UINT8 kSlotAStatus = 1;
static const UINT8 kSlotBStatus = 1;
static const UINT8 kSlotCStatus = 0;
static const UINT8 kActiveSlot = 1;
static const UINT8 kPinnedSlot = 2;
static const UINT8 kResetFlags = 0x0F;
static const UINT8 kHomeStatus = 1;

static UINTN wide_strlen(const CHAR16 *s) {
    UINTN len = 0;
    while (s != NULL && s[len] != L'\0') {
        len++;
    }
    return len;
}

static INTN wide_contains(const CHAR16 *haystack, const CHAR16 *needle) {
    if (haystack == NULL || needle == NULL) {
        return 0;
    }

    UINTN hay_len = wide_strlen(haystack);
    UINTN needle_len = wide_strlen(needle);
    if (needle_len == 0 || hay_len < needle_len) {
        return 0;
    }

    for (UINTN i = 0; i <= hay_len - needle_len; i++) {
        UINTN j = 0;
        for (; j < needle_len; j++) {
            if (haystack[i + j] != needle[j]) {
                break;
            }
        }
        if (j == needle_len) {
            return 1;
        }
    }
    return 0;
}

static INTN load_option_contains(EFI_HANDLE image, CHAR16 *needle) {
    EFI_LOADED_IMAGE *loaded_image = NULL;
    EFI_STATUS status = uefi_call_wrapper(
        BS->HandleProtocol,
        3,
        image,
        &LoadedImageProtocol,
        (VOID **)&loaded_image
    );
    if (status != EFI_SUCCESS || loaded_image == NULL || loaded_image->LoadOptions == NULL) {
        return 0;
    }

    CHAR16 *opts = (CHAR16 *)loaded_image->LoadOptions;
    if (wide_contains(opts, needle)) {
        return 1;
    }
    return 0;
}

static EFI_STATUS set_all_vars(EFI_RUNTIME_SERVICES *rt) {
    EFI_STATUS status = EFI_SUCCESS;

    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_SLOT_A_STATUS, kSlotAStatus, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_SLOT_B_STATUS, kSlotBStatus, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_SLOT_C_STATUS, kSlotCStatus, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_ACTIVE_SLOT, kActiveSlot, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_PINNED_SLOT, kPinnedSlot, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS, kResetFlags, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;
    status = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_HOME_STATUS, kHomeStatus, STRAT_EFI_VAR_ATTRS);
    if (status != EFI_SUCCESS) return status;

    return EFI_SUCCESS;
}

static EFI_STATUS read_and_check(
    EFI_RUNTIME_SERVICES *rt,
    CHAR16 *name,
    UINT8 expected
) {
    UINT8 value = 0;
    EFI_STATUS status = strat_efi_get_u8(rt, name, &value);
    if (status != EFI_SUCCESS) {
        Print(L"GetVariable failed for %s: %r\n", name, status);
        return status;
    }
    if (value != expected) {
        Print(L"Value mismatch for %s: got %d expected %d\n", name, value, expected);
        return EFI_COMPROMISED_DATA;
    }
    return EFI_SUCCESS;
}

static EFI_STATUS check_all_vars(EFI_RUNTIME_SERVICES *rt) {
    EFI_STATUS status = EFI_SUCCESS;

    status = read_and_check(rt, STRAT_EFI_VAR_NAME_SLOT_A_STATUS, kSlotAStatus);
    if (status != EFI_SUCCESS) return status;
    status = read_and_check(rt, STRAT_EFI_VAR_NAME_SLOT_B_STATUS, kSlotBStatus);
    if (status != EFI_SUCCESS) return status;
    status = read_and_check(rt, STRAT_EFI_VAR_NAME_SLOT_C_STATUS, kSlotCStatus);
    if (status != EFI_SUCCESS) return status;
    status = read_and_check(rt, STRAT_EFI_VAR_NAME_ACTIVE_SLOT, kActiveSlot);
    if (status != EFI_SUCCESS) return status;
    status = read_and_check(rt, STRAT_EFI_VAR_NAME_PINNED_SLOT, kPinnedSlot);
    if (status != EFI_SUCCESS) return status;
    status = read_and_check(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS, kResetFlags);
    if (status != EFI_SUCCESS) return status;
    status = read_and_check(rt, STRAT_EFI_VAR_NAME_HOME_STATUS, kHomeStatus);
    if (status != EFI_SUCCESS) return status;

    return EFI_SUCCESS;
}

static EFI_STATUS corrupt_and_verify(EFI_RUNTIME_SERVICES *rt) {
    EFI_GUID guid = strat_efi_namespace_guid();
    UINT8 bad_payload[2] = {0xAA, 0xBB};

    EFI_STATUS status = rt->SetVariable(
        STRAT_EFI_VAR_NAME_SLOT_A_STATUS,
        &guid,
        STRAT_EFI_VAR_ATTRS,
        sizeof(bad_payload),
        bad_payload
    );
    if (status != EFI_SUCCESS) {
        Print(L"Failed to write corrupt variable: %r\n", status);
        return status;
    }

    UINT8 value = 0;
    status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_SLOT_A_STATUS, &value);
    if (status != EFI_COMPROMISED_DATA) {
        Print(L"Expected EFI_COMPROMISED_DATA for corrupt var, got: %r\n", status);
        return EFI_COMPROMISED_DATA;
    }

    return EFI_SUCCESS;
}

EFI_STATUS EFIAPI efi_main(EFI_HANDLE image, EFI_SYSTEM_TABLE *system_table) {
    InitializeLib(image, system_table);

    EFI_RUNTIME_SERVICES *rt = system_table->RuntimeServices;
    if (rt == NULL) {
        Print(L"Runtime services unavailable.\n");
        return EFI_ABORTED;
    }

    INTN do_write = load_option_contains(image, L"write");
    INTN do_read = load_option_contains(image, L"read");
    INTN do_corrupt = load_option_contains(image, L"corrupt");

    if (!do_write && !do_read && !do_corrupt) {
        Print(L"No mode specified. Use: write, read, or corrupt.\n");
        return EFI_INVALID_PARAMETER;
    }

    EFI_STATUS status = EFI_SUCCESS;

    if (do_write) {
        Print(L"Writing EFI variables...\n");
        status = set_all_vars(rt);
        if (status != EFI_SUCCESS) {
            Print(L"Write failed: %r\n", status);
            return status;
        }
    }

    if (do_read) {
        Print(L"Reading EFI variables...\n");
        status = check_all_vars(rt);
        if (status != EFI_SUCCESS) {
            Print(L"Read/verify failed: %r\n", status);
            return status;
        }
    }

    if (do_corrupt) {
        Print(L"Corrupting variable and verifying detection...\n");
        status = corrupt_and_verify(rt);
        if (status != EFI_SUCCESS) {
            Print(L"Corrupt test failed: %r\n", status);
            return status;
        }
    }

    Print(L"STRAT EFI VAR TEST: PASS\n");
    return EFI_SUCCESS;
}
