#include <efi.h>
#include <efilib.h>
#include <efiser.h>
/* Simple FS + EFI_FILE types: gnu-efi <efi.h> → efiprot.h (no EDK2 protocol/SimpleFileSystem.h). */
#include "gop.h"
#include "font.h"
#include "input.h"
#include "slot.h"
#include "reset.h"
#include "partition.h"

#define STRAT_HOME_STATUS_HEALTHY  0
#define STRAT_HOME_STATUS_DEGRADED 1
#define STRAT_HOME_STATUS_CORRUPT  2

static BOOLEAN show_confirm_prompt(StratGop *gop, StratInput *input);

#ifdef DEBUG
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
#endif

#ifdef DEBUG
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

    // Best-effort normalization for firmware serial sinks.
    uefi_call_wrapper(serial->SetAttributes, 7, serial,
                      115200, 0, 0, NoParity, 8, OneStopBit);

    UINTN len = strlena(msg);
    if (len == 0) {
        return;
    }

    uefi_call_wrapper(serial->Write, 3, serial, &len, (VOID *)msg);
}
#endif

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

static const CHAR16 *slot_name(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return L"SLOT_A";
        case STRAT_SLOT_B: return L"SLOT_B";
        case STRAT_SLOT_C: return L"SLOT_C";
        default:           return NULL;
    }
}

static CHAR16 slot_root_partuuid[3][37];
/* GPT partition names CONFIG / STRAT_CACHE / HOME — passed through the kernel cmdline for initramfs. */
static CHAR16 misc_partuuid[3][37];

static BOOLEAN gpt_header_valid(const UINT8 *hdr, UINTN size) {
    if (hdr == NULL || size < 8) {
        return FALSE;
    }
    return (CompareMem(hdr, "EFI PART", 8) == 0);
}

static BOOLEAN gpt_entry_name_equals(const UINT8 *entry, const CHAR16 *partition_name) {
    if (entry == NULL || partition_name == NULL) {
        return FALSE;
    }

    CHAR16 name_field[37];
    const CHAR16 *raw = (const CHAR16 *)(entry + 56);
    for (UINTN i = 0; i < 36; i++) {
        name_field[i] = raw[i];
    }
    name_field[36] = L'\0';

    return (StrCmp(name_field, (CHAR16 *)partition_name) == 0);
}

static UINT32 read_le32(const UINT8 *ptr) {
    return (UINT32)ptr[0]
         | ((UINT32)ptr[1] << 8)
         | ((UINT32)ptr[2] << 16)
         | ((UINT32)ptr[3] << 24);
}

static UINT64 read_le64(const UINT8 *ptr) {
    return (UINT64)read_le32(ptr)
         | ((UINT64)read_le32(ptr + 4) << 32);
}

