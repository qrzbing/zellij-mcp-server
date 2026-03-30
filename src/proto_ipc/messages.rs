use std::path::PathBuf;

use anyhow::{Context, Result};

use zellij_utils::client_server_contract::client_server_contract::{
    Action, ActionMsg, AttachClientMsg, CliAssets, ClientToServerMsg, ConnStatusMsg,
    FirstClientConnectedMsg, ServerToClientMsg, Size, client_to_server_msg, self as contract,
    server_to_client_msg,
};

const UI_ATTACH_COLS: u32 = 4096;
const UI_ATTACH_ROWS: u32 = 4096;

pub fn conn_status_request() -> ClientToServerMsg {
    ClientToServerMsg {
        message: Some(client_to_server_msg::Message::ConnStatus(ConnStatusMsg {})),
    }
}

pub fn client_exited_request() -> ClientToServerMsg {
    ClientToServerMsg {
        message: Some(client_to_server_msg::Message::ClientExited(
            contract::ClientExitedMsg {},
        )),
    }
}

pub fn first_client_connected_request(cwd: Option<PathBuf>) -> ClientToServerMsg {
    ClientToServerMsg {
        message: Some(client_to_server_msg::Message::FirstClientConnected(
            FirstClientConnectedMsg {
                cli_assets: Some(CliAssets {
                    terminal_window_size: Some(Size { cols: 50, rows: 50 }),
                    cwd: cwd.map(path_to_string),
                    ..Default::default()
                }),
                is_web_client: false,
            },
        )),
    }
}

pub fn action_request(
    action: Action,
    terminal_id: Option<u32>,
    client_id: Option<u32>,
    is_cli_client: bool,
) -> ClientToServerMsg {
    ClientToServerMsg {
        message: Some(client_to_server_msg::Message::Action(ActionMsg {
            action: Some(action),
            terminal_id,
            client_id,
            is_cli_client,
        })),
    }
}

pub fn attach_client_request(
    cwd: Option<PathBuf>,
    tab_position_to_focus: Option<usize>,
) -> Result<ClientToServerMsg> {
    let tab_position_to_focus = tab_position_to_focus
        .map(|pos| {
            u32::try_from(pos)
                .with_context(|| format!("Tab position {} does not fit into u32", pos))
        })
        .transpose()?;

    Ok(ClientToServerMsg {
        message: Some(client_to_server_msg::Message::AttachClient(
            AttachClientMsg {
                cli_assets: Some(CliAssets {
                    terminal_window_size: Some(Size {
                        cols: UI_ATTACH_COLS,
                        rows: UI_ATTACH_ROWS,
                    }),
                    cwd: cwd.map(path_to_string),
                    ..Default::default()
                }),
                tab_position_to_focus,
                pane_to_focus: None,
                is_web_client: false,
            },
        )),
    })
}

pub fn is_connected_message(msg: &ServerToClientMsg) -> bool {
    matches!(
        msg.message,
        Some(server_to_client_msg::Message::Connected(_))
    )
}

pub(crate) fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use crate::proto_ipc::{
        action_request, client_exited_request, dump_layout_action, is_connected_message,
    };
    use zellij_utils::client_server_contract::client_server_contract::{
        ConnectedMsg, LogMsg, ServerToClientMsg, client_to_server_msg,
        server_to_client_msg
    };
    use zellij_utils::client_server_contract::client_server_contract::server_to_client_msg::Message as ProtoServerMessage;

    #[test]
    fn connected_helper_matches_only_connected() {
        let connected = ServerToClientMsg {
            message: Some(server_to_client_msg::Message::Connected(ConnectedMsg {})),
        };
        let log = ServerToClientMsg {
            message: Some(ProtoServerMessage::Log(LogMsg {
                lines: vec!["x".to_string()],
            })),
        };

        assert!(is_connected_message(&connected));
        assert!(!is_connected_message(&log));
    }

    #[test]
    fn action_request_wraps_action_as_expected() {
        let mut wrapped = action_request(dump_layout_action(), Some(7), Some(3), false);
        let Some(client_to_server_msg::Message::Action(msg)) = wrapped.message.take() else {
            panic!("expected action message");
        };
        assert_eq!(msg.terminal_id, Some(7));
        assert_eq!(msg.client_id, Some(3));
        assert!(!msg.is_cli_client);
        assert!(msg.action.is_some());
    }

    #[test]
    fn client_exited_request_builds_expected_message() {
        let msg = client_exited_request();
        assert!(matches!(
            msg.message,
            Some(client_to_server_msg::Message::ClientExited(_))
        ));
    }
}
