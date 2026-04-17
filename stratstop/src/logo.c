#include "logo.h"
#include "font.h"
#include <math.h>
#include <stdint.h>

static void draw_circle(struct framebuffer *fb, uint32_t cx, uint32_t cy, uint32_t radius, uint8_t r, uint8_t g, uint8_t b, int fill) {
    if (fb == NULL || fb->data == NULL) {
        return;
    }

    int x0 = (int)cx;
    int y0 = (int)cy;
    int r_int = (int)radius;

    for (int y = -r_int; y <= r_int; y++) {
        for (int x = -r_int; x <= r_int; x++) {
            if (x * x + y * y <= r_int * r_int) {
                uint32_t px = (uint32_t)(x0 + x);
                uint32_t py = (uint32_t)(y0 + y);
                
                if (px < fb->width && py < fb->height) {
                    if (fill) {
                        fb_put_pixel(fb, px, py, r, g, b);
                    } else {
                        // Draw outline only - check if this is on the edge
                        int dist_sq = x * x + y * y;
                        int outer_sq = r_int * r_int;
                        int inner_sq = (r_int - 1) * (r_int - 1);
                        if (dist_sq >= inner_sq) {
                            fb_put_pixel(fb, px, py, r, g, b);
                        }
                    }
                }
            }
        }
    }
}

static uint32_t centered_text_x(struct framebuffer *fb, const char *text) {
    if (fb == NULL || text == NULL) {
        return 0;
    }

    size_t len = 0;
    while (text[len] != '\0') {
        len++;
    }

    uint32_t text_width = (uint32_t)(len * STRAT_FONT_WIDTH);
    if (text_width >= fb->width) {
        return 0;
    }

    return (fb->width - text_width) / 2;
}

void logo_draw_boot_screen(struct framebuffer *fb) {
    if (fb == NULL || fb->data == NULL) {
        return;
    }

    // Background: black
    fb_clear(fb);

    uint32_t cx = fb->width / 2;
    uint32_t cy = fb->height / 3;

    // Center logo mark: filled circle + inner S + two halo rings.
    draw_circle(fb, cx, cy, 18, 0xE6, 0xE8, 0xEA, 1);  // Light gray filled circle
    font_draw_char(fb, cx - (STRAT_FONT_WIDTH / 2), cy - (STRAT_FONT_HEIGHT / 2), 'S', 0x08, 0x09, 0x0c);  // Dark blue 'S'
    draw_circle(fb, cx, cy, 26, 0x3A, 0x3C, 0x42, 0);  // Medium gray ring
    draw_circle(fb, cx, cy, 34, 0x3A, 0x3C, 0x42, 0);  // Medium gray outer ring

    // Wordmark "STRAT OS"
    const char *wordmark = "STRAT OS";
    uint32_t wordmark_y = cy + 34 + 20;
    font_draw_text(fb, centered_text_x(fb, wordmark), wordmark_y, wordmark, 0xE6, 0xE8, 0xEA);
}