static __attribute__((unused)) EFI_STATUS find_disk_block_io_from_loaded_image(
    EFI_SYSTEM_TABLE *st,
    EFI_HANDLE image,
    EFI_BLOCK_IO **out_disk_bio
) {
    if (st == NULL || st->BootServices == NULL || image == NULL || out_disk_bio == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    *out_disk_bio = NULL;

    EFI_LOADED_IMAGE *loaded = NULL;
    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        image,
        &LoadedImageProtocol,
        (VOID **)&loaded
    );
    if (status != EFI_SUCCESS || loaded == NULL) {
        return (status != EFI_SUCCESS) ? status : EFI_NOT_FOUND;
    }

    EFI_DEVICE_PATH *dp = NULL;
    status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        loaded->DeviceHandle,
        &DevicePathProtocol,
        (VOID **)&dp
    );
    if (status != EFI_SUCCESS || dp == NULL) {
        return (status != EFI_SUCCESS) ? status : EFI_NOT_FOUND;
    }

    EFI_DEVICE_PATH_PROTOCOL *node = (EFI_DEVICE_PATH_PROTOCOL *)dp;
    while (!IsDevicePathEnd(node)) {
        if (DevicePathType(node) == MEDIA_DEVICE_PATH && DevicePathSubType(node) == MEDIA_HARDDRIVE_DP) {
            UINTN prefix_len = (UINTN)((UINT8 *)node - (UINT8 *)dp);
            if (prefix_len < sizeof(EFI_DEVICE_PATH_PROTOCOL)) {
                return EFI_NOT_FOUND;
            }

            EFI_HANDLE *handles = NULL;
            UINTN handle_count = 0;
            status = uefi_call_wrapper(
                st->BootServices->LocateHandleBuffer,
                5,
                ByProtocol,
                &BlockIoProtocol,
                NULL,
                &handle_count,
                &handles
            );
            if (status != EFI_SUCCESS) {
                return status;
            }

            EFI_BLOCK_IO *disk_bio = NULL;
            for (UINTN i = 0; i < handle_count; i++) {
                EFI_BLOCK_IO *bio = NULL;
                EFI_STATUS bio_status = uefi_call_wrapper(
                    st->BootServices->HandleProtocol,
                    3,
                    handles[i],
                    &BlockIoProtocol,
                    (VOID **)&bio
                );
                if (bio_status != EFI_SUCCESS || bio == NULL || bio->Media == NULL) {
                    continue;
                }
                if (bio->Media->LogicalPartition) {
                    continue;
                }

                EFI_DEVICE_PATH *cand_dp = NULL;
                EFI_STATUS dp_status = uefi_call_wrapper(
                    st->BootServices->HandleProtocol,
                    3,
                    handles[i],
                    &DevicePathProtocol,
                    (VOID **)&cand_dp
                );
                if (dp_status != EFI_SUCCESS || cand_dp == NULL) {
                    continue;
                }

                if (CompareMem(cand_dp, dp, prefix_len) == 0) {
                    disk_bio = bio;
                    break;
                }
            }

            if (handles != NULL) {
                uefi_call_wrapper(st->BootServices->FreePool, 1, handles);
            }
            if (disk_bio == NULL) {
                // Fallback: pick the first whole-disk BlockIo that looks like GPT.
                EFI_HANDLE *all_handles = NULL;
                UINTN all_count = 0;
                EFI_STATUS list_status = uefi_call_wrapper(
                    st->BootServices->LocateHandleBuffer,
                    5,
                    ByProtocol,
                    &BlockIoProtocol,
                    NULL,
                    &all_count,
                    &all_handles
                );
                if (list_status != EFI_SUCCESS) {
                    return list_status;
                }

                EFI_BLOCK_IO *candidate_disk = NULL;
                for (UINTN i = 0; i < all_count; i++) {
                    EFI_BLOCK_IO *bio = NULL;
                    EFI_STATUS bio_status = uefi_call_wrapper(
                        st->BootServices->HandleProtocol,
                        3,
                        all_handles[i],
                        &BlockIoProtocol,
                        (VOID **)&bio
                    );
                    if (bio_status != EFI_SUCCESS || bio == NULL || bio->Media == NULL) {
                        continue;
                    }
                    if (bio->Media->LogicalPartition) {
                        continue;
                    }
                    if (!bio->Media->MediaPresent || bio->Media->BlockSize == 0) {
                        continue;
                    }

                    UINTN bs = bio->Media->BlockSize;
                    UINT8 *hdr = AllocatePool(bs);
                    if (hdr == NULL) {
                        continue;
                    }
                    EFI_STATUS read_status = uefi_call_wrapper(
                        bio->ReadBlocks,
                        5,
                        bio,
                        bio->Media->MediaId,
                        1,
                        bs,
                        hdr
                    );
                    if (read_status == EFI_SUCCESS && gpt_header_valid(hdr, bs)) {
                        candidate_disk = bio;
                        FreePool(hdr);
                        break;
                    }
                    FreePool(hdr);
                }

                if (all_handles != NULL) {
                    uefi_call_wrapper(st->BootServices->FreePool, 1, all_handles);
                }
                if (candidate_disk == NULL) {
                    return EFI_NOT_FOUND;
                }

                *out_disk_bio = candidate_disk;
                return EFI_SUCCESS;
            }
            *out_disk_bio = disk_bio;
            return EFI_SUCCESS;
        }
        node = NextDevicePathNode(node);
    }

    return EFI_NOT_FOUND;
}

