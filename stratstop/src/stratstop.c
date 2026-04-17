#include "fb.h"
#include "logo.h"
#include <unistd.h>

int main(void) {
    struct framebuffer fb;
    
    if (fb_open(&fb, "/dev/fb0") < 0) {
        return 1;
    }
    
    logo_draw_boot_screen(&fb);
    
    // Sleep for 1 second to display logo
    sleep(1);
    
    fb_close(&fb);
    
    return 0;
}
