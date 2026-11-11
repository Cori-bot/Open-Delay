//! Independent RTMP framing. No target application code is included.
pub mod amf0;
mod chunk;
mod handshake;

pub use chunk::{encode_message, ChunkDecoder, ChunkReader, ChunkWriter, Message};
pub use handshake::{client_handshake, server_handshake};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("RTMP I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid RTMP data: {0}")]
    Protocol(&'static str),
    #[error("RTMP resource limit: {0}")]
    Limit(&'static str),
    #[error("invalid AMF0: {0}")]
    Amf(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

pub const SET_CHUNK_SIZE: u8 = 1;
pub const ABORT: u8 = 2;
pub const ACKNOWLEDGEMENT: u8 = 3;
pub const USER_CONTROL: u8 = 4;
pub const WINDOW_ACK_SIZE: u8 = 5;
pub const SET_PEER_BANDWIDTH: u8 = 6;
pub const AUDIO: u8 = 8;
pub const VIDEO: u8 = 9;
pub const DATA_AMF3: u8 = 15;
pub const COMMAND_AMF3: u8 = 17;
pub const DATA_AMF0: u8 = 18;
pub const COMMAND_AMF0: u8 = 20;
pub const AGGREGATE: u8 = 22;

/// A protocol control message on CSID 2 and message stream 0.
pub fn control(type_id: u8, payload: impl Into<bytes::Bytes>) -> Message {
    Message {
        csid: 2,
        timestamp: 0,
        type_id,
        stream_id: 0,
        payload: payload.into(),
    }
}

pub fn user_control(event: u16, value: u32) -> Message {
    let mut bytes = Vec::with_capacity(6);
    bytes.extend_from_slice(&event.to_be_bytes());
    bytes.extend_from_slice(&value.to_be_bytes());
    control(USER_CONTROL, bytes)
}

/// AMF0 command helpers. `arguments` includes the command object (usually Null).
pub fn command_message(
    name: &str,
    transaction_id: f64,
    stream_id: u32,
    arguments: Vec<amf0::Value>,
) -> Result<Message> {
    let mut values = vec![
        amf0::Value::String(name.into()),
        amf0::Value::Number(transaction_id),
    ];
    values.extend(arguments);
    Ok(Message {
        csid: 3,
        timestamp: 0,
        type_id: COMMAND_AMF0,
        stream_id,
        payload: amf0::encode(&values)?.into(),
    })
}

pub fn connect_result(transaction_id: f64) -> Result<Message> {
    use amf0::Value as V;
    command_message(
        "_result",
        transaction_id,
        0,
        vec![
            V::object([
                ("fmsVer", V::String("FMS/3,5,7,7009".into())),
                ("capabilities", V::Number(31.0)),
            ]),
            V::object([
                ("level", V::String("status".into())),
                ("code", V::String("NetConnection.Connect.Success".into())),
                ("description", V::String("Connection succeeded".into())),
                ("objectEncoding", V::Number(0.0)),
            ]),
        ],
    )
}

pub fn create_stream_result(transaction_id: f64, stream_id: u32) -> Result<Message> {
    command_message(
        "_result",
        transaction_id,
        0,
        vec![amf0::Value::Null, amf0::Value::Number(stream_id as f64)],
    )
}

pub fn on_status(stream_id: u32, code: &str, description: &str) -> Result<Message> {
    use amf0::Value as V;
    command_message(
        "onStatus",
        0.0,
        stream_id,
        vec![
            V::Null,
            V::object([
                (
                    "level",
                    V::String(
                        if code.ends_with("Failed") || code.ends_with("BadName") {
                            "error"
                        } else {
                            "status"
                        }
                        .into(),
                    ),
                ),
                ("code", V::String(code.into())),
                ("description", V::String(description.into())),
            ]),
        ],
    )
}
