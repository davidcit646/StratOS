#include <adwaita.h>

#include "BrowserWindow.h"

static void on_activate(AdwApplication* app) {
    BrowserWindow* window = new BrowserWindow(app);
    window->present();
}

int main(int argc, char** argv) {
    AdwApplication* app = adw_application_new("org.stratos.stratbrowser", G_APPLICATION_FLAGS_NONE);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), NULL);
    return g_application_run(G_APPLICATION(app), argc, argv);
}