static EFI_STATUS gpt_find_partuuid_by_name(
    EFI_BLOCK_IO *disk_bio,
    const CHAR16 *partition_name,
    CHAR16 *out_partuuid,
    UINTN out_size
) {
    if (disk_bio == NULL || disk_bio->Media == NULL || partition_name == NULL || out_partuuid == NULL || out_size < 37) {
        return EFI_INVALID_PARAMETER;
    }
    if (!disk_bio->Media->MediaPresent) {
        return EFI_NO_MEDIA;
    }
    if (disk_bio->Media->BlockSize == 0) {
        return EFI_BAD_BUFFER_SIZE;
    }

    UINTN block_size = disk_bio->Media->BlockSize;
    UINT8 *gpt_header = AllocatePool(block_size);
    if (gpt_header == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    EFI_STATUS status = uefi_call_wrapper(
        disk_bio->ReadBlocks,
        5,
        disk_bio,
        disk_bio->Media->MediaId,
        1,
        block_size,
        gpt_header
    );
    if (status != EFI_SUCCESS) {
        FreePool(gpt_header);
        return status;
    }

    if (!gpt_header_valid(gpt_header, block_size)) {
        FreePool(gpt_header);
        return EFI_NOT_FOUND;
    }

    UINT64 part_entry_lba = 0;
    UINT32 num_part_entries = 0;
    UINT32 part_entry_size = 0;
    UINT64 header_lba = 0;
    UINT32 header_size = 0;
    part_entry_lba = read_le64(gpt_header + 72);
    num_part_entries = read_le32(gpt_header + 80);
    part_entry_size = read_le32(gpt_header + 84);
    header_lba = read_le64(gpt_header + 24);
    header_size = read_le32(gpt_header + 12);
    FreePool(gpt_header);

    if (header_lba != 1) {
        return EFI_NOT_FOUND;
    }
    if (header_size < 92 || header_size > block_size) {
        return EFI_NOT_FOUND;
    }
    if (part_entry_lba < 2 || num_part_entries == 0 || part_entry_size < 128) {
        return EFI_NOT_FOUND;
    }
    if (part_entry_size > 4096 || (part_entry_size % 8) != 0) {
        return EFI_NOT_FOUND;
    }
    if (num_part_entries > 4096) {
        return EFI_NOT_FOUND;
    }
    UINT64 table_bytes = (UINT64)num_part_entries * (UINT64)part_entry_size;
    UINT64 table_blocks = (table_bytes + (UINT64)block_size - 1ULL) / (UINT64)block_size;
    if (table_blocks == 0) {
        return EFI_NOT_FOUND;
    }
    if (part_entry_lba > (UINT64)disk_bio->Media->LastBlock) {
        return EFI_NOT_FOUND;
    }
    if (part_entry_lba + table_blocks - 1ULL > (UINT64)disk_bio->Media->LastBlock) {
        return EFI_NOT_FOUND;
    }

    UINT64 entries_per_lba = (UINT64)block_size / (UINT64)part_entry_size;
    if (entries_per_lba == 0) {
        return EFI_NOT_FOUND;
    }

    UINT8 *entry_block = AllocatePool(block_size);
    if (entry_block == NULL) {
        return EFI_OUT_OF_RESOURCES;
    }

    for (UINT32 i = 0; i < num_part_entries; i++) {
        UINT64 idx = (UINT64)i;
        UINT64 entry_lba = part_entry_lba + (idx / entries_per_lba);
        UINTN entry_offset = (UINTN)((idx % entries_per_lba) * (UINT64)part_entry_size);
        if (entry_lba > (UINT64)disk_bio->Media->LastBlock) {
            break;
        }
        if (entry_offset + part_entry_size > block_size) {
            continue;
        }

        status = uefi_call_wrapper(
            disk_bio->ReadBlocks,
            5,
            disk_bio,
            disk_bio->Media->MediaId,
            (EFI_LBA)entry_lba,
            block_size,
            entry_block
        );
        if (status != EFI_SUCCESS) {
            FreePool(entry_block);
            return status;
        }

        const UINT8 *entry = entry_block + entry_offset;
        if (!gpt_entry_name_equals(entry, partition_name)) {
            continue;
        }

        EFI_GUID *part_guid = (EFI_GUID *)(entry + 16);
        SPrint(out_partuuid, out_size * sizeof(CHAR16),
               L"%08x-%04x-%04x-%02x%02x-%02x%02x%02x%02x%02x%02x",
               part_guid->Data1,
               part_guid->Data2,
               part_guid->Data3,
               part_guid->Data4[0], part_guid->Data4[1],
               part_guid->Data4[2], part_guid->Data4[3],
               part_guid->Data4[4], part_guid->Data4[5],
               part_guid->Data4[6], part_guid->Data4[7]);

        FreePool(entry_block);
        return EFI_SUCCESS;
    }

    FreePool(entry_block);
    return EFI_NOT_FOUND;
}

static EFI_STATUS gpt_find_partuuid_on_any_disk(
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *partition_name,
    CHAR16 *out_partuuid,
    UINTN out_size
) {
    if (st == NULL || st->BootServices == NULL || partition_name == NULL || out_partuuid == NULL || out_size < 37) {
        return EFI_INVALID_PARAMETER;
    }

    EFI_HANDLE *handles = NULL;
    UINTN handle_count = 0;
    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->LocateHandleBuffer,
        5,
        ByProtocol,
        &BlockIoProtocol,
        NULL,
        &handle_count,
        &handles
    );
    if (status != EFI_SUCCESS) {
        return status;
    }

    EFI_STATUS result = EFI_NOT_FOUND;
    for (UINTN i = 0; i < handle_count; i++) {
        EFI_BLOCK_IO *bio = NULL;
        EFI_STATUS bio_status = uefi_call_wrapper(
            st->BootServices->HandleProtocol,
            3,
            handles[i],
            &BlockIoProtocol,
            (VOID **)&bio
        );
        if (bio_status != EFI_SUCCESS || bio == NULL || bio->Media == NULL) {
            continue;
        }
        if (bio->Media->LogicalPartition) {
            continue;
        }
        if (!bio->Media->MediaPresent || bio->Media->BlockSize == 0) {
            continue;
        }

        // Quick GPT signature probe at LBA1 to avoid expensive scans.
        UINTN bs = bio->Media->BlockSize;
        UINT8 *hdr = AllocatePool(bs);
        if (hdr == NULL) {
            continue;
        }
        EFI_STATUS read_status = uefi_call_wrapper(
            bio->ReadBlocks,
            5,
            bio,
            bio->Media->MediaId,
            1,
            bs,
            hdr
        );
        BOOLEAN is_gpt = (read_status == EFI_SUCCESS && gpt_header_valid(hdr, bs));
        FreePool(hdr);
        if (!is_gpt) {
            continue;
        }

        EFI_STATUS uuid_status = gpt_find_partuuid_by_name(bio, partition_name, out_partuuid, out_size);
        if (uuid_status == EFI_SUCCESS) {
            result = EFI_SUCCESS;
            break;
        }
        result = uuid_status;
    }

    if (handles != NULL) {
        uefi_call_wrapper(st->BootServices->FreePool, 1, handles);
    }
    return result;
}

