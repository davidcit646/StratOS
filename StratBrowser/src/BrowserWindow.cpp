#include "BrowserWindow.h"
#include <iostream>

BrowserWindow::BrowserWindow(AdwApplication* app) {
    m_window = GTK_WINDOW(adw_application_window_new(GTK_APPLICATION(app)));
    gtk_window_set_title(m_window, "Netscape Navigator (Ladybird Engine)");
    gtk_window_set_default_size(m_window, 1024, 768);

    m_main_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    adw_application_window_set_content(ADW_APPLICATION_WINDOW(m_window), m_main_box);

    setup_styling();
    setup_ui();
}

BrowserWindow::~BrowserWindow() {}

void BrowserWindow::present() {
    gtk_window_present(m_window);
}

void BrowserWindow::setup_styling() {
    GtkCssProvider* provider = gtk_css_provider_new();
    const char* css = 
        "window, .main-bg { background-color: #c0c0c0; color: black; font-family: 'Sans', 'Arial', sans-serif; }"
        "button { background-color: #c0c0c0; border: 2px solid; border-color: #ffffff #808080 #808080 #ffffff; border-radius: 0; padding: 4px 8px; font-weight: bold; }"
        "button:hover { background-color: #d0d0d0; }"
        "button:active { border-color: #808080 #ffffff #ffffff #808080; }"
        "entry { background-color: white; border: 2px inset #808080; border-radius: 0; color: black; padding: 4px; }"
        ".toolbar { padding: 4px; border-bottom: 1px solid #808080; }"
        ".status-bar { border-top: 1px solid #ffffff; padding: 2px 8px; font-size: 0.9em; }"
        ".logo-box { border: 2px inset #808080; background-color: #000040; min-width: 48px; min-height: 48px; margin-left: 4px; }"
    ;
    gtk_css_provider_load_from_data(provider, css, -1);
    gtk_style_context_add_provider_for_display(gdk_display_get_default(), GTK_STYLE_PROVIDER(provider), GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
}

void BrowserWindow::setup_ui() {
    // 1. Menu Bar (Mock)
    GtkWidget* menu_bar = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 10);
    gtk_widget_add_css_class(menu_bar, "toolbar");
    const char* menus[] = {"File", "Edit", "View", "Go", "Bookmarks", "Options", "Directory", "Window", "Help"};
    for (const char* m : menus) {
        GtkWidget* label = gtk_label_new(m);
        gtk_box_append(GTK_BOX(menu_bar), label);
    }
    gtk_box_append(GTK_BOX(m_main_box), menu_bar);

    // 2. Toolbar & Logo
    GtkWidget* top_row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    GtkWidget* toolbar = create_toolbar();
    gtk_widget_set_hexpand(toolbar, TRUE);
    
    GtkWidget* logo_box = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_widget_add_css_class(logo_box, "logo-box");
    GtkWidget* logo_label = gtk_label_new("N");
    gtk_widget_set_halign(logo_label, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(logo_label, GTK_ALIGN_CENTER);
    gtk_box_append(GTK_BOX(logo_box), logo_label);

    gtk_box_append(GTK_BOX(top_row), toolbar);
    gtk_box_append(GTK_BOX(top_row), logo_box);
    gtk_box_append(GTK_BOX(m_main_box), top_row);

    // 3. Location Bar
    gtk_box_append(GTK_BOX(m_main_box), create_location_bar());

    // 4. Content Area
    gtk_box_append(GTK_BOX(m_main_box), create_content_area());

    // 5. Status Bar
    gtk_box_append(GTK_BOX(m_main_box), create_status_bar());
}

GtkWidget* BrowserWindow::create_toolbar() {
    GtkWidget* box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 4);
    gtk_widget_add_css_class(box, "toolbar");

    const char* buttons[] = {"Back", "Forward", "Home", "Reload", "Images", "Open", "Print", "Find", "Stop"};
    for (const char* b : buttons) {
        GtkWidget* btn = gtk_button_new_with_label(b);
        gtk_box_append(GTK_BOX(box), btn);
    }
    return box;
}

GtkWidget* BrowserWindow::create_location_bar() {
    GtkWidget* box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 8);
    gtk_widget_set_margin_start(box, 8);
    gtk_widget_set_margin_end(box, 8);
    gtk_widget_set_margin_top(box, 4);
    gtk_widget_set_margin_bottom(box, 4);

    GtkWidget* label = gtk_label_new("Location:");
    m_url_entry = gtk_entry_new();
    gtk_entry_set_placeholder_text(GTK_ENTRY(m_url_entry), "http://www.netscape.com");
    gtk_widget_set_hexpand(m_url_entry, TRUE);

    gtk_box_append(GTK_BOX(box), label);
    gtk_box_append(GTK_BOX(box), m_url_entry);
    return box;
}

GtkWidget* BrowserWindow::create_content_area() {
    GtkWidget* frame = gtk_frame_new(NULL);
    gtk_widget_set_vexpand(frame, TRUE);
    gtk_widget_set_hexpand(frame, TRUE);
    gtk_widget_set_margin_start(frame, 4);
    gtk_widget_set_margin_end(frame, 4);

    // In a real Ladybird app, this would be a Ladybird::WebContentView
    m_web_view = gtk_label_new("Ladybird Rendering Engine Content Area\n\n[ Retro Netscape UI Active ]");
    gtk_widget_add_css_class(m_web_view, "main-bg");
    gtk_frame_set_child(GTK_FRAME(frame), m_web_view);

    return frame;
}

GtkWidget* BrowserWindow::create_status_bar() {
    GtkWidget* box = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 10);
    gtk_widget_add_css_class(box, "status-bar");

    m_status_label = gtk_label_new("Document: Done");
    gtk_box_append(GTK_BOX(box), m_status_label);

    return box;
}
