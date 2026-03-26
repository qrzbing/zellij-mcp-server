use std::{
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use interprocess::{TryClone, local_socket::Stream as LocalSocketStream};
use prost::Message;
use zellij_utils::{
    cli::CliAction,
    client_server_contract::client_server_contract::{
        self as contract, client_to_server_msg, server_to_client_msg, ClientToServerMsg,
        ConnStatusMsg, ServerToClientMsg,
    },
    consts::ipc_connect,
};

#[allow(unused_imports)]
pub use contract::{
    Action, ActionMsg, AttachClientMsg, AttachWatcherClientMsg, CliAssets, ConnectedMsg, ExitMsg,
    LogErrorMsg, LogMsg, PaneReference, Size,
};
#[allow(unused_imports)]
pub use contract::{ClientToServerMsg as ProtoClientToServerMsg, ServerToClientMsg as ProtoServerToClientMsg};
pub use contract::ExitReason as ProtoExitReason;
#[allow(unused_imports)]
pub use contract::{client_to_server_msg::Message as ProtoClientMessage, server_to_client_msg::Message as ProtoServerMessage};

const UI_ATTACH_COLS: u32 = 4096;
const UI_ATTACH_ROWS: u32 = 4096;

fn read_protobuf_message<T: Message + Default>(reader: &mut impl Read) -> Result<T> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    T::decode(&buf[..]).map_err(Into::into)
}

fn write_protobuf_message<T: Message>(writer: &mut impl Write, msg: &T) -> Result<()> {
    let len = msg.encoded_len() as u32;
    writer.write_all(&len.to_le_bytes())?;

    let mut buf = Vec::with_capacity(len as usize);
    msg.encode(&mut buf)?;
    writer.write_all(&buf)?;
    writer.flush()?;

    Ok(())
}

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
    matches!(msg.message, Some(server_to_client_msg::Message::Connected(_)))
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

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

