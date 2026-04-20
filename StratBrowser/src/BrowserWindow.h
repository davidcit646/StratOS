#pragma once

#include <gtk/gtk.h>
#include <adwaita.h>
#include <string>

class BrowserWindow {
public:
    BrowserWindow(AdwApplication* app);
    ~BrowserWindow();

    void present();

private:
    void setup_ui();
    void setup_styling();
    
    GtkWidget* create_toolbar();
    GtkWidget* create_location_bar();
    GtkWidget* create_status_bar();
    GtkWidget* create_content_area();

    GtkWindow* m_window;
    GtkWidget* m_main_box;
    GtkWidget* m_url_entry;
    GtkWidget* m_web_view; // This would be the Ladybird widget
    GtkWidget* m_status_label;
};
