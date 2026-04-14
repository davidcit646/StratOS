#include <efi.h>
#include <efilib.h>
#include <efiser.h>
#include "gop.h"
#include "font.h"
#include "input.h"
#include "slot.h"
#include "reset.h"

#define STRAT_HOME_STATUS_HEALTHY  0
#define STRAT_HOME_STATUS_DEGRADED 1
#define STRAT_HOME_STATUS_CORRUPT  2
static const CHAR16 STRAT_SMOKE_BOOTING_SLOT_VAR[] = L"STRAT_SMOKE_BOOTING_SLOT";
static const CHAR16 STRAT_SMOKE_EFI_MAIN_VAR[] = L"STRAT_SMOKE_EFI_MAIN";

static BOOLEAN show_confirm_prompt(StratGop *gop, StratInput *input);

static void debugcon_log(const CHAR8 *msg) {
    if (msg == NULL) {
        return;
    }
    const UINT16 debug_port = 0xE9;
    while (*msg != '\0') {
        __asm__ __volatile__("outb %0, %1" : : "a"((UINT8)*msg), "Nd"(debug_port));
        msg++;
    }
}

static void serial_log(EFI_SYSTEM_TABLE *st, const CHAR8 *msg) {
    if (st == NULL || st->BootServices == NULL || msg == NULL) {
        return;
    }

    EFI_SERIAL_IO_PROTOCOL *serial = NULL;
    EFI_STATUS status = uefi_call_wrapper(st->BootServices->LocateProtocol, 3,
                                          &SerialIoProtocol, NULL, (VOID **)&serial);
    if (status != EFI_SUCCESS || serial == NULL) {
        return;
    }

    // Best-effort normalization for OVMF/QEMU serial sinks.
    uefi_call_wrapper(serial->SetAttributes, 7, serial,
                      115200, 0, 0, NoParity, 8, OneStopBit);

    UINTN len = strlena(msg);
    if (len == 0) {
        return;
    }

    uefi_call_wrapper(serial->Write, 3, serial, &len, (VOID *)msg);
}

static INT32 centered_text_x(StratGop *gop, const CHAR8 *text) {
    UINTN width = strat_gop_width(gop);
    UINTN text_width = strlena(text) * STRAT_FONT_WIDTH;
    if (text_width >= width) {
        return 0;
    }
    return (INT32)((width - text_width) / 2);
}

static void draw_filled_circle(
    StratGop *gop,
    INT32 cx,
    INT32 cy,
    INT32 radius,
    UINT8 r,
    UINT8 g,
    UINT8 b
) {
    INT32 rr = radius * radius;
    for (INT32 y = -radius; y <= radius; y++) {
        for (INT32 x = -radius; x <= radius; x++) {
            if ((x * x) + (y * y) <= rr) {
                strat_gop_put_pixel(gop, cx + x, cy + y, r, g, b);
            }
        }
    }
}

static void draw_boot_screen(StratGop *gop) {
    strat_gop_clear(gop, 0x08, 0x09, 0x0c);

    INT32 cx = (INT32)(strat_gop_width(gop) / 2);
    INT32 cy = (INT32)(strat_gop_height(gop) / 3);

    // Center logo mark: filled circle + inner S + two halo rings.
    draw_filled_circle(gop, cx, cy, 18, 0xE6, 0xE8, 0xEA);
    strat_font_draw_char(gop, cx - (STRAT_FONT_WIDTH / 2), cy - (STRAT_FONT_HEIGHT / 2), 'S', 0x08, 0x09, 0x0c);
    strat_gop_draw_circle(gop, cx, cy, 26, 0x3A, 0x3C, 0x42);
    strat_gop_draw_circle(gop, cx, cy, 34, 0x3A, 0x3C, 0x42);

    // Static spinner segment at 0 degrees (pointing up), anchored at logo center.
    strat_gop_draw_line(gop, cx, cy, cx, cy - 12, 0x08, 0x09, 0x0c);

    const CHAR8 *wordmark = "STRAT OS";
    INT32 wordmark_y = cy + 34 + 20;
    strat_font_draw_text(gop, centered_text_x(gop, wordmark), wordmark_y, wordmark, 0xE6, 0xE8, 0xEA);

    const CHAR8 *version = "v0.1";
    INT32 version_y = wordmark_y + STRAT_FONT_HEIGHT + 10;
    strat_font_draw_text(gop, centered_text_x(gop, version), version_y, version, 0x5A, 0x5C, 0x62);

    const CHAR8 *hint = "Esc - interrupt boot";
    INT32 hint_y = (INT32)strat_gop_height(gop) - 24 - STRAT_FONT_HEIGHT;
    if (hint_y < 0) {
        hint_y = 0;
    }
    strat_font_draw_text(gop, centered_text_x(gop, hint), hint_y, hint, 0x3A, 0x3C, 0x42);
}

