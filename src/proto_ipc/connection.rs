use std::{
    io::{BufReader, BufWriter},
    path::Path,
};

use anyhow::{Context, Result};
use interprocess::{TryClone, local_socket::Stream as LocalSocketStream};
use zellij_utils::consts::ipc_connect;

use super::{
    client_exited_request, conn_status_request, is_connected_message, read_protobuf_message,
    write_protobuf_message,
};
use crate::proto_ipc::{ClientToServerMsg, ServerToClientMsg};

pub struct ProtoIpcConnection {
    reader: BufReader<LocalSocketStream>,
    writer: BufWriter<LocalSocketStream>,
}

impl ProtoIpcConnection {
    pub fn connect(path: &Path) -> Result<Self> {
        let stream = ipc_connect(path).with_context(|| {
            format!("Failed to connect to Zellij IPC socket {}", path.display())
        })?;
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
    let _ = conn.send_client_msg(&client_exited_request());
    Ok(is_connected_message(&msg))
}
