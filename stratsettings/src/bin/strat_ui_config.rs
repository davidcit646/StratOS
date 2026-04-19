//! CLI for merged StratOS UI settings (`/config/strat/settings.toml` + `settings.d`).

use std::env;
use std::path::Path;
use std::process;

use stratsettings::{write_stratvm_keybind_file, CONFIG_DIR};

fn usage() {
    eprintln!("Usage: strat-ui-config show");
    eprintln!("       strat-ui-config get <section> <key>");
    eprintln!("       strat-ui-config export-keybinds   # write stratvm-keybinds from [keyboard]");
    eprintln!("       strat-ui-config paths");
    eprintln!("Sections: stratterm | panel | chrome | keyboard | spotlite | network | <extension table name>");
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage();
        process::exit(1);
    }
    let cmd = args[0].clone();
    match cmd.as_str() {
        "paths" => {
            println!("config_dir={}", stratsettings::CONFIG_DIR);
            println!(
                "main={}/{}",
                stratsettings::CONFIG_DIR,
                stratsettings::SETTINGS_FILE
            );
            println!(
                "fragments={}/{}",
                stratsettings::CONFIG_DIR,
                stratsettings::SETTINGS_D
            );
            println!(
                "legacy_panel_conf={}/{}",
                stratsettings::CONFIG_DIR,
                stratsettings::LEGACY_PANEL_CONF
            );
        }
        "show" => match stratsettings::StratSettings::load() {
            Ok(s) => {
                println!(
                    "stratterm.file_explorer.status_bar_enabled = {}",
                    s.stratterm.file_explorer.status_bar_enabled
                );
                println!(
                    "stratterm.file_explorer.client_title_bar_enabled = {}",
                    s.stratterm.file_explorer.client_title_bar_enabled
                );
                println!(
                    "stratterm.term.terminal_font_scale = {}",
                    s.stratterm.term.terminal_font_scale
                );
                println!(
                    "stratterm.file_explorer.title_bar_font_scale = {}",
                    s.stratterm.file_explorer.title_bar_font_scale
                );
                println!("stratterm.term.scrollback_max_lines = {}", s.stratterm.term.scrollback_max_lines);
                println!(
                    "stratterm.file_explorer.default_view = {}",
                    s.stratterm.file_explorer.default_view
                );
                println!("panel.position = {}", s.panel.position);
                println!("panel.size = {}", s.panel.size);
                println!("panel.font_scale = {}", s.panel.font_scale);
                println!(
                    "panel.workspace.enabled = {}",
                    s.panel.workspace.enabled
                );
                println!(
                    "panel.workspace.poll_interval_secs = {}",
                    s.panel.workspace.poll_interval_secs
                );
                println!("chrome.decoration_titlebar_height = {}", s.chrome.decoration_titlebar_height);
                println!("chrome.border_pad = {}", s.chrome.border_pad);
                println!(
                    "chrome.decorations_enabled_default = {}",
                    s.chrome.decorations_enabled_default
                );
                println!("keyboard.spotlite = {}", s.keyboard.spotlite);
                println!("keyboard.cycle_layout = {}", s.keyboard.cycle_layout);
                println!("spotlite.headless.enabled = {}", s.spotlite.headless.enabled);
                println!("spotlite.headless.boot_start = {}", s.spotlite.headless.boot_start);
                println!("spotlite.headless.frequency_ms = {}", s.spotlite.headless.frequency_ms);
                println!("spotlite.headless.rescan_secs = {}", s.spotlite.headless.rescan_secs);
                println!("spotlite.headless.batch_limit = {}", s.spotlite.headless.batch_limit);
                println!(
                    "spotlite.headless.high_usage_load_per_cpu = {}",
                    s.spotlite.headless.high_usage_load_per_cpu
                );
                println!("spotlite.ui.enabled = {}", s.spotlite.ui.enabled);
                println!("spotlite.ui.tick_ms = {}", s.spotlite.ui.tick_ms);
                println!("spotlite.ui.batch_limit = {}", s.spotlite.ui.batch_limit);
                println!("spotlite.ui.idle_after_secs = {}", s.spotlite.ui.idle_after_secs);
                println!("spotlite.ui.startup_grace_secs = {}", s.spotlite.ui.startup_grace_secs);
                println!("spotlite.ui.post_nav_delay_ms = {}", s.spotlite.ui.post_nav_delay_ms);
                println!("spotlite.ui.post_nav_scan_limit = {}", s.spotlite.ui.post_nav_scan_limit);
                println!("spotlite.ui.post_nav_force_secs = {}", s.spotlite.ui.post_nav_force_secs);
                println!("network.interface = {}", s.network.interface);
                println!("network.use_dhcp = {}", s.network.use_dhcp);
                println!(
                    "network.static_ip = {}",
                    s.network.static_ip.as_deref().unwrap_or("")
                );
                println!(
                    "network.static_netmask = {}",
                    s.network.static_netmask.as_deref().unwrap_or("")
                );
                println!(
                    "network.static_gateway = {}",
                    s.network.static_gateway.as_deref().unwrap_or("")
                );
                println!("network.retry_interval_secs = {}", s.network.retry_interval_secs);
                println!(
                    "network.max_retries = {}",
                    s.network
                        .max_retries
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "unlimited".to_string())
                );
                println!("extension_tables = {}", s.extensions.len());
                for k in s.extensions.keys() {
                    println!("  [{k}]");
                }
            }
            Err(e) => {
                eprintln!("strat-ui-config: {e}");
                process::exit(1);
            }
        },
        "get" => {
            args.remove(0);
            if args.len() < 2 {
                usage();
                process::exit(1);
            }
            let section = args[0].as_str();
            let key = args[1].as_str();
            let s = match stratsettings::StratSettings::load() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("strat-ui-config: {e}");
                    process::exit(1);
                }
            };
            let v = match section {
                "stratterm" => match key {
                    "status_bar_enabled" | "file_explorer.status_bar_enabled" => {
                        format!("{}", s.stratterm.file_explorer.status_bar_enabled)
                    }
                    "client_title_bar_enabled" | "file_explorer.client_title_bar_enabled" => {
                        format!("{}", s.stratterm.file_explorer.client_title_bar_enabled)
                    }
                    "terminal_font_scale" | "term.terminal_font_scale" => {
                        format!("{}", s.stratterm.term.terminal_font_scale)
                    }
                    "title_bar_font_scale" | "file_explorer.title_bar_font_scale" => {
                        format!("{}", s.stratterm.file_explorer.title_bar_font_scale)
                    }
                    "scrollback_max_lines" | "term.scrollback_max_lines" => {
                        format!("{}", s.stratterm.term.scrollback_max_lines)
                    }
                    "default_view" | "file_explorer.default_view" => {
                        s.stratterm.file_explorer.default_view.clone()
                    }
                    _ => {
                        eprintln!("unknown key {key} for stratterm");
                        process::exit(1);
                    }
                },
                "panel" => match key {
                    "position" => s.panel.position.clone(),
                    "size" => format!("{}", s.panel.size),
                    "font_scale" => format!("{}", s.panel.font_scale),
                    "workspace.enabled" => format!("{}", s.panel.workspace.enabled),
                    "workspace.poll_interval_secs" => format!("{}", s.panel.workspace.poll_interval_secs),
                    "workspace.show_labels" => format!("{}", s.panel.workspace.show_labels),
                    "workspace.max_visible" => format!("{}", s.panel.workspace.max_visible),
                    _ => {
                        eprintln!("unknown key {key} for panel");
                        process::exit(1);
                    }
                },
                "chrome" => match key {
                    "decoration_titlebar_height" => format!("{}", s.chrome.decoration_titlebar_height),
                    "border_pad" => format!("{}", s.chrome.border_pad),
                    "decorations_enabled_default" => format!("{}", s.chrome.decorations_enabled_default),
                    _ => {
                        eprintln!("unknown key {key} for chrome");
                        process::exit(1);
                    }
                },
                "keyboard" => match key {
                    "spotlite" => s.keyboard.spotlite.clone(),
                    "cycle_layout" => s.keyboard.cycle_layout.clone(),
                    _ => {
                        eprintln!("unknown key {key} for keyboard");
                        process::exit(1);
                    }
                },
                "network" => match key {
                    "interface" => s.network.interface.clone(),
                    "use_dhcp" => format!("{}", s.network.use_dhcp),
                    "static_ip" => s
                        .network
                        .static_ip
                        .clone()
                        .unwrap_or_default(),
                    "static_netmask" => s
                        .network
                        .static_netmask
                        .clone()
                        .unwrap_or_default(),
                    "static_gateway" => s
                        .network
                        .static_gateway
                        .clone()
                        .unwrap_or_default(),
                    "retry_interval_secs" => format!("{}", s.network.retry_interval_secs),
                    "max_retries" => s
                        .network
                        .max_retries
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "unlimited".to_string()),
                    _ => {
                        eprintln!("unknown key {key} for network");
                        process::exit(1);
                    }
                },
                "spotlite" => match key {
                    "headless.enabled" => format!("{}", s.spotlite.headless.enabled),
                    "headless.boot_start" => format!("{}", s.spotlite.headless.boot_start),
                    "headless.frequency_ms" => format!("{}", s.spotlite.headless.frequency_ms),
                    "headless.rescan_secs" => format!("{}", s.spotlite.headless.rescan_secs),
                    "headless.batch_limit" => format!("{}", s.spotlite.headless.batch_limit),
                    "headless.high_usage_load_per_cpu" => {
                        format!("{}", s.spotlite.headless.high_usage_load_per_cpu)
                    }
                    "ui.enabled" => format!("{}", s.spotlite.ui.enabled),
                    "ui.tick_ms" => format!("{}", s.spotlite.ui.tick_ms),
                    "ui.batch_limit" => format!("{}", s.spotlite.ui.batch_limit),
                    "ui.idle_after_secs" => format!("{}", s.spotlite.ui.idle_after_secs),
                    "ui.startup_grace_secs" => format!("{}", s.spotlite.ui.startup_grace_secs),
                    "ui.post_nav_delay_ms" => format!("{}", s.spotlite.ui.post_nav_delay_ms),
                    "ui.post_nav_scan_limit" => format!("{}", s.spotlite.ui.post_nav_scan_limit),
                    "ui.post_nav_force_secs" => format!("{}", s.spotlite.ui.post_nav_force_secs),
                    _ => {
                        eprintln!("unknown key {key} for spotlite");
                        process::exit(1);
                    }
                },
                ext => {
                    if let Some(table) = s.extensions.get(ext) {
                        if let Some(inner) = table.get(key) {
                            format!("{inner}")
                        } else {
                            eprintln!("no key {key} in [{ext}]");
                            process::exit(1);
                        }
                    } else {
                        eprintln!("unknown section {ext}");
                        process::exit(1);
                    }
                }
            };
            println!("{v}");
        }
        "export-keybinds" => match stratsettings::StratSettings::load() {
            Ok(s) => {
                if let Err(e) = write_stratvm_keybind_file(Path::new(CONFIG_DIR), &s.keyboard) {
                    eprintln!("strat-ui-config: {e}");
                    process::exit(1);
                }
                println!("{} written", Path::new(CONFIG_DIR).join("stratvm-keybinds").display());
            }
            Err(e) => {
                eprintln!("strat-ui-config: {e}");
                process::exit(1);
            }
        },
        _ => {
            usage();
            process::exit(1);
        }
    }
}
