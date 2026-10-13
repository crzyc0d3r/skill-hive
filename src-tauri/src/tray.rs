use std::sync::Arc;

use hive_core::db::skill_store::SkillStore;
use hive_core::util::tray_menu::{
    build_tray_menu_from_data, close_effect, effective_close, left_click_restores, CloseChoice,
};
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

fn choice_from(value: &str) -> CloseChoice {
    match value {
        "minimize" => CloseChoice::Minimize,
        "quit" => CloseChoice::Quit,
        _ => CloseChoice::Ask,
    }
}

fn label<'a>(model: &'a hive_core::util::tray_menu::TrayEntry, id: &'a str) -> &'a str {
    model
        .children
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.label.as_str())
        .unwrap_or(id)
}

pub fn install(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let Some(icon) = app.default_window_icon().cloned() else {
        return Ok(());
    };
    let store = app.state::<Arc<SkillStore>>();
    let tray_off = store
        .get_setting("trayIcon")
        .ok()
        .flatten()
        .is_some_and(|value| value == "off");
    if tray_off {
        return Ok(());
    }
    let model = build_tray_menu_from_data(0, 0, &[("research", "Research")]);
    let status = MenuItem::with_id(app, "status", label(&model, "status"), false, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", label(&model, "updates"), true, None::<&str>)?;
    let apply = MenuItem::with_id(app, "apply:research", "Apply Research", true, None::<&str>)?;
    let remove = MenuItem::with_id(
        app,
        "remove:research",
        "Remove Research",
        true,
        None::<&str>,
    )?;
    let presets = Submenu::with_items(app, label(&model, "presets"), true, &[&apply, &remove])?;
    let open = MenuItem::with_id(app, "open", label(&model, "open"), true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", label(&model, "check"), true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", label(&model, "folder"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", label(&model, "quit"), true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&status, &updates, &presets, &open, &check, &folder, &quit],
    )?;
    let _tray = TrayIconBuilder::with_id("hive")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "open" | "check" | "folder" | "updates" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if !left_click_restores(std::env::consts::OS) {
                    return;
                }
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn on_window(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let Some(store) = window.try_state::<Arc<SkillStore>>() else {
        return;
    };
    let choice = store
        .get_setting("closeBehaviour")
        .ok()
        .flatten()
        .unwrap_or_else(|| "ask".into());
    let tray_on = store
        .get_setting("trayIcon")
        .ok()
        .flatten()
        .map(|value| value != "off")
        .unwrap_or(true);
    match close_effect(effective_close(choice_from(&choice), tray_on)) {
        "quit" => {}
        "hide_to_tray" => {
            api.prevent_close();
            let _ = window.hide();
        }
        _ => {
            api.prevent_close();
            let _ = window.emit("window-close-requested", "ask");
        }
    }
}
