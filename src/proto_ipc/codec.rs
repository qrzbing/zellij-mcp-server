use std::io::{Read, Write};

use anyhow::Result;
use prost::Message;

pub(crate) fn read_protobuf_message<T: Message + Default>(reader: &mut impl Read) -> Result<T> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    T::decode(&buf[..]).map_err(Into::into)
}

pub(crate) fn write_protobuf_message<T: Message>(writer: &mut impl Write, msg: &T) -> Result<()> {
    let len = msg.encoded_len() as u32;
    writer.write_all(&len.to_le_bytes())?;

    let mut buf = Vec::with_capacity(len as usize);
    msg.encode(&mut buf)?;
    writer.write_all(&buf)?;
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use zellij_utils::client_server_contract::client_server_contract::ClientToServerMsg as ProtoClientToServerMsg;

    use crate::proto_ipc::{conn_status_request, read_protobuf_message, write_protobuf_message};

    #[test]
    fn length_prefixed_proto_roundtrip_works() {
        let msg = conn_status_request();
        let mut buf = Vec::new();

        write_protobuf_message(&mut buf, &msg).unwrap();

        let decoded: ProtoClientToServerMsg = read_protobuf_message(&mut Cursor::new(buf)).unwrap();
        assert_eq!(decoded, msg);
    }
}
