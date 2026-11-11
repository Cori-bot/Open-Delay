use crate::{Error, Result, ABORT, SET_CHUNK_SIZE};
use bytes::{Buf, Bytes, BytesMut};
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_MESSAGE: usize = 16 * 1024 * 1024 - 1;
const MAX_PARTIAL_BYTES: usize = 32 * 1024 * 1024;
const MAX_CSID_COUNT: usize = 128;
const MAX_CHUNK: usize = 1024 * 1024;
const EXTENDED: u32 = 0x00ff_ffff;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub csid: u32,
    pub timestamp: u32,
    pub type_id: u8,
    pub stream_id: u32,
    pub payload: Bytes,
}

impl Message {
    pub fn is_media(&self) -> bool {
        matches!(self.type_id, crate::AUDIO | crate::VIDEO)
    }
}

#[derive(Clone)]
struct Header {
    timestamp: u32,
    delta: u32,
    length: usize,
    type_id: u8,
    stream_id: u32,
    extended: bool,
}

struct StreamState {
    header: Header,
    body: BytesMut,
    assembling: bool,
}

/// Incremental bounded decoder. Input and output are complete RTMP bytes/messages.
/// Chunk headers are committed only when their complete chunk payload is available.
pub struct ChunkDecoder {
    input: BytesMut,
    streams: HashMap<u32, StreamState>,
    chunk_size: usize,
    partial_bytes: usize,
    pub bytes_received: u64,
}

impl Default for ChunkDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkDecoder {
    pub fn new() -> Self {
        Self {
            input: BytesMut::new(),
            streams: HashMap::new(),
            chunk_size: 128,
            partial_bytes: 0,
            bytes_received: 0,
        }
    }

    pub fn feed(&mut self, input: &[u8]) -> Result<()> {
        if self.input.len().saturating_add(input.len()) > MAX_PARTIAL_BYTES {
            return Err(Error::Limit("input buffer"));
        }
        self.input.extend_from_slice(input);
        self.bytes_received = self.bytes_received.saturating_add(input.len() as u64);
        Ok(())
    }