static void draw_status(StratGop *gop, const CHAR8 *line1, const CHAR8 *line2) {
    draw_boot_screen(gop);
    strat_font_draw_text(gop, 40, 56, line1, 0xE6, 0xE8, 0xEA);
    strat_font_draw_text(gop, 40, 72, line2, 0x9A, 0x9C, 0xA0);
}

static void halt_with_message(EFI_SYSTEM_TABLE *st, StratGop *gop, const CHAR8 *line1, const CHAR8 *line2) {
    draw_status(gop, line1, line2);

    StratInput input;
    if (strat_input_init(st, &input) == EFI_SUCCESS) {
        EFI_INPUT_KEY key;
        strat_input_wait(&input, &key);
    }

    // If input is not available, just stall.
    for (;;) {
        uefi_call_wrapper(st->BootServices->Stall, 1, 1000000);
    }
}

static const CHAR8 *home_status_detail(UINT8 home_status) {
    switch (home_status) {
        case STRAT_HOME_STATUS_DEGRADED:
            return "Mount check failed: /home is degraded (STRAT_HOME_STATUS=1)";
        case STRAT_HOME_STATUS_CORRUPT:
            return "Mount check failed: /home is corrupt (STRAT_HOME_STATUS=2)";
        default:
            return "Mount check failed: /home status unknown";
    }
}

static void show_modal_notice(
    EFI_SYSTEM_TABLE *st,
    StratGop *gop,
    StratInput *input,
    const CHAR8 *line1,
    const CHAR8 *line2
) {
    if (gop == NULL) {
        return;
    }

    strat_gop_clear(gop, 0x08, 0x09, 0x0c);
    strat_font_draw_text(gop, centered_text_x(gop, line1), 80, line1, 0xE6, 0xE8, 0xEA);
    strat_font_draw_text(gop, centered_text_x(gop, line2), 96, line2, 0x9A, 0x9C, 0xA0);
    strat_font_draw_text(gop, centered_text_x(gop, "Press any key to continue"), 128, "Press any key to continue",
                         0x5A, 0x5C, 0x62);

    if (input != NULL) {
        EFI_INPUT_KEY key;
        strat_input_wait(input, &key);
        return;
    }

    if (st != NULL) {
        uefi_call_wrapper(st->BootServices->Stall, 1, 1500000);
    }
}

