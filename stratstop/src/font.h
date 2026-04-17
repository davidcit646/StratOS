#ifndef STRATSTOP_FONT_H
#define STRATSTOP_FONT_H

#include "fb.h"
#include <stdint.h>

#define STRAT_FONT_WIDTH 8
#define STRAT_FONT_HEIGHT 8

void font_draw_char(struct framebuffer *fb, uint32_t x, uint32_t y, char ch, uint8_t r, uint8_t g, uint8_t b);
void font_draw_text(struct framebuffer *fb, uint32_t x, uint32_t y, const char *text, uint8_t r, uint8_t g, uint8_t b);

#endif