    pub fn buffered_bytes(&self) -> usize {
        self.input.len() + self.partial_bytes
    }
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size as u32
    }

    pub fn next_message(&mut self) -> Result<Option<Message>> {
        loop {
            let data = &self.input[..];
            if data.is_empty() {
                return Ok(None);
            }
            let fmt = data[0] >> 6;
            let mut pos = 1;
            let csid = match data[0] & 0x3f {
                0 => {
                    if data.len() < 2 {
                        return Ok(None);
                    }
                    pos = 2;
                    64 + data[1] as u32
                }
                1 => {
                    if data.len() < 3 {
                        return Ok(None);
                    }
                    pos = 3;
                    64 + data[1] as u32 + 256 * data[2] as u32
                }
                n => n as u32,
            };
            let old = self.streams.get(&csid);
            let continuing = old.is_some_and(|state| state.assembling);
            if fmt != 3 && continuing {
                return Err(Error::Protocol(
                    "new header before previous message completed",
                ));
            }
            let required = match fmt {
                0 => 11,
                1 => 7,
                2 => 3,
                _ => 0,
            };
            if data.len() < pos + required {
                return Ok(None);
            }
            let mut header = match fmt {
                0 => {
                    let timestamp = u24(&data[pos..]);
                    let h = Header {
                        timestamp,
                        delta: timestamp,
                        length: u24(&data[pos + 3..]) as usize,
                        type_id: data[pos + 6],
                        stream_id: u32::from_le_bytes(data[pos + 7..pos + 11].try_into().unwrap()),
                        extended: timestamp == EXTENDED,
                    };
                    pos += 11;
                    h
                }
                1 | 2 => {
                    let mut h = old
                        .ok_or(Error::Protocol("compressed header without predecessor"))?
                        .header
                        .clone();
                    let delta = u24(&data[pos..]);
                    h.delta = delta;
                    h.extended = delta == EXTENDED;
                    if fmt == 1 {
                        h.length = u24(&data[pos + 3..]) as usize;
                        h.type_id = data[pos + 6];
                        pos += 7;
                    } else {
                        pos += 3;
                    }
                    h
                }
                3 => old
                    .ok_or(Error::Protocol("continuation without predecessor"))?
                    .header
                    .clone(),
                _ => unreachable!(),
            };
            if header.extended {
                if data.len() < pos + 4 {
                    return Ok(None);
                }
                let extended = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap());
                pos += 4;
                if fmt == 0 {
                    header.timestamp = extended;
                    header.delta = extended;
                } else if fmt == 1 || fmt == 2 {
                    header.delta = extended;
                }
                // Type 3 repeats the extension; its message timestamp follows the
                // preceding delta, while a continuation retains the same timestamp.
            }
            if fmt == 1 || fmt == 2 || (fmt == 3 && !continuing) {
                let previous = &old
                    .ok_or(Error::Protocol("missing timestamp predecessor"))?
                    .header;
                header.timestamp = previous.timestamp.wrapping_add(header.delta);
            }
            if header.length > MAX_MESSAGE {
                return Err(Error::Limit("message length"));
            }
            let previous_len = if continuing {
                old.unwrap().body.len()
            } else {
                0
            };
            if previous_len > header.length {
                return Err(Error::Protocol("payload exceeds declared length"));
            }
            let payload_len = self.chunk_size.min(header.length - previous_len);
            if data.len() < pos + payload_len {
                return Ok(None);
            }
            if self.partial_bytes.saturating_add(payload_len) > MAX_PARTIAL_BYTES {
                return Err(Error::Limit("interleaved payload budget"));
            }
            if !self.streams.contains_key(&csid) {
                if self.streams.len() >= MAX_CSID_COUNT {
                    return Err(Error::Limit("chunk stream count"));
                }
                self.streams.insert(
                    csid,
                    StreamState {
                        header: header.clone(),
                        body: BytesMut::new(),
                        assembling: false,
                    },
                );
            }
            let state = self.streams.get_mut(&csid).unwrap();
            if !continuing {
                state.body.clear();
            }
            state.header = header;
            state
                .body
                .extend_from_slice(&self.input[pos..pos + payload_len]);
            self.partial_bytes += payload_len;
            self.input.advance(pos + payload_len);
            state.assembling = state.body.len() < state.header.length;
            if !state.assembling {
                let message = Message {
                    csid,
                    timestamp: state.header.timestamp,
                    type_id: state.header.type_id,
                    stream_id: state.header.stream_id,
                    payload: state.body.split().freeze(),
                };
                self.partial_bytes -= message.payload.len();
                if message.type_id == SET_CHUNK_SIZE {
                    if message.payload.len() != 4 {
                        return Err(Error::Protocol("Set Chunk Size length"));
                    }
                    let size = u32::from_be_bytes(message.payload[..].try_into().unwrap()) as usize;
                    if size == 0 || size > MAX_CHUNK {
                        return Err(Error::Limit("chunk size must be 1..1048576"));
                    }
                    self.chunk_size = size;
                } else if message.type_id == ABORT {
                    if message.payload.len() != 4 {
                        return Err(Error::Protocol("Abort length"));
                    }
                    let aborted = u32::from_be_bytes(message.payload[..].try_into().unwrap());
                    if let Some(state) = self.streams.get_mut(&aborted) {
                        self.partial_bytes -= state.body.len();
                        state.body.clear();
                        state.assembling = false;
                    }
                }
                return Ok(Some(message));
            }
        }
    }
}

pub struct ChunkReader<R> {
    reader: R,
    decoder: ChunkDecoder,
}

impl<R: AsyncRead + Unpin> ChunkReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            decoder: ChunkDecoder::new(),
        }
    }
    pub fn bytes_received(&self) -> u64 {
        self.decoder.bytes_received
    }
    pub fn total_bytes_read(&self) -> u64 {
        self.decoder.bytes_received
    }
    pub fn into_inner(self) -> R {
        self.reader
    }
    /// Cancellation safe: bytes already read remain in the decoder.
    pub async fn read_message(&mut self) -> Result<Message> {
        loop {
            if let Some(message) = self.decoder.next_message()? {
                return Ok(message);
            }
            let mut bytes = [0; 8192];
            let count = self.reader.read(&mut bytes).await?;
            if count == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "RTMP peer closed",
                )
                .into());
            }
            self.decoder.feed(&bytes[..count])?;
        }
    }
}

pub struct ChunkWriter<W> {
    writer: W,
    chunk_size: usize,
    pub bytes_sent: u64,
}