static const CHAR16 *slot_root_device(StratSlotId slot) {
    switch (slot) {
        case STRAT_SLOT_A: return slot_root_partuuid[0];
        case STRAT_SLOT_B: return slot_root_partuuid[1];
        case STRAT_SLOT_C: return slot_root_partuuid[2];
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

/* LoadedImage->LoadOptions must point at firmware heap; stack goes invalid after return. */
#define STRAT_KERNEL_CMDLINE_MAX   1024
#define STRAT_KERNEL_CMDLINE_BYTES (STRAT_KERNEL_CMDLINE_MAX * sizeof(CHAR16))

static EFI_STATUS strat_start_kernel_with_pooled_cmdline(
    EFI_SYSTEM_TABLE *st,
    EFI_HANDLE kernel_handle,
    CHAR16 *cmdline_pool
) {
    if (st == NULL || cmdline_pool == NULL) {
        if (cmdline_pool != NULL) {
            FreePool(cmdline_pool);
        }
        return EFI_INVALID_PARAMETER;
    }

    EFI_LOADED_IMAGE *kernel_image = NULL;
    EFI_STATUS li_status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        kernel_handle,
        &LoadedImageProtocol,
        (VOID **)&kernel_image
    );

    BOOLEAN attached = (li_status == EFI_SUCCESS && kernel_image != NULL);
    if (attached) {
        kernel_image->LoadOptions = cmdline_pool;
        kernel_image->LoadOptionsSize =
            (UINT32)((StrLen(cmdline_pool) + 1) * sizeof(CHAR16));
    }

    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->StartImage,
        3,
        kernel_handle,
        NULL,
        NULL
    );

    if (!attached || status != EFI_SUCCESS) {
        FreePool(cmdline_pool);
    }
    return status;
}

