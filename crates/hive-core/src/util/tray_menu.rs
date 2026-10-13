// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

//! Tray menu contents and the close-behaviour rules. The OS icon itself needs
//! a display; this module is what the menu is built from.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayEntry {
    pub id: String,
    pub label: String,
    pub children: Vec<TrayEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseChoice {
    Ask,
    Minimize,
    Quit,
}

pub fn build_tray_menu_from_data(
    skill_count: usize,
    update_count: usize,
    presets: &[(&str, &str)],
) -> TrayEntry {
    let mut children = vec![
        TrayEntry {
            id: "status".into(),
            label: format!("{skill_count} skills"),
            children: Vec::new(),
        },
        TrayEntry {
            id: "updates".into(),
            label: if update_count == 0 {
                "No skill updates".into()
            } else {
                format!("{update_count} skill updates")
            },
            children: Vec::new(),
        },
    ];
    let preset_children = presets
        .iter()
        .map(|(id, name)| TrayEntry {
            id: (*id).to_string(),
            label: (*name).to_string(),
            children: vec![
                TrayEntry {
                    id: format!("apply:{id}"),
                    label: format!("Apply {name}"),
                    children: Vec::new(),
                },
                TrayEntry {
                    id: format!("remove:{id}"),
                    label: format!("Remove {name}"),
                    children: Vec::new(),
                },
            ],
        })
        .collect();
    children.push(TrayEntry {
        id: "presets".into(),
        label: "Presets".into(),
        children: preset_children,
    });
    for (id, label) in [
        ("open", "Open"),
        ("check", "Check for skill updates"),
        ("folder", "Open Skills Folder"),
        ("quit", "Quit"),
    ] {
        children.push(TrayEntry {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
        });
    }
    TrayEntry {
        id: "root".into(),
        label: "Skill Hive".into(),
        children,
    }
}

pub fn left_click_restores(os: &str) -> bool {
    matches!(os, "windows" | "linux")
}

/// When the tray icon is hidden, Minimize is not available and falls back to Ask.
pub fn effective_close(choice: CloseChoice, tray_enabled: bool) -> CloseChoice {
    if !tray_enabled && choice == CloseChoice::Minimize {
        CloseChoice::Ask
    } else {
        choice
    }
}

pub fn close_effect(choice: CloseChoice) -> &'static str {
    match choice {
        CloseChoice::Ask => "ask",
        CloseChoice::Minimize => "hide_to_tray",
        CloseChoice::Quit => "quit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_menu_lists_status_presets_and_quit() {
        let menu = build_tray_menu_from_data(3, 1, &[("research", "Research")]);
        let labels: Vec<_> = menu
            .children
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(
            labels,
            vec![
                "3 skills",
                "1 skill updates",
                "Presets",
                "Open",
                "Check for skill updates",
                "Open Skills Folder",
                "Quit"
            ]
        );
        let presets = menu
            .children
            .iter()
            .find(|item| item.id == "presets")
            .unwrap();
        assert_eq!(presets.children[0].children[0].id, "apply:research");
        assert_eq!(presets.children[0].children[1].label, "Remove Research");
        assert!(left_click_restores("windows"));
        assert!(left_click_restores("linux"));
        assert!(!left_click_restores("macos"));
    }

    #[test]
    fn hidden_tray_refuses_minimize() {
        assert_eq!(
            effective_close(CloseChoice::Minimize, false),
            CloseChoice::Ask
        );
        assert_eq!(
            close_effect(effective_close(CloseChoice::Minimize, true)),
            "hide_to_tray"
        );
        assert_eq!(close_effect(CloseChoice::Quit), "quit");
    }
}