impl<W: AsyncWrite + Unpin> ChunkWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            chunk_size: 128,
            bytes_sent: 0,
        }
    }
    pub fn into_inner(self) -> W {
        self.writer
    }
    pub fn set_chunk_size(&mut self, size: u32) -> Result<()> {
        validate_chunk_size(size)?;
        self.chunk_size = size as usize;
        Ok(())
    }
    /// Writes cannot be canceled and resumed: serialize them in one owner task.
    /// Sending Set Chunk Size changes framing only after that message is sent.
    pub async fn write_message(&mut self, message: &Message) -> Result<()> {
        validate_message(message)?;
        let extended = message.timestamp >= EXTENDED;
        let header = first_header(message);
        self.writer.write_all(&header).await?;
        self.bytes_sent += header.len() as u64;
        for (index, chunk) in message.payload.chunks(self.chunk_size).enumerate() {
            if index != 0 {
                let mut header = basic_header(3, message.csid);
                if extended {
                    header.extend_from_slice(&message.timestamp.to_be_bytes());
                }
                self.writer.write_all(&header).await?;
                self.bytes_sent += header.len() as u64;
            }
            self.writer.write_all(chunk).await?;
            self.bytes_sent += chunk.len() as u64;
        }
        self.writer.flush().await?;
        if message.type_id == SET_CHUNK_SIZE {
            self.set_chunk_size(u32::from_be_bytes(message.payload[..].try_into().unwrap()))?;
        }
        Ok(())
    }
}

/// Encode one message using full type-0 initial headers and type-3 continuations.
pub fn encode_message(message: &Message, chunk_size: u32) -> Result<Vec<u8>> {
    validate_message(message)?;
    validate_chunk_size(chunk_size)?;
    let mut output = first_header(message);
    for (index, chunk) in message.payload.chunks(chunk_size as usize).enumerate() {
        if index != 0 {
            output.extend(basic_header(3, message.csid));
            if message.timestamp >= EXTENDED {
                output.extend_from_slice(&message.timestamp.to_be_bytes());
            }
        }
        output.extend_from_slice(chunk);
    }
    Ok(output)
}