static void draw_home_corruption_screen_frame(
    StratGop *gop,
    INT32 focus,
    UINT8 pulse_step,
    UINT8 home_status
) {
    static const CHAR8 *kOptions[4] = {
        "Attempt Boot",
        "Reset / Wipe Home",
        "Attempt Hard Recovery",
        "Open Recovery Terminal",
    };
    static const CHAR8 *kTags[4] = {
        "safe",
        "destructive",
        "technical",
        "terminal",
    };

    UINT8 pulse_r[8] = {0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xD0, 0xC0, 0xB0};
    UINT8 pulse_g[8] = {0x58, 0x60, 0x66, 0x6C, 0x74, 0x6C, 0x66, 0x60};
    UINT8 pulse_b[8] = {0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
    UINT8 p = pulse_step % 8;

    INT32 screen_w = (INT32)strat_gop_width(gop);

    strat_gop_clear(gop, 0x08, 0x09, 0x0c);
    draw_filled_circle(gop, 24, 20, 4, pulse_r[p], pulse_g[p], pulse_b[p]);
    strat_font_draw_text(gop, 36, 16, "Strat OS - Boot Alert - Home Partition", 0xE6, 0xE8, 0xEA);

    strat_font_draw_text(gop, 32, 44, "Hey.", 0xE6, 0xE8, 0xEA);
    strat_font_draw_text(gop, 32, 60, "So this sucks, but your home directory is gone.", 0xE6, 0xE8, 0xEA);
    strat_font_draw_text(gop, 32, 76, "Your system and config are fine. /home cannot mount cleanly.",
                         0x9A, 0x9C, 0xA0);

    INT32 box_x = 32;
    INT32 box_y = 104;
    INT32 box_w = screen_w - 64;
    INT32 box_h = 26;
    if (box_w < 120) {
        box_w = 120;
    }
    strat_gop_fill_rect(gop, box_x, box_y, box_w, box_h, 0x16, 0x17, 0x1B);
    strat_font_draw_text(gop, box_x + 8, box_y + 8, home_status_detail(home_status), 0xD8, 0xB4, 0x6A);

    strat_font_draw_text(gop, 32, 144, "Your options:", 0xE6, 0xE8, 0xEA);
    for (INT32 i = 0; i < 4; i++) {
        INT32 y = 164 + (i * 24);
        if (i == focus) {
            strat_gop_fill_rect(gop, 28, y, 3, STRAT_FONT_HEIGHT, 0x5B, 0x9B, 0xD5);
            strat_font_draw_text(gop, 36, y, kOptions[i], 0xE6, 0xE8, 0xEA);
            strat_font_draw_text(gop, 220, y, kTags[i], 0xD8, 0xB4, 0x6A);
        } else {
            strat_font_draw_text(gop, 36, y, kOptions[i], 0x5A, 0x5C, 0x62);
            strat_font_draw_text(gop, 220, y, kTags[i], 0x3A, 0x3C, 0x42);
        }
    }

    strat_font_draw_text(gop, 32, 268, "System: healthy  Config: healthy  Home: degraded/corrupt",
                         0x5A, 0x5C, 0x62);
    strat_font_draw_text(gop, 32, 284, "Up/Down navigate  Enter select  Esc attempt boot",
                         0x3A, 0x3C, 0x42);
}

static BOOLEAN show_home_corruption_screen(
    EFI_SYSTEM_TABLE *st,
    StratGop *gop,
    StratInput *input,
    UINT8 home_status
) {
    if (st == NULL || gop == NULL || input == NULL) {
        return FALSE;
    }

    INT32 focus = 0;
    UINT8 pulse_step = 0;
    while (1) {
        draw_home_corruption_screen_frame(gop, focus, pulse_step, home_status);
        pulse_step = (UINT8)((pulse_step + 1) % 8);

        for (INTN tick = 0; tick < 4; tick++) {
            EFI_INPUT_KEY key;
            EFI_STATUS poll_status = strat_input_poll(input, &key);
            if (poll_status != EFI_SUCCESS) {
                uefi_call_wrapper(st->BootServices->Stall, 1, 100000);
                continue;
            }

            if (key.ScanCode == 0x17) {
                return TRUE;
            }
            if (key.ScanCode == 0x01) {
                if (focus > 0) {
                    focus--;
                }
                break;
            }
            if (key.ScanCode == 0x02) {
                if (focus < 3) {
                    focus++;
                }
                break;
            }
            if (!(key.ScanCode == 0x00 && key.UnicodeChar == 0x000D)) {
                break;
            }

            if (focus == 0) {
                return TRUE;
            }

            if (focus == 1) {
                if (!show_confirm_prompt(gop, input)) {
                    break;
                }
                StratResetState reset_state;
                UINT8 flags = STRAT_RESET_FLAG_HOME;
                if (strat_reset_read(st->RuntimeServices, &reset_state) == EFI_SUCCESS) {
                    flags = (UINT8)(reset_state.flags | STRAT_RESET_FLAG_HOME);
                }

                EFI_STATUS set_status = strat_efi_set_u8(
                    st->RuntimeServices,
                    STRAT_EFI_VAR_NAME_RESET_FLAGS,
                    flags,
                    STRAT_EFI_VAR_ATTRS
                );
                if (set_status != EFI_SUCCESS) {
                    show_modal_notice(st, gop, input, "Failed to schedule home wipe", "Please retry.");
                    break;
                }
                uefi_call_wrapper(st->RuntimeServices->ResetSystem, 4, EfiResetWarm, EFI_SUCCESS, 0, NULL);
                return FALSE;
            }

            if (focus == 2) {
                show_modal_notice(st, gop, input, "Hard recovery is not available yet.",
                                  "Phase 15 implementation pending.");
                break;
            }

            show_modal_notice(st, gop, input, "Recovery terminal is not available yet.",
                              "Phase 15 implementation pending.");
            break;
        }
    }

    return FALSE;
}

static BOOLEAN show_confirm_prompt(StratGop *gop, StratInput *input) {
    if (gop == NULL || input == NULL) {
        return FALSE;
    }

    CHAR8 buffer[17];
    UINTN len = 0;
    buffer[0] = '\0';

    while (1) {
        strat_gop_clear(gop, 0x08, 0x09, 0x0c);
        const CHAR8 *line1 = "Type CONFIRM and press Enter to proceed.";
        const CHAR8 *line2 = "Any other input cancels.";
        strat_font_draw_text(gop, centered_text_x(gop, line1), 40, line1, 0xE6, 0xE8, 0xEA);
        strat_font_draw_text(gop, centered_text_x(gop, line2), 56, line2, 0x5A, 0x5C, 0x62);
        strat_font_draw_text(gop, centered_text_x(gop, buffer), 80, buffer, 0xE6, 0xE8, 0xEA);

        EFI_INPUT_KEY key;
        EFI_STATUS key_status = strat_input_wait(input, &key);
        if (key_status != EFI_SUCCESS) {
            continue;
        }

        if (key.ScanCode == 0x17) {
            return FALSE;
        }

        if (key.ScanCode == 0x00 && key.UnicodeChar == 0x000D) {
            if (strcmpa(buffer, "CONFIRM") == 0) {
                return TRUE;
            }
            return FALSE;
        }

        if (key.UnicodeChar == 0x0008) {
            if (len > 0) {
                len--;
                buffer[len] = '\0';
            }
            continue;
        }

        if (key.UnicodeChar >= 0x0020 && key.UnicodeChar <= 0x007E && len < 16) {
            buffer[len++] = (CHAR8)key.UnicodeChar;
            buffer[len] = '\0';
        }
    }
}

static void show_recovery_menu(
    EFI_SYSTEM_TABLE *st,
    StratGop *gop,
    StratInput *input
) {
    if (st == NULL || gop == NULL || input == NULL) {
        return;
    }

    static const CHAR8 *kMenuItems[6] = {
        "Reset CONFIG to defaults",
        "Wipe HOME",
        "Reset CONFIG + Wipe HOME",
        "Reflash system from pinned slot",
        "Factory reset - everything",
        "Cancel",
    };

    INT32 focus = 0;
    while (1) {
        strat_gop_clear(gop, 0x08, 0x09, 0x0c);

        const CHAR8 *title = "Recovery options. These are destructive. Be sure.";
        strat_font_draw_text(gop, centered_text_x(gop, title), 40, title, 0xE6, 0xE8, 0xEA);

        for (INT32 i = 0; i < 6; i++) {
            INT32 item_y = 80 + (i * 20);
            if (i == focus) {
                strat_gop_fill_rect(gop, 32, item_y, 3, STRAT_FONT_HEIGHT, 0x5B, 0x9B, 0xD5);
                strat_font_draw_text(
                    gop, centered_text_x(gop, kMenuItems[i]), item_y, kMenuItems[i], 0xE6, 0xE8, 0xEA
                );
            } else {
                strat_font_draw_text(
                    gop, centered_text_x(gop, kMenuItems[i]), item_y, kMenuItems[i], 0x5A, 0x5C, 0x62
                );
            }
        }

        EFI_INPUT_KEY key;
        EFI_STATUS key_status = strat_input_wait(input, &key);
        if (key_status != EFI_SUCCESS) {
            continue;
        }

        if (key.ScanCode == 0x17) {
            return;
        }

        if (key.ScanCode == 0x01) {
            if (focus > 0) {
                focus--;
            }
            continue;
        }

        if (key.ScanCode == 0x02) {
            if (focus < 5) {
                focus++;
            }
            continue;
        }

        if (!(key.ScanCode == 0x00 && key.UnicodeChar == 0x000D)) {
            continue;
        }

        if (focus == 5) {
            return;
        }

        if (!show_confirm_prompt(gop, input)) {
            continue;
        }

        UINT8 flags = 0;
        switch (focus) {
            case 0:
                flags = STRAT_RESET_FLAG_CONFIG;
                break;
            case 1:
                flags = STRAT_RESET_FLAG_HOME;
                break;
            case 2:
                flags = STRAT_RESET_FLAG_CONFIG | STRAT_RESET_FLAG_HOME;
                break;
            case 3:
                flags = STRAT_RESET_FLAG_SYSTEM;
                break;
            case 4:
                flags = STRAT_RESET_FLAG_FACTORY;
                break;
            default:
                continue;
        }

        EFI_STATUS set_status = strat_efi_set_u8(
            st->RuntimeServices,
            STRAT_EFI_VAR_NAME_RESET_FLAGS,
            flags,
            STRAT_EFI_VAR_ATTRS
        );
        if (set_status != EFI_SUCCESS) {
            show_modal_notice(st, gop, input, "Failed to schedule reset operation", "Please retry.");
            continue;
        }
        uefi_call_wrapper(st->RuntimeServices->ResetSystem, 4, EfiResetWarm, EFI_SUCCESS, 0, NULL);
        return;
    }
}

static void request_firmware_ui_and_reset(
    EFI_SYSTEM_TABLE *st,
    StratGop *gop,
    StratInput *input
) {
    if (st == NULL || st->RuntimeServices == NULL) {
        return;
    }

    UINT64 indications = 0;
    UINTN size = sizeof(indications);
    UINT32 attrs = EFI_VARIABLE_NON_VOLATILE |
                   EFI_VARIABLE_BOOTSERVICE_ACCESS |
                   EFI_VARIABLE_RUNTIME_ACCESS;

    EFI_STATUS get_status = uefi_call_wrapper(
        st->RuntimeServices->GetVariable,
        5,
        L"OsIndications",
        &gEfiGlobalVariableGuid,
        &attrs,
        &size,
        &indications
    );

    if (get_status == EFI_NOT_FOUND) {
        indications = 0;
    } else if (get_status == EFI_SUCCESS) {
        if (size != sizeof(indications)) {
            indications = 0;
        }
    } else {
        show_modal_notice(st, gop, input, "Could not read OsIndications", "Rebooting without firmware hint.");
    }

    indications |= EFI_OS_INDICATIONS_BOOT_TO_FW_UI;
    EFI_STATUS set_status = uefi_call_wrapper(
        st->RuntimeServices->SetVariable,
        5,
        L"OsIndications",
        &gEfiGlobalVariableGuid,
        attrs,
        sizeof(indications),
        &indications
    );
    if (set_status != EFI_SUCCESS) {
        show_modal_notice(st, gop, input, "Failed to request firmware settings", "Attempting warm reboot.");
    }

    uefi_call_wrapper(st->RuntimeServices->ResetSystem, 4, EfiResetWarm, EFI_SUCCESS, 0, NULL);
}

static void show_interrupt_menu(
    EFI_HANDLE image,
    EFI_SYSTEM_TABLE *st,
    StratGop *gop,
    StratInput *input,
    StratSlotDecision *decision
) {
    (void)image;
    if (st == NULL || gop == NULL || input == NULL || decision == NULL) {
        return;
    }

    static const CHAR8 *kMenuItems[7] = {
        "Boot normally",
        "Boot pinned image",
        "Safe mode",
        "Recovery options",
        "UEFI settings",
        "Reboot",
        "Power off",
    };

    INT32 focus = 0;
    while (1) {
        strat_gop_clear(gop, 0x08, 0x09, 0x0c);

        const CHAR8 *title = "Hey. You interrupted boot. No worries. What do you need?";
        strat_font_draw_text(gop, centered_text_x(gop, title), 40, title, 0xE6, 0xE8, 0xEA);

        for (INT32 i = 0; i < 7; i++) {
            INT32 item_y = 80 + (i * 20);
            if (i == focus) {
                strat_gop_fill_rect(gop, 32, item_y, 3, STRAT_FONT_HEIGHT, 0x5B, 0x9B, 0xD5);
                strat_font_draw_text(
                    gop, centered_text_x(gop, kMenuItems[i]), item_y, kMenuItems[i], 0xE6, 0xE8, 0xEA
                );
            } else {
                strat_font_draw_text(
                    gop, centered_text_x(gop, kMenuItems[i]), item_y, kMenuItems[i], 0x5A, 0x5C, 0x62
                );
            }
        }

        EFI_INPUT_KEY key;
        EFI_STATUS key_status = strat_input_wait(input, &key);
        if (key_status != EFI_SUCCESS) {
            continue;
        }

        if (key.ScanCode == 0x17) {
            return;
        }

        if (key.ScanCode == 0x01) {
            if (focus > 0) {
                focus--;
            }
            continue;
        }

        if (key.ScanCode == 0x02) {
            if (focus < 6) {
                focus++;
            }
            continue;
        }

        if (!(key.ScanCode == 0x00 && key.UnicodeChar == 0x000D)) {
            continue;
        }

        switch (focus) {
            case 0:
                return;
            case 1: {
                UINT8 pinned_slot = 0xFF;
                EFI_STATUS pinned_status = strat_efi_get_u8(
                    st->RuntimeServices, STRAT_EFI_VAR_NAME_PINNED_SLOT, &pinned_slot
                );
                if (pinned_status == EFI_SUCCESS && pinned_slot <= STRAT_SLOT_C) {
                    decision->slot = (StratSlotId)pinned_slot;
                    decision->kind = STRAT_SLOT_DECISION_BOOT;
                }
                return;
            }
            case 2:
                return;
            case 3:
                show_recovery_menu(st, gop, input);
                continue;
            case 4:
                request_firmware_ui_and_reset(st, gop, input);
                return;
            case 5:
                uefi_call_wrapper(st->RuntimeServices->ResetSystem, 4, EfiResetWarm, EFI_SUCCESS, 0, NULL);
                return;
            case 6:
                uefi_call_wrapper(st->RuntimeServices->ResetSystem, 4, EfiResetShutdown, EFI_SUCCESS, 0, NULL);
                return;
            default:
                return;
        }
    }
}

static const CHAR16 *slot_kernel_path(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"\\EFI\\STRAT\\SLOT_A\\vmlinuz.efi";
        case STRAT_SLOT_B: return L"\\EFI\\STRAT\\SLOT_B\\vmlinuz.efi";
        case STRAT_SLOT_C: return L"\\EFI\\STRAT\\SLOT_C\\vmlinuz.efi";
        default:           return NULL;
    }
}

