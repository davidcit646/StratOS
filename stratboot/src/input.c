#include "input.h"

EFI_STATUS strat_input_init(EFI_SYSTEM_TABLE *st, StratInput *out) {
    if (st == NULL || out == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    if (st->ConIn == NULL) {
        return EFI_NOT_FOUND;
    }

    out->text_in = st->ConIn;
    return EFI_SUCCESS;
}

EFI_STATUS strat_input_poll(StratInput *input, EFI_INPUT_KEY *out_key) {
    if (input == NULL || input->text_in == NULL || out_key == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    return uefi_call_wrapper(
        input->text_in->ReadKeyStroke,
        2,
        input->text_in,
        out_key
    );
}

EFI_STATUS strat_input_wait(StratInput *input, EFI_INPUT_KEY *out_key) {
    if (input == NULL || input->text_in == NULL || out_key == NULL) {
        return EFI_INVALID_PARAMETER;
    }

    UINTN index = 0;
    EFI_EVENT event = input->text_in->WaitForKey;
    if (event == NULL) {
        return EFI_NOT_FOUND;
    }

    EFI_STATUS status = uefi_call_wrapper(
        BS->WaitForEvent,
        3,
        1,
        &event,
        &index
    );
    if (status != EFI_SUCCESS) {
        return status;
    }

    return uefi_call_wrapper(
        input->text_in->ReadKeyStroke,
        2,
        input->text_in,
        out_key
    );
}
