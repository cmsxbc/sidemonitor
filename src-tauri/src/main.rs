#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod website;

use std::error::Error;
use std::sync::Mutex;

use tauri::api::dialog;
use tauri::utils::config::WindowUrl;
use tauri::window::{Monitor, Window, WindowBuilder};
use tauri::Manager;
use tauri::{
    CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    SystemTraySubmenu,
};


struct WebsiteState {
    current_id: Mutex<usize>,
    website_info: website::WebSiteInfo
}


struct AppState {
    website: Mutex<Option<WebsiteState>>,
    visible: Mutex<bool>
}


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

fn trigger_visible(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let mut visible = state.visible.lock().unwrap();
    if *visible {
        *visible = false;
        let website = state.website.lock().unwrap();
        let current_id = website.as_ref().unwrap().current_id.lock().unwrap();
        app.get_window(&format!("window-{}", *current_id)).unwrap().hide().unwrap();
        app.tray_handle().get_item("visible").set_title("Show").unwrap();
    } else {
        *visible = true;
        let website = state.website.lock().unwrap();
        let current_id = website.as_ref().unwrap().current_id.lock().unwrap();
        app.get_window(&format!("window-{}", *current_id)).unwrap().show().unwrap();
        app.tray_handle().get_item("visible").set_title("Hide").unwrap();
    }
}


fn system_tray_event_handler(app: &tauri::AppHandle, event: tauri::SystemTrayEvent) -> () {
    match event {
        SystemTrayEvent::DoubleClick { .. } => {
            trigger_visible(app);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                app.exit(0);
            }
            "visible" => {
                trigger_visible(app);
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
                let (_, id_str) = label.split_once("-").unwrap();
                let chosen_id: usize = id_str.parse().unwrap();
                let state = app.state::<AppState>();
                let website = state.website.lock().unwrap();
                let mut current_id = website.as_ref().unwrap().current_id.lock().unwrap();
                app.get_window(&format!("window-{}", *current_id)).unwrap().hide().unwrap();
                *current_id = chosen_id;
                app.get_window(label).unwrap().show().unwrap();
                app.tray_handle()
                    .get_item("visible")
                    .set_title("Hide")
                    .unwrap();
                *state.visible.lock().unwrap() = true;
            }
        },
        _ => {}
    }
}

fn run_handler(app: &tauri::AppHandle, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::WindowEvent { event, .. } => match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                trigger_visible(app);
            }
            _ => {}
        },
        _ => {}
    }
}

fn get_website_info(
    websites_path: std::path::PathBuf,
) -> Result<website::WebSiteInfo, Box<dyn Error>> {
    if !websites_path.is_file() {
        let err_msg = format!("{} does not exist!", websites_path.display());
        dialog::blocking::MessageDialogBuilder::new("Error!", err_msg.clone()).show();
        return Err(err_msg.into());
    }
    match website::WebSiteInfo::from_json(websites_path) {
        Ok(website_info) => Ok(website_info),
        Err(err_msg) => {
            dialog::blocking::MessageDialogBuilder::new("Error!", format!("{}", err_msg)).show();
            Err(err_msg)
        }
    }
}

fn setup_handler(app: &mut tauri::App) -> Result<(), Box<dyn Error>> {
    if let Some(config_dir) = app.path_resolver().app_config_dir() {
        let website_info = get_website_info(config_dir.join("websites.json")).unwrap();
        let state = app.state::<AppState>();
        let mut current_id = 0;
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
            } else {
                current_id = i;
            }
            reset(&window);
            sub_menu = sub_menu.add_item(CustomMenuItem::new(label, website.name.clone()));
        }
        *state.website.lock().unwrap() = Some(WebsiteState { current_id: Mutex::new(current_id), website_info: website_info.clone() });

        let tray_menu = SystemTrayMenu::new()
            .add_submenu(SystemTraySubmenu::new("Websites", sub_menu))
            .add_native_item(SystemTrayMenuItem::Separator)
            .add_item(CustomMenuItem::new("reset".to_string(), "Reset"))
            .add_item(CustomMenuItem::new("restart".to_string(), "Restart"))
            .add_item(CustomMenuItem::new("visible".to_string(), "Hide"))
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
                let state = handle.state::<AppState>();
                if !*state.visible.lock().unwrap() {
                    continue;
                }
                let websites = state.website.lock().unwrap();
                let mut current_id = websites.as_ref().unwrap().current_id.lock().unwrap();
                let current_total = websites.as_ref().unwrap().website_info.websites.len();
                let id = *current_id;
                *current_id += 1;
                *current_id %= current_total;
                handle.get_window(&format!("window-{}", id)).unwrap().hide().unwrap();
                handle.get_window(&format!("window-{}", (id + 1) % current_total)).unwrap().show().unwrap();
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
        .manage(AppState { website: Default::default(), visible: Mutex::new(true) })
        .on_system_tray_event(system_tray_event_handler)
        .setup(setup_handler)
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(run_handler);
}