static const CHAR16 *slot_root_device(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"/dev/sda2";
        case STRAT_SLOT_B: return L"/dev/sda3";
        case STRAT_SLOT_C: return L"/dev/sda4";
        default:           return NULL;
    }
}

static const CHAR16 *slot_initrd_path(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"\\EFI\\STRAT\\SLOT_A\\initramfs.img";
        case STRAT_SLOT_B: return L"\\EFI\\STRAT\\SLOT_B\\initramfs.img";
        case STRAT_SLOT_C: return L"\\EFI\\STRAT\\SLOT_C\\initramfs.img";
        default:           return NULL;
    }
}

static EFI_STATUS start_kernel_efi(EFI_HANDLE image, EFI_SYSTEM_TABLE *st,
                                    const CHAR16 *kernel_path,
                                    const CHAR16 *root_device,
                                    const CHAR16 *initrd_path) {
    EFI_STATUS status;
    EFI_LOADED_IMAGE *loaded = NULL;

    status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        image,
        &LoadedImageProtocol,
        (VOID **)&loaded
    );
    if (status != EFI_SUCCESS || loaded == NULL) {
        return status;
    }

    EFI_DEVICE_PATH *dp = FileDevicePath(loaded->DeviceHandle, (CHAR16 *)kernel_path);
    if (dp == NULL) {
        return EFI_NOT_FOUND;
    }

    EFI_HANDLE kernel_handle = NULL;
    status = uefi_call_wrapper(
        st->BootServices->LoadImage,
        6,
        FALSE,
        image,
        dp,
        NULL,
        0,
        &kernel_handle
    );
    if (status != EFI_SUCCESS) {
        return status;
    }

    // Build cmdline with explicit console targets for VM visibility.
    // Use a fixed-size CHAR16 buffer (512 chars is enough)
    // NOTE: keep verbose logging during bring-up.
    CHAR16 cmdline[512];
    // Use SPrint from gnu-efi (efilib.h) to build the string:
    SPrint(cmdline, sizeof(cmdline),
           L"root=%s rootfstype=erofs ro initrd=%s console=tty0 console=ttyS0,115200 loglevel=7",
           root_device, initrd_path);

    EFI_LOADED_IMAGE *kernel_image = NULL;
    EFI_STATUS li_status = uefi_call_wrapper(
        st->BootServices->HandleProtocol, 3,
        kernel_handle,
        &LoadedImageProtocol,
        (VOID **)&kernel_image
    );
    if (li_status == EFI_SUCCESS && kernel_image != NULL) {
        kernel_image->LoadOptions = cmdline;
        kernel_image->LoadOptionsSize = (UINT32)((StrLen(cmdline) + 1) * sizeof(CHAR16));
    }

    return uefi_call_wrapper(st->BootServices->StartImage, 3, kernel_handle, NULL, NULL);
}

