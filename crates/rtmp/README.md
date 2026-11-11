# open-delay-rtmp

Independent protocol implementation based on the [Adobe RTMP specification](https://rtmp.veriskope.com/pdf/rtmp_specification_1.0.pdf). No extracted proprietary code or GPL implementation is included. Dependencies: bytes (MIT), thiserror (MIT/Apache-2.0), Tokio (MIT).

## Integration

1. `server_handshake(&mut tcp).await?` for an incoming publisher; `client_handshake(&mut tcp).await?` for upstream. Both support RTMP version 3 simple handshakes. Enforce a timeout outside these functions. Complex-only handshake servers are not supported.
2. Split the transport and construct `ChunkReader::new(read_half)`, `ChunkWriter::new(write_half)`.
3. `read_message().await?` returns `Message { csid, timestamp, type_id, stream_id, payload: bytes::Bytes }`; it automatically processes Set Chunk Size and Abort, still returning those control messages to its caller. `bytes_received()` supplies raw bytes for window acknowledgements. The read operation is cancellation safe.
4. `write_message(&message).await?` uses full initial headers and type-3 continuations; writing Set Chunk Size updates subsequent output framing. Serialize writes in a single task; do not cancel a write and reuse its transport. The writer exposes `bytes_sent` and supports `into_inner()`.
5. `amf0::command(&message)?` returns `{name, transaction_id, arguments}`. Connect object is `arguments[0]`; publish/play key is usually `arguments[1]`. `Value::get`, `as_str`, `as_number` assist parsing. `command_message` arguments include the command object, usually `Null`.
6. Response helpers: `connect_result(transaction_id)`, `create_stream_result(transaction_id, stream_id)`, `on_status(stream_id, code, description)`. `control(type_id,payload)` and `user_control(event,value)` create protocol controls. These helpers do not manage a server/client state machine.

The session layer owns command sequencing, stream IDs, route admission, acknowledgements, ping replies, timeouts, URL validation, TLS and backpressure. Reject unsupported AMF3-native commands explicitly. AMF3 RTMP commands with the AMF0 switch byte are accepted. FLV aggregate messages are returned unchanged for the session/media layer to handle.

Decoder limits: 16 MiB minus one byte per message (RTMP 24-bit length), 128 chunk streams, chunk size 1..1 MiB, partial payloads 32 MiB, input staging 32 MiB. Drain the decoder between input feeds. AMF0 limits: depth 32, 100,000 values, 16 MiB minus one byte; unknown markers and invalid UTF-8 fail explicitly.
