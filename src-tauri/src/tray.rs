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

fn preset_refs(store: &SkillStore) -> Vec<(String, String)> {
    crate::commands::tray_preset_pairs(store)
}

fn write_manifest(pairs: &[(String, String)]) {
    let path = hive_core::library::central_repo::base_dir().join("tray-menu.json");
    let labels: Vec<String> = pairs
        .iter()
        .map(|(id, name)| format!("apply:{id}:{name}"))
        .collect();
    if let Ok(body) = serde_json::to_string(&labels) {
        let _ = std::fs::write(path, body);
    }
}

fn build_menu(
    app: &tauri::AppHandle,
    store: &SkillStore,
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let pairs = preset_refs(store);
    write_manifest(&pairs);
    let borrowed: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();
    let skill_count = store.get_all_skills().map(|rows| rows.len()).unwrap_or(0);
    let model = build_tray_menu_from_data(skill_count, 0, &borrowed);
    let status = MenuItem::with_id(app, "status", label(&model, "status"), false, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", label(&model, "updates"), true, None::<&str>)?;
    let mut preset_items: Vec<MenuItem<tauri::Wry>> = Vec::new();
    for (id, name) in &pairs {
        preset_items.push(MenuItem::with_id(
            app,
            format!("apply:{id}"),
            format!("Apply {name}"),
            true,
            None::<&str>,
        )?);
        preset_items.push(MenuItem::with_id(
            app,
            format!("remove:{id}"),
            format!("Remove {name}"),
            true,
            None::<&str>,
        )?);
    }
    let preset_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = preset_items
        .iter()
        .map(|item| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    let presets = Submenu::with_items(app, label(&model, "presets"), true, &preset_refs)?;
    let open = MenuItem::with_id(app, "open", label(&model, "open"), true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", label(&model, "check"), true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", label(&model, "folder"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", label(&model, "quit"), true, None::<&str>)?;
    Ok(Menu::with_items(
        app,
        &[&status, &updates, &presets, &open, &check, &folder, &quit],
    )?)
}

fn on_menu(app: &tauri::AppHandle, id: &str) {
    if let Some(preset) = id.strip_prefix("apply:") {
        if let Some(store) = app.try_state::<Arc<SkillStore>>() {
            let _ = crate::commands::apply_preset_by_id(store.inner(), preset);
        }
        return;
    }
    if let Some(preset) = id.strip_prefix("remove:") {
        if let Some(store) = app.try_state::<Arc<SkillStore>>() {
            let _ = crate::commands::remove_preset_by_id(store.inner(), preset);
        }
        return;
    }
    if matches!(id, "open" | "check" | "folder" | "updates") {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
    if id == "quit" {
        app.exit(0);
    }
}

pub fn refresh(app: &tauri::AppHandle) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let Some(store) = app.try_state::<Arc<SkillStore>>() else {
        return Ok(Vec::new());
    };
    let pairs = preset_refs(store.inner());
    write_manifest(&pairs);
    if let Some(icon) = app.tray_by_id("hive") {
        let menu = build_menu(app, store.inner())?;
        icon.set_menu(Some(menu))?;
    }
    Ok(pairs.into_iter().map(|(_, name)| name).collect())
}

pub fn install(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let Some(store) = app.try_state::<Arc<SkillStore>>() else {
        return Ok(());
    };
    let pairs = preset_refs(store.inner());
    write_manifest(&pairs);
    let Some(icon) = app.default_window_icon().cloned() else {
        return Ok(());
    };
    let tray_off = store
        .get_setting("trayIcon")
        .ok()
        .flatten()
        .is_some_and(|value| value == "off");
    if tray_off {
        return Ok(());
    }
    let menu = build_menu(app, store.inner())?;
    let _tray = TrayIconBuilder::with_id("hive")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
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