// strat_maybe_init_vars: on a factory-fresh machine, EFI vars are absent.
// strat_slot_select() requires CONFIRMED (1) to boot, so without this function
// it halts immediately on first boot. We probe SLOT_A_STATUS: if EFI_NOT_FOUND,
// write all first-boot defaults. If the var already exists (any value), skip.
static EFI_STATUS strat_maybe_init_vars(EFI_RUNTIME_SERVICES *rt) {
    UINT8 probe = 0;
    EFI_STATUS probe_status = strat_efi_get_u8(rt, STRAT_EFI_VAR_NAME_SLOT_A_STATUS, &probe);
    if (probe_status != EFI_NOT_FOUND) {
        // Vars already present (or unreadable) â leave them alone.
        return EFI_SUCCESS;
    }

    // Factory-fresh: write all first-boot defaults.
    EFI_STATUS s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_SLOT_A_STATUS,  1, STRAT_EFI_VAR_ATTRS); // CONFIRMED
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_SLOT_B_STATUS,  0, STRAT_EFI_VAR_ATTRS); // STAGING
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_SLOT_C_STATUS,  0, STRAT_EFI_VAR_ATTRS); // STAGING
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_ACTIVE_SLOT,    0, STRAT_EFI_VAR_ATTRS); // SLOT_A
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_PINNED_SLOT,    0, STRAT_EFI_VAR_ATTRS); // NONE
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS,    0, STRAT_EFI_VAR_ATTRS);
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_BOOT_COUNT,     0, STRAT_EFI_VAR_ATTRS);
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_LAST_GOOD_SLOT, 0, STRAT_EFI_VAR_ATTRS); // SLOT_A
    if (s != EFI_SUCCESS) return s;

    return EFI_SUCCESS;
}

