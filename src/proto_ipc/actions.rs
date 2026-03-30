use std::path::PathBuf;

use anyhow::Result;
pub use zellij_utils::client_server_contract::client_server_contract::{self as contract, Action};

use super::messages::path_to_string;

fn action(action_type: contract::action::ActionType) -> Action {
    Action {
        action_type: Some(action_type),
    }
}

pub fn write_action(bytes: Vec<u8>) -> Action {
    action(contract::action::ActionType::Write(contract::WriteAction {
        key_with_modifier: None,
        bytes: bytes.into_iter().map(u32::from).collect(),
        is_kitty_keyboard_protocol: false,
    }))
}

pub fn write_chars_action(chars: String) -> Action {
    action(contract::action::ActionType::WriteChars(
        contract::WriteCharsAction { chars },
    ))
}

pub fn dump_screen_action(path: Option<PathBuf>, include_scrollback: bool, ansi: bool) -> Action {
    let dump_to_stdout = path.is_none();
    action(contract::action::ActionType::DumpScreen(
        contract::DumpScreenAction {
            file_path: path.map(path_to_string).unwrap_or_default(),
            include_scrollback,
            pane_id: None,
            dump_to_stdout,
            ansi,
        },
    ))
}

pub fn dump_layout_action() -> Action {
    action(contract::action::ActionType::DumpLayout(
        contract::DumpLayoutAction {},
    ))
}

pub fn close_tab_action() -> Action {
    action(contract::action::ActionType::CloseTab(
        contract::CloseTabAction {},
    ))
}

pub fn go_to_tab_name_action(name: String, create: bool) -> Action {
    action(contract::action::ActionType::GoToTabName(
        contract::GoToTabNameAction { name, create },
    ))
}

pub fn rename_current_tab_actions(name: String) -> Vec<Action> {
    vec![
        action(contract::action::ActionType::TabNameInput(
            contract::TabNameInputAction { input: vec![0] },
        )),
        action(contract::action::ActionType::TabNameInput(
            contract::TabNameInputAction {
                input: name.bytes().map(u32::from).collect(),
            },
        )),
    ]
}

pub fn undo_rename_tab_action() -> Action {
    action(contract::action::ActionType::UndoRenameTab(
        contract::UndoRenameTabAction {},
    ))
}

pub fn query_tab_names_action() -> Action {
    action(contract::action::ActionType::QueryTabNames(
        contract::QueryTabNamesAction {},
    ))
}

pub fn rename_session_action(name: String) -> Action {
    action(contract::action::ActionType::RenameSession(
        contract::RenameSessionAction { name },
    ))
}

pub fn list_clients_action() -> Action {
    action(contract::action::ActionType::ListClients(
        contract::ListClientsAction {},
    ))
}

pub fn new_tab_action(name: Option<String>, cwd: Option<PathBuf>) -> Result<Action> {
    let cwd = cwd
        .map(|path| std::env::current_dir().unwrap_or_default().join(path))
        .or_else(|| std::env::current_dir().ok());

    Ok(action(contract::action::ActionType::NewTab(
        contract::NewTabAction {
            tiled_layout: None,
            floating_layouts: vec![],
            swap_tiled_layouts: vec![],
            swap_floating_layouts: vec![],
            tab_name: name,
            should_change_focus_to_new_tab: true,
            cwd: cwd.map(path_to_string),
            initial_panes: vec![],
            first_pane_unblock_condition: None,
        },
    )))
}

#[cfg(test)]
mod tests {
    use crate::proto_ipc::contract::action::ActionType;
    use crate::proto_ipc::{new_tab_action, rename_current_tab_actions};

    #[test]
    fn rename_current_tab_matches_upstream_tab_name_input_sequence() {
        let actions = rename_current_tab_actions("renamed".to_string());

        assert_eq!(actions.len(), 2);
        match &actions[0].action_type {
            Some(ActionType::TabNameInput(input)) => assert_eq!(input.input, vec![0]),
            _ => panic!("unexpected first action"),
        }
        match &actions[1].action_type {
            Some(ActionType::TabNameInput(input)) => {
                assert_eq!(
                    input.input,
                    "renamed".bytes().map(u32::from).collect::<Vec<_>>()
                );
            }
            _ => panic!("unexpected second action"),
        }
    }

    #[test]
    fn simple_new_tab_action_maps_to_proto() {
        let actions = vec![new_tab_action(Some("demo".to_string()), None).unwrap()];

        assert!(matches!(
            actions[0].action_type,
            Some(ActionType::NewTab(_))
        ));
    }
}