static EFI_STATUS start_kernel_efi(
    EFI_HANDLE image,
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *kernel_path,
    const CHAR16 *root_device,
    const CHAR16 *config_partuuid,
    const CHAR16 *apps_partuuid,
    const CHAR16 *home_partuuid,
    const CHAR16 *initrd_path
) {
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
    // Include CONFIG / STRAT_CACHE (apps) / HOME PARTUUIDs so virtio (/dev/vda*) boots do not depend on guessed device names.
    CHAR16 *cmdline = AllocatePool(STRAT_KERNEL_CMDLINE_BYTES);
    if (cmdline == NULL) {
        uefi_call_wrapper(st->BootServices->UnloadImage, 1, kernel_handle);
        return EFI_OUT_OF_RESOURCES;
    }
    SPrint(
        cmdline,
        STRAT_KERNEL_CMDLINE_BYTES,
        L"root=PARTUUID=%s rootfstype=erofs ro "
        L"config=PARTUUID=%s apps=PARTUUID=%s home=PARTUUID=%s "
        L"initrd=%s loglevel=4 console=tty0 console=ttyS0,115200",
        root_device,
        config_partuuid,
        apps_partuuid,
        home_partuuid,
        initrd_path
    );

    return strat_start_kernel_with_pooled_cmdline(st, kernel_handle, cmdline);
}

/* UEFI ISO / removable layout: marker file next to StratBoot on the FAT volume. */
static EFI_STATUS strat_detect_live_medium(
    EFI_HANDLE image,
    EFI_SYSTEM_TABLE *st,
    BOOLEAN *out_live
) {
    if (out_live == NULL || st == NULL || st->BootServices == NULL) {
        return EFI_INVALID_PARAMETER;
    }
    *out_live = FALSE;

    EFI_LOADED_IMAGE *loaded = NULL;
    EFI_STATUS status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        image,
        &LoadedImageProtocol,
        (VOID **)&loaded
    );
    if (status != EFI_SUCCESS || loaded == NULL) {
        return EFI_SUCCESS;
    }

    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *volume = NULL;
    status = uefi_call_wrapper(
        st->BootServices->HandleProtocol,
        3,
        loaded->DeviceHandle,
        &gEfiSimpleFileSystemProtocolGuid,
        (VOID **)&volume
    );
    if (status != EFI_SUCCESS || volume == NULL) {
        return EFI_SUCCESS;
    }

    EFI_FILE_PROTOCOL *root = NULL;
    status = uefi_call_wrapper(volume->OpenVolume, 2, volume, &root);
    if (status != EFI_SUCCESS || root == NULL) {
        return EFI_SUCCESS;
    }

    EFI_FILE_PROTOCOL *marker = NULL;
    status = uefi_call_wrapper(
        root->Open,
        5,
        root,
        &marker,
        L"\\EFI\\STRAT\\LIVE",
        EFI_FILE_MODE_READ,
        0
    );
    if (status == EFI_SUCCESS && marker != NULL) {
        uefi_call_wrapper(marker->Close, 1, marker);
        *out_live = TRUE;
    }
    uefi_call_wrapper(root->Close, 1, root);
    return EFI_SUCCESS;
}

static EFI_STATUS start_kernel_efi_live(
    EFI_HANDLE image,
    EFI_SYSTEM_TABLE *st,
    const CHAR16 *kernel_path,
    const CHAR16 *initrd_path
) {
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

    CHAR16 *cmdline = AllocatePool(STRAT_KERNEL_CMDLINE_BYTES);
    if (cmdline == NULL) {
        uefi_call_wrapper(st->BootServices->UnloadImage, 1, kernel_handle);
        return EFI_OUT_OF_RESOURCES;
    }
    SPrint(
        cmdline,
        STRAT_KERNEL_CMDLINE_BYTES,
        L"strat.live=1 strat.live_iso=1 "
        L"initrd=%s loglevel=4 console=tty0 console=ttyS0,115200",
        initrd_path
    );

    return strat_start_kernel_with_pooled_cmdline(st, kernel_handle, cmdline);
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
    s = strat_efi_set_u8(
        rt,
        STRAT_EFI_VAR_NAME_PINNED_SLOT,
        (UINT8)STRAT_SLOT_NONE,
        STRAT_EFI_VAR_ATTRS
    ); /* 0xFF — do not pin; 0 would mean SLOT_A */
    if (s != EFI_SUCCESS) return s;
    s = strat_efi_set_u8(rt, STRAT_EFI_VAR_NAME_RESET_FLAGS,    0, STRAT_EFI_VAR_ATTRS);
    if (s != EFI_SUCCESS) return s;

    return EFI_SUCCESS;
}