pub fn close_tab_by_id_action(id: u64) -> Action {
    action(contract::action::ActionType::CloseTabById(
        contract::CloseTabByIdAction { id },
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

pub fn rename_tab_by_id_action(id: u64, name: String) -> Action {
    action(contract::action::ActionType::RenameTabById(
        contract::RenameTabByIdAction { id, name },
    ))
}

pub fn undo_rename_tab_action() -> Action {
    action(contract::action::ActionType::UndoRenameTab(
        contract::UndoRenameTabAction {},
    ))
}

pub fn undo_rename_tab_by_id_action(id: u64) -> Action {
    action(contract::action::ActionType::UndoRenameTabByTabId(
        contract::UndoRenameTabByTabIdAction { id },
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

pub fn actions_from_cli(cli_action: CliAction) -> Result<Vec<Action>> {
    match cli_action {
        CliAction::Write { bytes, pane_id: None } => Ok(vec![write_action(bytes)]),
        CliAction::Write {
            pane_id: Some(_), ..
        } => bail!("Pane-targeted Write is not migrated to protobuf yet"),
        CliAction::WriteChars {
            chars,
            pane_id: None,
        } => Ok(vec![write_chars_action(chars)]),
        CliAction::WriteChars {
            pane_id: Some(_), ..
        } => bail!("Pane-targeted WriteChars is not migrated to protobuf yet"),
        CliAction::DumpScreen {
            path,
            full,
            pane_id: None,
            ansi,
        } => Ok(vec![dump_screen_action(path, full, ansi)]),
        CliAction::DumpScreen {
            pane_id: Some(_), ..
        } => bail!("Pane-targeted DumpScreen is not migrated to protobuf yet"),
        CliAction::DumpLayout => Ok(vec![dump_layout_action()]),
        CliAction::CloseTab { tab_id: None } => Ok(vec![close_tab_action()]),
        CliAction::CloseTab { tab_id: Some(id) } => {
            Ok(vec![close_tab_by_id_action(u64::try_from(id)?)])
        }
        CliAction::GoToTabName { name, create } => Ok(vec![go_to_tab_name_action(name, create)]),
        CliAction::RenameTab { name, tab_id: None } => Ok(rename_current_tab_actions(name)),
        CliAction::RenameTab {
            name,
            tab_id: Some(id),
        } => Ok(vec![rename_tab_by_id_action(u64::try_from(id)?, name)]),
        CliAction::UndoRenameTab { tab_id: None } => Ok(vec![undo_rename_tab_action()]),
        CliAction::UndoRenameTab { tab_id: Some(id) } => {
            Ok(vec![undo_rename_tab_by_id_action(u64::try_from(id)?)])
        }
        CliAction::QueryTabNames => Ok(vec![query_tab_names_action()]),
        CliAction::RenameSession { name } => Ok(vec![rename_session_action(name)]),
        CliAction::ListClients => Ok(vec![list_clients_action()]),
        CliAction::NewTab {
            layout,
            layout_dir,
            name,
            cwd,
            initial_command,
            initial_plugin,
            close_on_exit,
            start_suspended,
            block_until_exit_success,
            block_until_exit_failure,
            block_until_exit,
        } => {
            if layout.is_some()
                || layout_dir.is_some()
                || !initial_command.is_empty()
                || initial_plugin.is_some()
                || close_on_exit
                || start_suspended
                || block_until_exit_success
                || block_until_exit_failure
                || block_until_exit
            {
                bail!("Only the simple NewTab behavior is migrated to protobuf so far");
            }
            Ok(vec![new_tab_action(name, cwd)?])
        }
        other => bail!("CliAction {:?} is not migrated to protobuf yet", other),
    }
}

pub struct ProtoIpcConnection {
    reader: BufReader<LocalSocketStream>,
    writer: BufWriter<LocalSocketStream>,
}

impl ProtoIpcConnection {
    pub fn connect(path: &Path) -> Result<Self> {
        let stream = ipc_connect(path)
            .with_context(|| format!("Failed to connect to Zellij IPC socket {}", path.display()))?;
        let reader_stream = stream.try_clone().context("Failed to clone IPC stream")?;

        Ok(Self {
            reader: BufReader::new(reader_stream),
            writer: BufWriter::new(stream),
        })
    }

    pub fn send_client_msg(&mut self, msg: &ClientToServerMsg) -> Result<()> {
        write_protobuf_message(&mut self.writer, msg)
    }

    pub fn recv_server_msg(&mut self) -> Result<ServerToClientMsg> {
        read_protobuf_message(&mut self.reader)
    }
}

pub fn probe_session(path: &Path) -> Result<bool> {
    let mut conn = ProtoIpcConnection::connect(path)?;
    conn.send_client_msg(&conn_status_request())?;
    let msg = conn.recv_server_msg()?;
    Ok(is_connected_message(&msg))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        ProtoClientToServerMsg, ProtoServerMessage, ProtoServerToClientMsg, action_request,
        actions_from_cli, client_exited_request, conn_status_request, is_connected_message,
        read_protobuf_message, rename_current_tab_actions, write_protobuf_message,
    };
    use zellij_utils::client_server_contract::client_server_contract::action::ActionType;
    use zellij_utils::client_server_contract::client_server_contract::{
        ConnectedMsg, ServerToClientMsg, client_to_server_msg, server_to_client_msg,
    };
    use zellij_utils::cli::CliAction;

    #[test]
    fn length_prefixed_proto_roundtrip_works() {
        let msg = conn_status_request();
        let mut buf = Vec::new();

        write_protobuf_message(&mut buf, &msg).unwrap();

        let decoded: ProtoClientToServerMsg =
            read_protobuf_message(&mut Cursor::new(buf)).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn connected_helper_matches_only_connected() {
        let connected = ServerToClientMsg {
            message: Some(server_to_client_msg::Message::Connected(ConnectedMsg {})),
        };
        let log = ProtoServerToClientMsg {
            message: Some(ProtoServerMessage::Log(super::LogMsg {
                lines: vec!["x".to_string()],
            })),
        };

        assert!(is_connected_message(&connected));
        assert!(!is_connected_message(&log));
    }

    #[test]
    fn rename_current_tab_matches_upstream_tab_name_input_sequence() {
        let actions = rename_current_tab_actions("renamed".to_string());

        assert_eq!(actions.len(), 2);
        match &actions[0].action_type {
            Some(ActionType::TabNameInput(input)) => {
                assert_eq!(input.input, vec![0]);
            }
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
    fn action_request_wraps_action_as_expected() {
        let mut wrapped = action_request(super::dump_layout_action(), Some(7), Some(3), false);
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

    #[test]
    fn simple_new_tab_cli_action_maps_to_proto() {
        let actions = actions_from_cli(CliAction::NewTab {
            layout: None,
            layout_dir: None,
            name: Some("demo".to_string()),
            cwd: None,
            initial_command: vec![],
            initial_plugin: None,
            close_on_exit: false,
            start_suspended: false,
            block_until_exit_success: false,
            block_until_exit_failure: false,
            block_until_exit: false,
        })
        .unwrap();

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0].action_type,
            Some(ActionType::NewTab(_))
        ));
    }
}