EFI_STATUS EFIAPI efi_main(EFI_HANDLE image, EFI_SYSTEM_TABLE *system_table) {
    InitializeLib(image, system_table);
    strat_efi_set_u8(system_table->RuntimeServices, (CHAR16 *)STRAT_SMOKE_EFI_MAIN_VAR, 1, STRAT_EFI_VAR_ATTRS);
    debugcon_log("StratBoot: efi_main entered\n");
    serial_log(system_table, "StratBoot: efi_main entered\n");

    StratGop gop;
    debugcon_log("StratBoot: calling gop_init\n");
    EFI_STATUS status = strat_gop_init(system_table, &gop);
    if (status != EFI_SUCCESS) {
        debugcon_log("StratBoot: GOP init failed\n");
        serial_log(system_table, "StratBoot: GOP init failed\n");
        Print(L"StratBoot: GOP init failed: %r\n", status);
        return status;
    }
    debugcon_log("StratBoot: gop ok\n");

    StratInput input;
    BOOLEAN input_ready = FALSE;
    if (strat_input_init(system_table, &input) == EFI_SUCCESS) {
        input_ready = TRUE;
    }
    debugcon_log("StratBoot: input ok\n");

    // Initialize EFI vars on first boot. On a factory-fresh machine all vars
    // are absent; this writes the defaults so slot selection can proceed.
    status = strat_maybe_init_vars(system_table->RuntimeServices);
    if (status != EFI_SUCCESS) {
        debugcon_log("StratBoot: var init failed\n");
        halt_with_message(system_table, &gop, "STRAT OS", "EFI var init failed");
        return EFI_ABORTED;
    }
    debugcon_log("StratBoot: vars ok\n");

    UINT8 home_status = STRAT_HOME_STATUS_HEALTHY;
    status = strat_efi_get_u8(system_table->RuntimeServices, STRAT_EFI_VAR_NAME_HOME_STATUS, &home_status);
    if (status != EFI_SUCCESS) {
        home_status = STRAT_HOME_STATUS_HEALTHY;
    }
    if (home_status > STRAT_HOME_STATUS_CORRUPT) {
        home_status = STRAT_HOME_STATUS_CORRUPT;
    }

    if (home_status == STRAT_HOME_STATUS_DEGRADED || home_status == STRAT_HOME_STATUS_CORRUPT) {
        debugcon_log("StratBoot: home corrupt path\n");
        if (!input_ready) {
            halt_with_message(system_table, &gop, "STRAT OS", "Home corruption detected");
            return EFI_ABORTED;
        }
        BOOLEAN continue_boot = show_home_corruption_screen(system_table, &gop, &input, home_status);
        if (!continue_boot) {
            halt_with_message(system_table, &gop, "STRAT OS", "Boot aborted by user");
            return EFI_ABORTED;
        }
    }

    debugcon_log("StratBoot: drawing boot screen\n");
    draw_boot_screen(&gop);
    debugcon_log("StratBoot: boot screen drawn, starting ESC poll\n");

    // Poll for ESC for up to 3 seconds (3,000,000 microseconds in 100ms ticks)
    BOOLEAN esc_pressed = FALSE;
    if (input_ready) {
        for (INTN i = 0; i < 30; i++) {
            EFI_INPUT_KEY key;
            EFI_STATUS poll_status = strat_input_poll(&input, &key);
            if (poll_status == EFI_SUCCESS && key.ScanCode == 0x17) { // ESC scan code
                esc_pressed = TRUE;
                break;
            }
            uefi_call_wrapper(system_table->BootServices->Stall, 1, 100000); // 100ms
        }
    }

    debugcon_log("StratBoot: reading slot state\n");
    StratSlotState slot_state;
    StratSlotDecision decision;
    status = strat_slot_read_state(system_table->RuntimeServices, &slot_state);
    if (status != EFI_SUCCESS) {
        debugcon_log("StratBoot: slot read failed\n");
        halt_with_message(system_table, &gop, "STRAT OS", "EFI var read failed");
        return EFI_ABORTED;
    }
    debugcon_log("StratBoot: slot state ok, selecting\n");

    status = strat_slot_select(&slot_state, &decision);
    if (status != EFI_SUCCESS) {
        debugcon_log("StratBoot: slot select failed\n");
        halt_with_message(system_table, &gop, "STRAT OS", "Slot select failed");
        return EFI_ABORTED;
    }
    debugcon_log("StratBoot: slot selected\n");

    if (esc_pressed && input_ready) {
        show_interrupt_menu(image, system_table, &gop, &input, &decision);
    }

    if (decision.kind == STRAT_SLOT_DECISION_RESET_PENDING) {
        StratResetState reset_state;
        strat_reset_read(system_table->RuntimeServices, &reset_state);
        draw_status(&gop, "STRAT OS", strat_reset_describe(reset_state.flags));

        EFI_STATUS reset_status = strat_execute_resets(system_table, reset_state.flags);
        if (reset_status != EFI_SUCCESS) {
            halt_with_message(system_table, &gop, "Reset failed", "Boot unchanged. Try again.");
            return EFI_ABORTED;
        }

        EFI_STATUS clear_status = strat_reset_clear(system_table->RuntimeServices);
        if (clear_status != EFI_SUCCESS) {
            halt_with_message(system_table, &gop, "Reset completed", "Could not clear reset flag");
            return EFI_ABORTED;
        }

        uefi_call_wrapper(system_table->RuntimeServices->ResetSystem, 4, EfiResetWarm, EFI_SUCCESS, 0, NULL);
        return EFI_ABORTED;
    }

    if (decision.kind == STRAT_SLOT_DECISION_HALT) {
        debugcon_log("StratBoot: HALT - no bootable slot\n");
        halt_with_message(system_table, &gop, "STRAT OS", "No bootable slot");
        return EFI_ABORTED;
    }

    const CHAR16 *kpath    = slot_kernel_path(decision.slot);
    const CHAR16 *rootdev  = slot_root_device(decision.slot);
    const CHAR16 *initrd   = slot_initrd_path(decision.slot);

    if (kpath == NULL || rootdev == NULL || initrd == NULL) {
        halt_with_message(system_table, &gop, "STRAT OS", "Invalid slot id");
        return EFI_ABORTED;
    }

    UINT8 boot_count = 0;
    strat_efi_get_u8(system_table->RuntimeServices, STRAT_EFI_VAR_NAME_BOOT_COUNT, &boot_count);
    if (boot_count < 255) {
        boot_count++;
        strat_efi_set_u8(system_table->RuntimeServices, STRAT_EFI_VAR_NAME_BOOT_COUNT,
                         boot_count, STRAT_EFI_VAR_ATTRS);
    }

    debugcon_log("StratBoot: booting slot\n");
    strat_efi_set_u8(system_table->RuntimeServices, (CHAR16 *)STRAT_SMOKE_BOOTING_SLOT_VAR, 1, STRAT_EFI_VAR_ATTRS);
    serial_log(system_table, "StratBoot: booting slot\n");
    Print(L"StratBoot: booting slot\n");
    draw_status(&gop, "STRAT OS", "Booting selected slot");
    status = start_kernel_efi(image, system_table, kpath, rootdev, initrd);
    if (status != EFI_SUCCESS) {
        halt_with_message(system_table, &gop, "STRAT OS", "Kernel load failed");
        return EFI_ABORTED;
    }

    return EFI_SUCCESS;
}