EFI_STATUS EFIAPI efi_main(EFI_HANDLE image, EFI_SYSTEM_TABLE *system_table) {
    InitializeLib(image, system_table);
#ifdef DEBUG
    debugcon_log("StratBoot: efi_main entered\n");
    serial_log(system_table, "StratBoot: efi_main entered\n");
#endif

    StratGop gop;
#ifdef DEBUG
    debugcon_log("StratBoot: calling gop_init\n");
#endif
    EFI_STATUS status = strat_gop_init(system_table, &gop);
    if (status != EFI_SUCCESS) {
#ifdef DEBUG
        debugcon_log("StratBoot: GOP init failed\n");
        serial_log(system_table, "StratBoot: GOP init failed\n");
#endif
        Print(L"StratBoot: GOP init failed: %r\n", status);
        return status;
    }
#ifdef DEBUG
    debugcon_log("StratBoot: gop ok\n");
#endif

    StratInput input;
    BOOLEAN input_ready = FALSE;
    if (strat_input_init(system_table, &input) == EFI_SUCCESS) {
        input_ready = TRUE;
    }
#ifdef DEBUG
    debugcon_log("StratBoot: input ok\n");
#endif

    // Initialize EFI vars on first boot. On a factory-fresh machine all vars
    // are absent; this writes the defaults so slot selection can proceed.
    status = strat_maybe_init_vars(system_table->RuntimeServices);
    if (status != EFI_SUCCESS) {
#ifdef DEBUG
        debugcon_log("StratBoot: var init failed\n");
#endif
        halt_with_message(system_table, &gop, "STRAT OS", "EFI var init failed");
        return EFI_ABORTED;
    }
#ifdef DEBUG
    debugcon_log("StratBoot: vars ok\n");
#endif

    BOOLEAN strat_live_medium = FALSE;
    strat_detect_live_medium(image, system_table, &strat_live_medium);

    if (!strat_live_medium) {
        EFI_STATUS upd = strat_slot_process_update_request(system_table, system_table->RuntimeServices);
        if (upd == EFI_SUCCESS) {
            Print(L"StratBoot: staged update verified; activating target slot\n");
        } else if (upd == EFI_NOT_READY) {
            /* No pending update, or vars absent — normal boot */
        } else if (upd == EFI_SECURITY_VIOLATION) {
            halt_with_message(system_table, &gop, "STRAT OS", "Staged update failed: hash mismatch");
            return EFI_ABORTED;
        } else {
            Print(L"StratBoot: staged update failed: %r\n", upd);
            halt_with_message(system_table, &gop, "STRAT OS", "Staged update failed");
            return EFI_ABORTED;
        }
    }

    for (INT32 i = 0; i < 3; i++) {
        slot_root_partuuid[i][0] = L'\0';
        misc_partuuid[i][0] = L'\0';
    }

    for (StratSlotId slot = STRAT_SLOT_A; slot <= STRAT_SLOT_C; slot = (StratSlotId)(slot + 1)) {
        const CHAR16 *name = slot_name(slot);
        if (name == NULL) continue;

        INT32 idx = (INT32)slot - (INT32)STRAT_SLOT_A;
        if (idx >= 0 && idx < 3) {
            EFI_STATUS uuid_status = gpt_find_partuuid_on_any_disk(system_table, name, slot_root_partuuid[idx], 37);
            if (uuid_status != EFI_SUCCESS) {
#ifdef DEBUG
                debugcon_log("StratBoot: failed to read PARTUUID\n");
#endif
                slot_root_partuuid[idx][0] = L'\0';
                Print(L"StratBoot: PARTUUID read failed for %s: status=0x%lx\n", name, (UINTN)uuid_status);
            } else {
                Print(L"StratBoot: %s PARTUUID=%s\n", name, slot_root_partuuid[idx]);
            }
        }
    }

    {
        const CHAR16 *misc_names[3] = { L"CONFIG", L"STRAT_CACHE", L"HOME" };
        for (UINTN i = 0; i < 3; i++) {
            EFI_STATUS mu = gpt_find_partuuid_on_any_disk(system_table, misc_names[i], misc_partuuid[i], 37);
            if (mu != EFI_SUCCESS) {
                misc_partuuid[i][0] = L'\0';
                Print(L"StratBoot: PARTUUID read failed for %s: status=0x%lx\n", misc_names[i], (UINTN)mu);
            } else {
                Print(L"StratBoot: %s PARTUUID=%s\n", misc_names[i], misc_partuuid[i]);
            }
        }
    }

    UINT8 home_status = STRAT_HOME_STATUS_HEALTHY;
    status = strat_efi_get_u8(system_table->RuntimeServices, STRAT_EFI_VAR_NAME_HOME_STATUS, &home_status);
    if (status != EFI_SUCCESS) {
        home_status = STRAT_HOME_STATUS_HEALTHY;
    }
    if (home_status > STRAT_HOME_STATUS_CORRUPT) {
        home_status = STRAT_HOME_STATUS_CORRUPT;
    }

    if (home_status == STRAT_HOME_STATUS_DEGRADED || home_status == STRAT_HOME_STATUS_CORRUPT) {
#ifdef DEBUG
        debugcon_log("StratBoot: home corrupt path\n");
#endif
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

#ifdef DEBUG
    debugcon_log("StratBoot: drawing boot screen\n");
#endif
    draw_boot_screen(&gop);
#ifdef DEBUG
    debugcon_log("StratBoot: boot screen drawn, starting ESC poll\n");
#endif

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

#ifdef DEBUG
    debugcon_log("StratBoot: reading slot state\n");
#endif
    StratSlotState slot_state;
    StratSlotDecision decision;
    status = strat_slot_read_state(system_table->RuntimeServices, &slot_state);
    if (status != EFI_SUCCESS) {
#ifdef DEBUG
        debugcon_log("StratBoot: slot read failed\n");
#endif
        halt_with_message(system_table, &gop, "STRAT OS", "EFI var read failed");
        return EFI_ABORTED;
    }
#ifdef DEBUG
    debugcon_log("StratBoot: slot state ok, selecting\n");
#endif

    status = strat_slot_select(&slot_state, &decision);
    if (status != EFI_SUCCESS) {
#ifdef DEBUG
        debugcon_log("StratBoot: slot select failed\n");
#endif
        halt_with_message(system_table, &gop, "STRAT OS", "Slot select failed");
        return EFI_ABORTED;
    }
#ifdef DEBUG
    debugcon_log("StratBoot: slot selected\n");
#endif

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
#ifdef DEBUG
        debugcon_log("StratBoot: HALT - no bootable slot\n");
#endif
        halt_with_message(system_table, &gop, "STRAT OS", "No bootable slot");
        return EFI_ABORTED;
    }

    /* Live ISO only carries SLOT_A payloads under EFI (see scripts/build-live-iso.sh). */
    StratSlotId boot_slot = strat_live_medium ? STRAT_SLOT_A : decision.slot;
    const CHAR16 *kpath    = slot_kernel_path(boot_slot);
    const CHAR16 *rootdev  = slot_root_device(boot_slot);
    const CHAR16 *initrd   = slot_initrd_path(boot_slot);

    if (kpath == NULL || rootdev == NULL || initrd == NULL) {
        halt_with_message(system_table, &gop, "STRAT OS", "Invalid slot id");
        return EFI_ABORTED;
    }

    if (!strat_live_medium && rootdev[0] == L'\0') {
        Print(L"StratBoot: Missing slot PARTUUID\n");
        halt_with_message(system_table, &gop, "STRAT OS", "Missing slot PARTUUID");
        return EFI_ABORTED;
    }

    if (!strat_live_medium &&
        (misc_partuuid[0][0] == L'\0' || misc_partuuid[1][0] == L'\0' ||
         misc_partuuid[2][0] == L'\0')) {
        Print(L"StratBoot: Missing CONFIG / STRAT_CACHE / HOME PARTUUID(s)\n");
        halt_with_message(system_table, &gop, "STRAT OS", "Storage layout incomplete");
        return EFI_ABORTED;
    }

#ifdef DEBUG
    debugcon_log("StratBoot: booting slot\n");
    serial_log(system_table, "StratBoot: booting slot\n");
#endif
    Print(L"StratBoot: booting slot\n");
    draw_status(&gop, "STRAT OS", "Booting selected slot");

    if (strat_live_medium) {
        status = start_kernel_efi_live(image, system_table, kpath, initrd);
    } else {
        status = start_kernel_efi(
            image,
            system_table,
            kpath,
            rootdev,
            misc_partuuid[0],
            misc_partuuid[1],
            misc_partuuid[2],
            initrd
        );
    }
    if (status != EFI_SUCCESS) {
        halt_with_message(system_table, &gop, "STRAT OS", "Kernel load failed");
        return EFI_ABORTED;
    }

    return EFI_SUCCESS;
}
