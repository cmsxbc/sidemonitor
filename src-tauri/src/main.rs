#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod website;

use std::error::Error;

use tauri::api::dialog;
use tauri::utils::config::WindowUrl;
use tauri::window::{Monitor, Window, WindowBuilder};
use tauri::Manager;
use tauri::{
    CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    SystemTraySubmenu,
};

fn reset(window: &Window) -> () {
    if let Some(monitor) = window.current_monitor().unwrap() {
        let monitors = window.available_monitors().unwrap();
        let mut smonitor: Monitor = monitor.clone();
        for m in monitors.into_iter() {
            let size = m.size();
            if size.width * size.height < smonitor.size().width * smonitor.size().height {
                smonitor = m.clone();
            }
        }
        window
            .set_size(smonitor.size().to_logical::<u32>(smonitor.scale_factor()))
            .unwrap();
        window.set_position(smonitor.position().clone()).unwrap();
    }
}

fn system_tray_event_handler(app: &tauri::AppHandle, event: tauri::SystemTrayEvent) -> () {
    match event {
        SystemTrayEvent::DoubleClick { .. } => {
            for window in app.windows().into_values().into_iter() {
                if window.is_visible().unwrap() {
                    window.hide().unwrap();
                    return ();
                }
            }
            if let Some(config_dir) = app.path_resolver().app_config_dir() {
                let website_info =
                    website::WebSiteInfo::from_json(config_dir.join("websites.json")).unwrap();
                for (i, website) in website_info.websites.into_iter().enumerate() {
                    if website.name == website_info.default {
                        app.get_window(&format!("window-{}", i))
                            .unwrap()
                            .show()
                            .unwrap();
                    }
                }
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                app.exit(0);
            }
            "hide" => {
                for window in app.windows().into_values().into_iter() {
                    window.hide().unwrap();
                }
                app.tray_handle().get_item(&id).set_enabled(false).unwrap();
            }
            "reset" => {
                for window in app.windows().into_values().into_iter() {
                    reset(&window);
                }
            }
            "restart" => {
                app.restart();
            }
            label => {
                for window in app.windows().into_values().into_iter() {
                    window.hide().unwrap();
                }
                app.get_window(label).unwrap().show().unwrap();
                app.tray_handle()
                    .get_item("hide")
                    .set_enabled(true)
                    .unwrap();
            }
        },
        _ => {}
    }
}

fn run_handler(app: &tauri::AppHandle, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::WindowEvent { label, event, .. } => match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                app.tray_handle()
                    .get_item("hide")
                    .set_enabled(false)
                    .unwrap();
                app.get_window(&*label).unwrap().hide().unwrap();
                app.tray_handle()
                    .get_item(&*label)
                    .set_selected(false)
                    .unwrap();
            }
            _ => {}
        },
        _ => {}
    }
}

fn setup_handler(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    if let Some(config_dir) = app.path_resolver().app_config_dir() {
        let websites_path = config_dir.join("websites.json");
        if !websites_path.is_file() {
            let err_msg = format!("{} does not exist!", websites_path.display());
            dialog::blocking::MessageDialogBuilder::new("Error!", err_msg.clone()).show();
            return Err(err_msg.into());
        }
        let website_info = website::WebSiteInfo::from_json(websites_path).unwrap();
        let mut sub_menu = SystemTrayMenu::new();
        for (i, website) in website_info.websites.clone().into_iter().enumerate() {
            let label = format!("window-{}", i);
            let window = WindowBuilder::new(
                app,
                label.clone(),
                WindowUrl::External(website.url.parse().unwrap()),
            )
            .skip_taskbar(true)
            .decorations(false)
            .title(&website.name)
            .build()?;
            if website.name != website_info.default {
                window.hide().unwrap();
            }
            reset(&window);
            sub_menu = sub_menu.add_item(CustomMenuItem::new(label, website.name.clone()));
        }

        let tray_menu = SystemTrayMenu::new()
            .add_submenu(SystemTraySubmenu::new("Websites", sub_menu))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(CustomMenuItem::new("reset".to_string(), "Reset"))
            .add_item(CustomMenuItem::new("restart".to_string(), "Restart"))
            .add_item(CustomMenuItem::new("hide".to_string(), "Hide"))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(CustomMenuItem::new("quit".to_string(), "Quit"));
        let system_tray = SystemTray::new().with_menu(tray_menu);
        system_tray.build(app)?;
        if None == website_info.slider {
            return Ok(());
        }
        if website_info.websites.len() < 1 {
            return Ok(());
        }
        let handle = app.handle();
        let duration = website_info.slider.unwrap();
        tauri::async_runtime::spawn(async move {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(duration));
                let websites_count = handle.windows().len();
                if websites_count < 1 {
                    continue;
                }
                let mut has_shown = false;
                for i in 0..websites_count {
                    let label = format!("window-{}", i);
                    let window = handle.get_window(&label).unwrap();
                    if has_shown {
                        window.show().unwrap();
                        has_shown = false;
                        break;
                    }
                    if window.is_visible().unwrap() {
                        has_shown = true;
                        window.hide().unwrap();
                    }
                }
                if !has_shown {
                    continue;
                }
                handle.get_window("window-0").unwrap().show().unwrap();
            }
        });
        Ok(())
    } else {
        dialog::blocking::MessageDialogBuilder::new("Error!", "no config_dir").show();
        Err("no config_dir".into())
    }
}

fn main() {
    tauri::Builder::default()
        .on_system_tray_event(system_tray_event_handler)
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(run_handler);
}