fn validate_chunk_size(size: u32) -> Result<()> {
    if size == 0 || size as usize > MAX_CHUNK {
        Err(Error::Limit("chunk size"))
    } else {
        Ok(())
    }
}
fn validate_message(message: &Message) -> Result<()> {
    if !(2..=65599).contains(&message.csid) {
        return Err(Error::Protocol("chunk stream id"));
    }
    if message.payload.len() > MAX_MESSAGE {
        return Err(Error::Limit("message length"));
    }
    if message.type_id == SET_CHUNK_SIZE {
        if message.payload.len() != 4 {
            return Err(Error::Protocol("Set Chunk Size length"));
        }
        validate_chunk_size(u32::from_be_bytes(message.payload[..].try_into().unwrap()))?;
    }
    Ok(())
}
fn u24(input: &[u8]) -> u32 {
    ((input[0] as u32) << 16) | ((input[1] as u32) << 8) | input[2] as u32
}
fn put_u24(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes()[1..]);
}
fn basic_header(fmt: u8, csid: u32) -> Vec<u8> {
    if csid < 64 {
        vec![(fmt << 6) | csid as u8]
    } else if csid < 320 {
        vec![fmt << 6, (csid - 64) as u8]
    } else {
        vec![(fmt << 6) | 1, (csid - 64) as u8, ((csid - 64) >> 8) as u8]
    }
}
fn first_header(message: &Message) -> Vec<u8> {
    let mut header = basic_header(0, message.csid);
    put_u24(&mut header, message.timestamp.min(EXTENDED));
    put_u24(&mut header, message.payload.len() as u32);
    header.push(message.type_id);
    header.extend_from_slice(&message.stream_id.to_le_bytes());
    if message.timestamp >= EXTENDED {
        header.extend_from_slice(&message.timestamp.to_be_bytes());
    }
    header
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message(csid: u32, timestamp: u32, len: usize) -> Message {
        Message {
            csid,
            timestamp,
            type_id: 20,
            stream_id: 1,
            payload: (0..len).map(|i| (i % 251) as u8).collect::<Vec<_>>().into(),
        }
    }
    #[test]
    fn fragmented_connect_larger_than_initial_chunk_has_no_duplicate_prefix() {
        let value = message(3, 0, 901);
        let encoded = encode_message(&value, 128).unwrap();
        let mut decoder = ChunkDecoder::new();
        for (i, byte) in encoded.iter().enumerate() {
            decoder.feed(&[*byte]).unwrap();
            let decoded = decoder.next_message().unwrap();
            if i + 1 == encoded.len() {
                assert_eq!(decoded, Some(value.clone()));
            } else {
                assert!(decoded.is_none());
            }
        }
        assert_eq!(decoder.buffered_bytes(), 0);
    }
    #[test]
    fn extended_timestamps_and_all_basic_header_sizes_round_trip() {
        for csid in [2, 63, 64, 319, 320, 65599] {
            let value = message(csid, 0xfedcba98, 389);
            let mut decoder = ChunkDecoder::new();
            decoder.feed(&encode_message(&value, 128).unwrap()).unwrap();
            assert_eq!(decoder.next_message().unwrap(), Some(value));
        }
    }
    #[test]
    fn chunk_streams_can_interleave_incomplete_messages() {
        let a = message(3, 40, 200);
        let b = message(4, 55, 200);
        let a_bytes = encode_message(&a, 128).unwrap();
        let b_bytes = encode_message(&b, 128).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&a_bytes[..140]);
        bytes.extend_from_slice(&b_bytes[..140]);
        bytes.extend_from_slice(&a_bytes[140..]);
        bytes.extend_from_slice(&b_bytes[140..]);
        let mut decoder = ChunkDecoder::new();
        decoder.feed(&bytes).unwrap();
        assert_eq!(decoder.next_message().unwrap(), Some(a));
        assert_eq!(decoder.next_message().unwrap(), Some(b));
    }
    #[test]
    fn compressed_headers_inherit_stream_and_apply_deltas() {
        let mut bytes = encode_message(&message(3, 10, 1), 128).unwrap();
        bytes.extend_from_slice(&[0x43, 0, 0, 5, 0, 0, 1, 20, 91]); // type 1, timestamp 15
        bytes.extend_from_slice(&[0x83, 0, 0, 7, 92]); // type 2, timestamp 22
        bytes.extend_from_slice(&[0xc3, 93]); // type 3 new message, timestamp 29
        let mut decoder = ChunkDecoder::new();
        decoder.feed(&bytes).unwrap();
        for expected in [10, 15, 22, 29] {
            let m = decoder.next_message().unwrap().unwrap();
            assert_eq!(m.timestamp, expected);
            assert_eq!(m.stream_id, 1);
        }
    }
    #[test]
    fn type3_after_type0_uses_absolute_timestamp_as_delta() {
        let mut bytes = encode_message(&message(3, 10, 1), 128).unwrap();
        bytes.extend_from_slice(&[0xc3, 9]);
        let mut decoder = ChunkDecoder::new();
        decoder.feed(&bytes).unwrap();
        decoder.next_message().unwrap();
        assert_eq!(decoder.next_message().unwrap().unwrap().timestamp, 20);
    }
    #[test]
    fn abort_discards_only_target_partial_message() {
        let a = encode_message(&message(3, 0, 200), 128).unwrap();
        let abort = crate::control(ABORT, 3u32.to_be_bytes().to_vec());
        let mut bytes = a[..140].to_vec();
        bytes.extend_from_slice(&encode_message(&abort, 128).unwrap());
        bytes.extend_from_slice(&encode_message(&message(3, 30, 1), 128).unwrap());
        let mut decoder = ChunkDecoder::new();
        decoder.feed(&bytes).unwrap();
        assert_eq!(decoder.next_message().unwrap(), Some(abort));
        assert_eq!(decoder.next_message().unwrap().unwrap().timestamp, 30);
        assert_eq!(decoder.buffered_bytes(), 0);
    }
    #[test]
    fn rejects_unbounded_or_invalid_chunk_size() {
        let bad = Message {
            csid: 2,
            timestamp: 0,
            type_id: 1,
            stream_id: 0,
            payload: 0u32.to_be_bytes().to_vec().into(),
        };
        assert!(encode_message(&bad, 128).is_err());
        let mut decoder = ChunkDecoder::new();
        decoder
            .feed(&[2, 0, 0, 0, 0, 0, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0])
            .unwrap();
        assert!(decoder.next_message().is_err());
    }
    #[tokio::test]
    async fn negotiated_chunk_size_applies_after_complete_control_message() {
        let (send, recv) = tokio::io::duplex(17);
        let value = message(5, 65535, 2048);
        let sent = value.clone();
        let task = tokio::spawn(async move {
            let mut w = ChunkWriter::new(send);
            w.write_message(&crate::control(1, 512u32.to_be_bytes().to_vec()))
                .await
                .unwrap();
            w.write_message(&sent).await.unwrap();
        });
        let mut reader = ChunkReader::new(recv);
        assert_eq!(reader.read_message().await.unwrap().type_id, 1);
        assert_eq!(reader.read_message().await.unwrap(), value);
        task.await.unwrap();
    }
}
