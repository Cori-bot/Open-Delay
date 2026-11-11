//! Independent RTMP ingest, upstream publication and disk-backed playback.
//! Raw connect/publish values and upstream errors are deliberately never logged.
use crate::{Error, Service};
use bytes::Bytes;
use open_delay_core::{
    Config, DiskHistory, MediaProgress, PacketMeta, Route, TimelineAction, TimelineActionKind,
};
use open_delay_rtmp::{
    self as rtmp,
    amf0::{self, Value},
    ChunkReader, ChunkWriter, Message,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Notify, Semaphore},
    task::JoinSet,
    time::{timeout, Instant},
};
use tokio_util::sync::CancellationToken;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const PUBLISHER_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_CONTEXT_BYTES: usize = 1024 * 1024;
const MAX_PACKET_BYTES: usize = 8 * 1024 * 1024;
const ACK_WINDOW: u32 = 5_000_000;
const RETRY_SECONDS: [u64; 6] = [1, 2, 4, 8, 15, 30];

#[derive(Debug, Clone, Copy)]
struct Fault(&'static str);
impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Fault {}
type EngineResult<T> = std::result::Result<T, Fault>;

pub fn open_history(directory: &Path, config: &Config) -> Result<DiskHistory, Error> {
    let path = directory.join(format!("history-{}.ring", uuid::Uuid::new_v4()));
    Ok(DiskHistory::new(
        path,
        config.preferences.disk_bytes,
        config.preferences.cache_bytes,
    )?)
}

struct SessionHandle {
    generation: u64,
    commands: mpsc::Sender<TimelineAction>,
    cancel: CancellationToken,
    publish_digest: [u8; 32],
}
type Sessions = Arc<Mutex<HashMap<String, SessionHandle>>>;

fn admit_route(
    controller: &Mutex<open_delay_core::Controller>,
    shutdown: &CancellationToken,
    route_id: &str,
) -> EngineResult<u64> {
    let mut controller = controller
        .lock()
        .map_err(|_| Fault("Controller unavailable"))?;
    // The updater checks idle state and cancels shutdown under this same lock.
    // A publisher queued behind that check must never become an active route.
    if shutdown.is_cancelled() {
        return Err(Fault("Local engine is stopping"));
    }
    controller
        .connect(route_id)
        .map_err(|_| Fault("Route admission failed"))
}

/// All connection work is owned by this server and joined during shutdown.
pub async fn serve(listener: TcpListener, service: Service) {
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    let permits = Arc::new(Semaphore::new(32));
    let mut tasks = JoinSet::new();
    let mut dispatcher = tokio::time::interval(Duration::from_millis(20));
    dispatcher.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _=service.shutdown.cancelled()=>break,
            accepted=listener.accept()=>{
                let Ok((socket,_))=accepted else {break;};
                let Ok(permit)=permits.clone().try_acquire_owned() else {drop(socket);continue;};
                let child=service.clone();let registry=sessions.clone();
                tasks.spawn(async move {let _permit=permit;let _=publisher(socket,child,registry).await;});
            }
            _=dispatcher.tick()=>{
                let actions=match service.controller.lock(){Ok(mut c)=>c.drain_actions(),Err(_)=>break};
                for action in actions {
                    if let Ok(registry)=sessions.lock() {
                        if let Some(session)=registry.get(&action.route_id) {
                            if session.generation==action.generation && session.commands.try_send(action).is_err() {
                                // A bounded command queue must never grow without limit.
                                session.cancel.cancel();
                            }
                        }
                    }
                }
            }
            _=tasks.join_next(), if !tasks.is_empty()=>{}
        }
    }
    if let Ok(registry) = sessions.lock() {
        for session in registry.values() {
            session.cancel.cancel();
        }
    }
    // Socket handshakes and idle readers observe this cancellation branch as well.
    while !tasks.is_empty() {
        if timeout(Duration::from_secs(12), tasks.join_next())
            .await
            .is_err()
        {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
            break;
        }
    }
}

#[derive(Default)]
struct SessionMedia {
    configs: HashMap<u8, Message>,
    latest_source: u64,
    bytes_received: u64,
    bytes_sent: u64,
    output_source: Option<u64>,
    received_media: bool,
    publisher_closed: bool,
    cadence: HashMap<u8, TrackCadence>,
    output_sequence: Option<u64>,
}

#[derive(Default)]
struct TrackCadence {
    previous: Option<u64>,
    intervals: VecDeque<u64>,
}
impl TrackCadence {
    fn observe(&mut self, source: u64) {
        if let Some(previous) = self.previous {
            if let Some(interval @ 1..=1000) = source.checked_sub(previous) {
                if self.intervals.len() == 16 {
                    self.intervals.pop_front();
                }
                self.intervals.push_back(interval);
            }
        }
        self.previous = Some(source);
    }
    fn spacing(&self) -> u64 {
        // RTMP has millisecond timestamps. A 30fps source alternates 33/34ms;
        // retain the upper interval so a new frame cannot round to the previous
        // frame in a receiver's coarser 1/fps timebase.
        self.intervals.iter().copied().max().unwrap_or(1)
    }
}

async fn publisher(
    mut socket: TcpStream,
    service: Service,
    sessions: Sessions,
) -> EngineResult<()> {
    let _ = socket.set_nodelay(true);
    tokio::select! {
        _=service.shutdown.cancelled()=>return Ok(()),
        result=timeout(HANDSHAKE_TIMEOUT,rtmp::server_handshake(&mut socket))=>{
            result.map_err(|_|Fault("Publisher handshake timed out"))?.map_err(|_|Fault("Invalid publisher handshake"))?;
        }
    }
    let (read, write_half) = socket.into_split();
    let mut reader = ChunkReader::new(read);
    let mut writer = ChunkWriter::new(write_half);
    let mut route = None;
    let mut published = None;
    let mut stream_created = false;
    // A client must connect and publish within a bounded setup window.
    let setup_deadline = Instant::now() + Duration::from_secs(30);
    while published.is_none() {
        let message = tokio::select! {
            _=service.shutdown.cancelled()=>return Ok(()),
            result=timeout_at(setup_deadline,reader.read_message())=>result.map_err(|_|Fault("Publisher setup timed out"))?.map_err(|_|Fault("Publisher setup read failed"))?,
        };
        if message.type_id == rtmp::USER_CONTROL {
            reply_ping(&mut writer, &message).await?;
            continue;
        }
        if !matches!(message.type_id, rtmp::COMMAND_AMF0 | rtmp::COMMAND_AMF3) {
            continue;
        }
        let command =
            amf0::command(&message).map_err(|_| Fault("Unsupported publisher command encoding"))?;
        match command.name.as_str() {
            "connect" => {
                if route.is_some() {
                    return Err(Fault("Duplicate publisher connect"));
                }
                let app = command
                    .arguments
                    .first()
                    .and_then(|v| v.get("app"))
                    .and_then(Value::as_str)
                    .ok_or(Fault("Publisher connect requires an application path"))?;
                let local_app = app.split('?').next().unwrap_or(app).trim_matches('/');
                let chosen = {
                    let controller = service
                        .controller
                        .lock()
                        .map_err(|_| Fault("Controller unavailable"))?;
                    controller
                        .config()
                        .routes
                        .iter()
                        .find(|r| r.local_app == local_app && r.enabled)
                        .cloned()
                };
                let Some(chosen) = chosen else {
                    write(
                        &mut writer,
                        &rtmp::on_status(
                            0,
                            "NetConnection.Connect.Rejected",
                            "Local route is disabled or unknown",
                        )
                        .map_err(|_| Fault("Command encoding failed"))?,
                    )
                    .await?;
                    return Err(Fault("Local route is disabled or unknown"));
                };
                write(
                    &mut writer,
                    &rtmp::control(rtmp::WINDOW_ACK_SIZE, ACK_WINDOW.to_be_bytes().to_vec()),
                )
                .await?;
                let mut bandwidth = ACK_WINDOW.to_be_bytes().to_vec();
                bandwidth.push(2);
                write(
                    &mut writer,
                    &rtmp::control(rtmp::SET_PEER_BANDWIDTH, bandwidth),
                )
                .await?;
                write(
                    &mut writer,
                    &rtmp::control(rtmp::SET_CHUNK_SIZE, 4096u32.to_be_bytes().to_vec()),
                )
                .await?;
                write(
                    &mut writer,
                    &rtmp::connect_result(command.transaction_id)
                        .map_err(|_| Fault("Command encoding failed"))?,
                )
                .await?;
                route = Some(chosen);
            }
            "releaseStream" | "FCPublish" | "FCUnpublish" => {
                write(
                    &mut writer,
                    &rtmp::command_message(
                        "_result",
                        command.transaction_id,
                        0,
                        vec![Value::Null, Value::Undefined],
                    )
                    .map_err(|_| Fault("Command encoding failed"))?,
                )
                .await?;
            }
            "createStream" => {
                if route.is_none() {
                    return Err(Fault("createStream before connect"));
                }
                write(
                    &mut writer,
                    &rtmp::create_stream_result(command.transaction_id, 1)
                        .map_err(|_| Fault("Command encoding failed"))?,
                )
                .await?;
                stream_created = true;
            }
            "publish" => {
                if route.is_none() || !stream_created || message.stream_id != 1 {
                    return Err(Fault(
                        "publish before createStream or invalid message stream",
                    ));
                }
                let key = command
                    .arguments
                    .get(1)
                    .and_then(Value::as_str)
                    .ok_or(Fault("Missing publish name"))?;
                if key.is_empty() || key.len() > 4096 {
                    return Err(Fault("Invalid publish name length"));
                }
                published = Some((key.to_owned(), message.stream_id));
            }
            "deleteStream" | "closeStream" => return Ok(()),
            _ => {}
        }
    }
    let route = route.ok_or(Fault("Publisher route missing"))?;
    let (key, stream_id) = published.ok_or(Fault("Publisher name missing"))?;
    let cancel = service.shutdown.child_token();
    let _cancel_guard = cancel.clone().drop_guard();
    let (commands_tx, commands_rx) = mpsc::channel(32);
    let publish_digest: [u8; 32] = Sha256::digest(key.as_bytes()).into();
    let admission = {
        let mut registry = sessions
            .lock()
            .map_err(|_| Fault("Session registry unavailable"))?;
        if registry
            .get(&route.id)
            .is_some_and(|current| !same_publish_name(&current.publish_digest, &publish_digest))
        {
            None
        } else {
            // Admission and replacement share the registry lock. A competing
            // publisher cannot increment generation before proving the same
            // ephemeral publish name as the currently active session.
            let generation = admit_route(&service.controller, &service.shutdown, &route.id)?;
            if let Some(previous) = registry.insert(
                route.id.clone(),
                SessionHandle {
                    generation,
                    commands: commands_tx,
                    cancel: cancel.clone(),
                    publish_digest,
                },
            ) {
                previous.cancel.cancel();
            }
            Some(generation)
        }
    };
    let Some(generation) = admission else {
        write(
            &mut writer,
            &rtmp::on_status(
                stream_id,
                "NetStream.Publish.BadName",
                "This route already has a different active publisher",
            )
            .map_err(|_| Fault("Command encoding failed"))?,
        )
        .await?;
        return Err(Fault("A different publisher already owns this route"));
    };
    service.record("info", &format!("Route {}: publisher admitted", route.id));
    let media = Arc::new(Mutex::new(SessionMedia::default()));
    let notify = Arc::new(Notify::new());
    let mut output = tokio::spawn(playback(
        service.clone(),
        route.clone(),
        generation,
        key,
        media.clone(),
        notify.clone(),
        commands_rx,
        cancel.clone(),
    ));
    service.publish();
    let result=async {
        write(&mut writer,&rtmp::user_control(0,stream_id)).await?;
        write(&mut writer,&rtmp::on_status(stream_id,"NetStream.Publish.Start","Publishing to local delay route").map_err(|_|Fault("Command encoding failed"))?).await?;
        let mut clock=SourceClock::default();let mut last_ack=0u64;
        let mut last_input=Instant::now();
        let mut progress_tick=tokio::time::interval(Duration::from_millis(100));
        progress_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _=cancel.cancelled()=>return Ok(()),
                result=&mut output=>return match result {Ok(result)=>result,Err(_)=>Err(Fault("Playback task stopped unexpectedly"))},
                _=tokio::time::sleep_until(last_input+PUBLISHER_IDLE_TIMEOUT)=>return Err(Fault("Publisher has been idle for 30 seconds")),
                _=progress_tick.tick()=>{report_progress(&service,&route.id,generation,&media)?;}
                result=reader.read_message()=>{
                    let message=match result {Ok(message)=>message,Err(rtmp::Error::Io(e)) if e.kind()==std::io::ErrorKind::UnexpectedEof=>return Ok(()),Err(_)=>return Err(Fault("Publisher RTMP framing failed"))};
                    last_input=Instant::now();
                    let received=reader.total_bytes_read();
                    if received.saturating_sub(last_ack)>=u64::from(ACK_WINDOW/2) {
                        write(&mut writer,&rtmp::control(rtmp::ACKNOWLEDGEMENT,(received as u32).to_be_bytes().to_vec())).await?;last_ack=received;
                    }
                    match message.type_id {
                        rtmp::USER_CONTROL=>reply_ping(&mut writer,&message).await?,
                        rtmp::COMMAND_AMF0|rtmp::COMMAND_AMF3=>{
                            let command=amf0::command(&message).map_err(|_|Fault("Unsupported publisher command encoding"))?;
                            if matches!(command.name.as_str(),"deleteStream"|"closeStream"|"FCUnpublish"){return Ok(());}
                        }
                        rtmp::AUDIO|rtmp::VIDEO|rtmp::DATA_AMF0|rtmp::DATA_AMF3=>{
                            if message.stream_id!=stream_id{return Err(Fault("Multiple message streams per route are unsupported"));}
                            append_media(&service,&route.id,generation,&media,&mut clock,&message,received)?;
                            notify.notify_one();
                        }
                        rtmp::AGGREGATE=>return Err(Fault("Aggregate RTMP media is not supported by this build")),
                        _=>{}
                    }
                }
            }
        }
    }.await;
    if let Ok(mut shared) = media.lock() {
        shared.publisher_closed = true;
    }
    // Notify before cancellation so a final complete message can be observed;
    // termination intentionally does not drain an obsolete delayed session.
    notify.notify_waiters();
    cancel.cancel();
    if !output.is_finished() {
        let _ = timeout(Duration::from_secs(11), &mut output).await;
        if !output.is_finished() {
            output.abort();
            let _ = output.await;
        }
    }
    {
        let mut registry = sessions
            .lock()
            .map_err(|_| Fault("Session registry unavailable"))?;
        if registry
            .get(&route.id)
            .is_some_and(|s| s.generation == generation)
        {
            registry.remove(&route.id);
        }
    }
    if let Ok(mut controller) = service.controller.lock() {
        match result {
            Err(fault) => {
                let _ = controller.disconnect_with_error(&route.id, generation, fault.0.into());
            }
            Ok(()) => {
                let _ = controller.disconnect(&route.id, generation);
            }
        }
    }
    if let Ok(mut history) = service.history.lock() {
        history.remove_route(&route.id, generation);
    }
    service.record(
        if result.is_err() { "error" } else { "info" },
        &format!(
            "Route {}: {}",
            route.id,
            match result {
                Err(fault) => fault.0,
                Ok(()) => "publisher disconnected",
            }
        ),
    );
    service.publish();
    result
}

fn same_publish_name(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

async fn timeout_at<F: std::future::Future>(
    deadline: Instant,
    future: F,
) -> std::result::Result<F::Output, tokio::time::error::Elapsed> {
    tokio::time::timeout_at(deadline, future).await
}

#[derive(Default)]
struct SourceClock {
    last: Option<u32>,
    epoch: u64,
}
impl SourceClock {
    fn unwrap(&mut self, raw: u32) -> EngineResult<u64> {
        if let Some(last) = self.last {
            if raw < last && last.wrapping_sub(raw) > 0x8000_0000 {
                self.epoch = self
                    .epoch
                    .checked_add(1u64 << 32)
                    .ok_or(Fault("Source timeline exhausted"))?;
            } else if raw > last && raw.wrapping_sub(last) > 0x8000_0000 {
                return Ok(self.epoch.saturating_sub(1u64 << 32) + u64::from(raw));
            } else if raw < last && last - raw > 10_000 {
                return Err(Fault("Encoder timestamp reset requires a new session"));
            }
        }
        self.last = Some(raw);
        Ok(self.epoch + u64::from(raw))
    }
}

fn append_media(
    service: &Service,
    id: &str,
    generation: u64,
    media: &Arc<Mutex<SessionMedia>>,
    clock: &mut SourceClock,
    message: &Message,
    received: u64,
) -> EngineResult<()> {
    if message.payload.len() > MAX_PACKET_BYTES {
        return Err(Fault("Media packet exceeds the bounded packet budget"));
    }
    if message.payload.is_empty() {
        return Err(Fault("Empty media message"));
    }
    let (configuration, keyframe) = classify(message)?;
    let timestamp = if configuration || matches!(message.type_id, rtmp::DATA_AMF0 | rtmp::DATA_AMF3)
    {
        media
            .lock()
            .map_err(|_| Fault("Media state unavailable"))?
            .latest_source
    } else {
        clock.unwrap(message.timestamp)?
    };
    {
        let mut shared = media.lock().map_err(|_| Fault("Media state unavailable"))?;
        if configuration || matches!(message.type_id, rtmp::DATA_AMF0 | rtmp::DATA_AMF3) {
            if message.payload.len() > MAX_CONTEXT_BYTES {
                return Err(Fault("Codec or metadata context exceeds 1 MiB"));
            }
            if configuration
                && shared
                    .configs
                    .get(&message.type_id)
                    .is_some_and(|old| old.payload != message.payload)
            {
                return Err(Fault(
                    "Midstream codec configuration changes require a new session",
                ));
            }
            shared.configs.insert(message.type_id, message.clone());
        } else if message.is_media() && !shared.configs.contains_key(&message.type_id) {
            return Err(Fault("Media arrived before its codec configuration"));
        }
        if message.is_media() && !configuration {
            shared.latest_source = shared.latest_source.max(timestamp);
            shared.received_media = true;
            shared
                .cadence
                .entry(message.type_id)
                .or_default()
                .observe(timestamp);
        }
        shared.bytes_received = received;
    }
    service
        .history
        .lock()
        .map_err(|_| Fault("History unavailable"))?
        .append(
            id,
            generation,
            timestamp,
            message.type_id,
            message.csid,
            message.stream_id,
            &message.payload,
            keyframe,
            configuration,
        )
        .map_err(|_| Fault("History storage failed or capacity is insufficient"))?;
    Ok(())
}

fn classify(message: &Message) -> EngineResult<(bool, bool)> {
    match message.type_id {
        rtmp::VIDEO => {
            if message.payload.len() < 5
                || message.payload[0] & 0x80 != 0
                || message.payload[0] & 0x0f != 7
            {
                return Err(Fault("Only H.264 AVC video is supported by this build"));
            }
            if message.payload[1] > 2 {
                return Err(Fault("Unsupported AVC packet type"));
            }
            Ok((
                message.payload[1] == 0,
                message.payload[0] >> 4 == 1 && message.payload[1] == 1,
            ))
        }
        rtmp::AUDIO => {
            if message.payload.len() < 2 || message.payload[0] >> 4 != 10 {
                return Err(Fault("Only AAC audio is supported by this build"));
            }
            if message.payload[1] > 1 {
                return Err(Fault("Unsupported AAC packet type"));
            }
            Ok((message.payload[1] == 0, false))
        }
        rtmp::DATA_AMF0 | rtmp::DATA_AMF3 => Ok((false, false)),
        _ => Err(Fault("Unsupported media message type")),
    }
}

fn report_progress(
    service: &Service,
    id: &str,
    generation: u64,
    media: &Arc<Mutex<SessionMedia>>,
) -> EngineResult<()> {
    let (latest, received, sent, has_media, output, output_sequence) = {
        let shared = media.lock().map_err(|_| Fault("Media state unavailable"))?;
        (
            shared.latest_source,
            shared.bytes_received,
            shared.bytes_sent,
            shared.received_media,
            shared.output_source,
            shared.output_sequence,
        )
    };
    if !has_media {
        return Ok(());
    }
    let (retention, protection) = {
        let controller = service
            .controller
            .lock()
            .map_err(|_| Fault("Controller unavailable"))?;
        let retention = controller.config().preferences.buffer_ms;
        let protection = controller
            .snapshot()
            .routes
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| {
                r.effective_delay_ms
                    .max(r.desired_delay_ms)
                    .max(r.actual_latency_ms)
            })
            .unwrap_or(0)
            .max(
                output
                    .map(|source| latest.saturating_sub(source))
                    .unwrap_or(0),
            );
        (retention, protection)
    };
    let (oldest, size) = {
        let mut history = service
            .history
            .lock()
            .map_err(|_| Fault("History unavailable"))?;
        // Keep the keyframe before the configured/protected window, plus one
        // second of scheduler headroom. Never prune the current output cursor
        // while a fresh live keyframe or an upstream reconnect is pending.
        let physical_window = retention.max(protection).saturating_add(1000);
        if let Some(cutoff) =
            history.seek_keyframe(id, generation, latest.saturating_sub(physical_window))
        {
            let retain_sequence =
                output_sequence.map_or(cutoff.sequence, |cursor| cursor.min(cutoff.sequence));
            history.retain_from(id, generation, retain_sequence);
        }
        // Report the actual decodable window, including history deliberately
        // protected while output is stalled. Reporting only the configured
        // duration would falsely mark a retained reconnect cursor as expired.
        let oldest = history.first_keyframe_after(id, generation, 0);
        (
            oldest.map(|p| p.timestamp_ms).unwrap_or(latest),
            history.route_bytes(id, generation),
        )
    };
    let mut controller = service
        .controller
        .lock()
        .map_err(|_| Fault("Controller unavailable"))?;
    controller
        .progress(
            id,
            generation,
            MediaProgress {
                latest_source_ms: latest,
                oldest_source_ms: oldest.min(latest),
                bytes_received: received,
                bytes_sent: sent,
                stored_bytes: size,
            },
        )
        .map_err(|_| Fault("Route session was superseded"))?;
    if let Some(source) = output {
        controller
            .observed_output(id, generation, source)
            .map_err(|_| Fault("Route session was superseded"))?;
    }
    drop(controller);
    service.publish();
    Ok(())
}

async fn write<W: AsyncWrite + Unpin>(
    writer: &mut ChunkWriter<W>,
    message: &Message,
) -> EngineResult<()> {
    timeout(WRITE_TIMEOUT, writer.write_message(message))
        .await
        .map_err(|_| Fault("RTMP write timed out"))?
        .map_err(|_| Fault("RTMP write failed"))
}

async fn reply_ping<W: AsyncWrite + Unpin>(
    writer: &mut ChunkWriter<W>,
    message: &Message,
) -> EngineResult<()> {
    if message.payload.len() >= 6 && message.payload[..2] == [0, 6] {
        let value = u32::from_be_bytes(
            message.payload[2..6]
                .try_into()
                .map_err(|_| Fault("Invalid ping"))?,
        );
        write(writer, &rtmp::user_control(7, value)).await?;
    }
    Ok(())
}

trait Transport: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> Transport for T {}
type Wire = Box<dyn Transport>;
type UpReader = ChunkReader<ReadHalf<Wire>>;
type UpWriter = ChunkWriter<WriteHalf<Wire>>;

struct Upstream {
    reader: UpReader,
    writer: UpWriter,
    stream_id: u32,
    ack_window: u32,
    last_ack: u64,
}

fn upstream_command(message: &Message) -> EngineResult<Option<amf0::Command>> {
    let payload = if message.type_id == rtmp::COMMAND_AMF3 {
        message
            .payload
            .strip_prefix(&[0])
            .ok_or(Fault("Unsupported upstream command encoding"))?
    } else {
        &message.payload[..]
    };
    // FFmpeg listen emits these historical advisory notifications as a single
    // AMF0 string, without an RPC transaction id. They carry no acceptance or
    // status authority; wait for the proper _result/onStatus afterwards.
    let values =
        amf0::decode(payload).map_err(|_| Fault("Unsupported upstream command encoding"))?;
    if values.len() == 1 && matches!(values[0].as_str(), Some("onFCPublish" | "onFCUnpublish")) {
        return Ok(None);
    }
    amf0::command(message)
        .map(Some)
        .map_err(|_| Fault("Unsupported upstream command encoding"))
}

impl Upstream {
    async fn controls(&mut self, message: &Message) -> EngineResult<()> {
        match message.type_id {
            rtmp::WINDOW_ACK_SIZE => {
                if message.payload.len() != 4 {
                    return Err(Fault("Invalid upstream acknowledgement window"));
                }
                self.ack_window = u32::from_be_bytes(
                    message.payload[..]
                        .try_into()
                        .map_err(|_| Fault("Invalid acknowledgement window"))?,
                );
            }
            rtmp::USER_CONTROL => reply_ping(&mut self.writer, message).await?,
            rtmp::COMMAND_AMF0 | rtmp::COMMAND_AMF3 => {
                let Some(command) = upstream_command(message)? else {
                    return Ok(());
                };
                if command.name == "_error" {
                    return Err(Fault("Upstream rejected an RTMP command"));
                }
                if command.name == "onStatus" {
                    if let Some(object) = command.arguments.get(1) {
                        let level = object.get("level").and_then(Value::as_str).unwrap_or("");
                        let code = object.get("code").and_then(Value::as_str).unwrap_or("");
                        if level == "error"
                            || code.ends_with("Failed")
                            || code.ends_with("BadName")
                            || code.ends_with("Rejected")
                            || code.ends_with("Stop")
                        {
                            return Err(Fault("Upstream publication was rejected or stopped"));
                        }
                    }
                }
            }
            _ => {}
        }
        let received = self.reader.total_bytes_read();
        if self.ack_window != 0
            && received.saturating_sub(self.last_ack) >= u64::from((self.ack_window / 2).max(1))
        {
            write(
                &mut self.writer,
                &rtmp::control(
                    rtmp::ACKNOWLEDGEMENT,
                    (received as u32).to_be_bytes().to_vec(),
                ),
            )
            .await?;
            self.last_ack = received;
        }
        Ok(())
    }
    async fn result(&mut self, transaction: f64) -> EngineResult<amf0::Command> {
        let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
        loop {
            let message = timeout_at(deadline, self.reader.read_message())
                .await
                .map_err(|_| Fault("Upstream command timed out"))?
                .map_err(|_| Fault("Upstream command read failed"))?;
            self.controls(&message).await?;
            if matches!(message.type_id, rtmp::COMMAND_AMF0 | rtmp::COMMAND_AMF3) {
                let Some(command) = upstream_command(&message)? else {
                    continue;
                };
                if command.name == "_result" && command.transaction_id == transaction {
                    return Ok(command);
                }
            }
        }
    }
}

async fn connect_upstream(route: &Route, key: &str) -> EngineResult<Upstream> {
    let url =
        url::Url::parse(&route.upstream_url).map_err(|_| Fault("Invalid upstream server URL"))?;
    let host = url
        .host_str()
        .ok_or(Fault("Missing upstream hostname"))?
        .to_owned();
    let tls = match url.scheme() {
        "rtmp" => false,
        "rtmps" => true,
        _ => return Err(Fault("Unsupported upstream transport")),
    };
    let port = url.port().unwrap_or(if tls { 443 } else { 1935 });
    let socket = timeout(HANDSHAKE_TIMEOUT, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| Fault("Upstream connection timed out"))?
        .map_err(|_| Fault("Upstream connection failed"))?;
    let _ = socket.set_nodelay(true);
    let mut wire: Wire = if tls {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| Fault("TLS configuration failed"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let server_name = rustls::pki_types::ServerName::try_from(host)
            .map_err(|_| Fault("Invalid TLS hostname"))?;
        let stream = timeout(
            HANDSHAKE_TIMEOUT,
            tokio_rustls::TlsConnector::from(Arc::new(config)).connect(server_name, socket),
        )
        .await
        .map_err(|_| Fault("TLS handshake timed out"))?
        .map_err(|_| Fault("TLS certificate validation or handshake failed"))?;
        Box::new(stream)
    } else {
        Box::new(socket)
    };
    timeout(HANDSHAKE_TIMEOUT, rtmp::client_handshake(&mut wire))
        .await
        .map_err(|_| Fault("Upstream handshake timed out"))?
        .map_err(|_| Fault("Upstream RTMP handshake failed"))?;
    let (read, write_half) = tokio::io::split(wire);
    let mut upstream = Upstream {
        reader: ChunkReader::new(read),
        writer: ChunkWriter::new(write_half),
        stream_id: 1,
        ack_window: ACK_WINDOW,
        last_ack: 0,
    };
    let app = url.path().trim_matches('/');
    let connect = rtmp::command_message(
        "connect",
        1.0,
        0,
        vec![Value::object([
            ("app", Value::String(app.into())),
            (
                "tcUrl",
                Value::String(route.upstream_url.trim_end_matches('/').into()),
            ),
            (
                "flashVer",
                Value::String("FMLE/3.0 (compatible; Open-Delay)".into()),
            ),
            ("fpad", Value::Boolean(false)),
            ("capabilities", Value::Number(15.0)),
            ("audioCodecs", Value::Number(4071.0)),
            ("videoCodecs", Value::Number(252.0)),
            ("videoFunction", Value::Number(1.0)),
            ("objectEncoding", Value::Number(0.0)),
        ])],
    )
    .map_err(|_| Fault("Upstream connect encoding failed"))?;
    // Keep the initial 128-byte framing here: long connect commands must work.
    write(&mut upstream.writer, &connect).await?;
    upstream.result(1.0).await?;
    write(
        &mut upstream.writer,
        &rtmp::control(rtmp::SET_CHUNK_SIZE, 4096u32.to_be_bytes().to_vec()),
    )
    .await?;
    for (name, transaction, args) in [
        (
            "releaseStream",
            2.0,
            vec![Value::Null, Value::String(key.into())],
        ),
        (
            "FCPublish",
            3.0,
            vec![Value::Null, Value::String(key.into())],
        ),
        ("createStream", 4.0, vec![Value::Null]),
    ] {
        write(
            &mut upstream.writer,
            &rtmp::command_message(name, transaction, 0, args)
                .map_err(|_| Fault("Upstream command encoding failed"))?,
        )
        .await?;
    }
    let result = upstream.result(4.0).await?;
    let stream_id = result
        .arguments
        .get(1)
        .and_then(Value::as_number)
        .filter(|v| v.is_finite() && *v >= 1.0 && *v <= u32::MAX as f64 && v.fract() == 0.0)
        .ok_or(Fault("Invalid upstream message stream id"))? as u32;
    upstream.stream_id = stream_id;
    write(
        &mut upstream.writer,
        &rtmp::command_message(
            "publish",
            5.0,
            stream_id,
            vec![
                Value::Null,
                Value::String(key.into()),
                Value::String("live".into()),
            ],
        )
        .map_err(|_| Fault("Upstream publish encoding failed"))?,
    )
    .await?;
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    loop {
        let message = timeout_at(deadline, upstream.reader.read_message())
            .await
            .map_err(|_| Fault("Upstream publish timed out"))?
            .map_err(|_| Fault("Upstream publish read failed"))?;
        upstream.controls(&message).await?;
        if matches!(message.type_id, rtmp::COMMAND_AMF0 | rtmp::COMMAND_AMF3) {
            let Some(command) = upstream_command(&message)? else {
                continue;
            };
            if command.name == "onStatus"
                && command
                    .arguments
                    .get(1)
                    .and_then(|v| v.get("code"))
                    .and_then(Value::as_str)
                    == Some("NetStream.Publish.Start")
            {
                break;
            }
        }
    }
    Ok(upstream)
}

enum Pending {
    Initial,
    Seek {
        target: u64,
        revision: u64,
        requested_delay: u64,
    },
    Live {
        after: u64,
        revision: u64,
        started: Instant,
    },
    Reconnect {
        requested_delay: u64,
        after: u64,
        started: Instant,
    },
}
struct Playback {
    cursor: Option<u64>,
    pending: Option<Pending>,
    anchor_source: u64,
    anchor_output: u64,
    anchor_wall: Instant,
    last_output: u64,
    last_by_type: HashMap<u8, u64>,
    requested_delay: u64,
    last_output_source: Option<u64>,
    unapplied_revision: Option<u64>,
}
impl Playback {
    fn new() -> Self {
        Self {
            cursor: None,
            pending: Some(Pending::Initial),
            anchor_source: 0,
            anchor_output: 0,
            anchor_wall: Instant::now(),
            last_output: 0,
            last_by_type: HashMap::new(),
            requested_delay: 0,
            last_output_source: None,
            unapplied_revision: None,
        }
    }
    fn command(&mut self, action: TimelineAction, latest: u64) -> EngineResult<()> {
        match action.kind {
            TimelineActionKind::Seek {
                target_source_ms,
                requested_delay_ms,
            } => {
                self.pending = Some(Pending::Seek {
                    target: target_source_ms,
                    revision: action.revision,
                    requested_delay: requested_delay_ms,
                });
            }
            TimelineActionKind::Live => {
                self.pending = Some(Pending::Live {
                    after: latest,
                    revision: action.revision,
                    started: Instant::now(),
                });
                self.requested_delay = 0;
            }
            TimelineActionKind::Stop => {
                return Err(Fault(
                    "Delay protection can no longer be sustained; output stopped",
                ))
            }
        }
        Ok(())
    }
    fn next_anchor(&self, cadence: &HashMap<u8, TrackCadence>) -> u64 {
        if self.cursor.is_none() {
            return 0;
        }
        let mut anchor = self.last_output.saturating_add(1);
        for kind in [rtmp::VIDEO, rtmp::AUDIO] {
            if let Some(last) = self.last_by_type.get(&kind) {
                let spacing = cadence.get(&kind).map(TrackCadence::spacing).unwrap_or(1);
                anchor = anchor.max(last.saturating_add(spacing));
            }
        }
        anchor
    }
}

#[allow(clippy::too_many_arguments)]
async fn playback(
    service: Service,
    route: Route,
    generation: u64,
    key: String,
    media: Arc<Mutex<SessionMedia>>,
    notify: Arc<Notify>,
    mut commands: mpsc::Receiver<TimelineAction>,
    cancel: CancellationToken,
) -> EngineResult<()> {
    let mut state = Playback::new();
    let mut retry = 0usize;
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let connection = tokio::select! {_ = cancel.cancelled()=>return Ok(()),result=connect_upstream(&route,&key)=>result};
        let mut upstream = match connection {
            Ok(upstream) => {
                if let Ok(mut controller) = service.controller.lock() {
                    let _ = controller.set_error(&route.id, generation, None, false);
                }
                service.publish();
                upstream
            }
            Err(fault) => {
                recovering(&service, &route.id, generation, fault.0);
                wait_retry(&cancel, &mut retry).await?;
                continue;
            }
        };
        if state.cursor.is_some() {
            let latest = media
                .lock()
                .map_err(|_| Fault("Media state unavailable"))?
                .latest_source;
            state.pending = Some(match state.pending.take() {
                Some(Pending::Seek {
                    revision,
                    requested_delay,
                    ..
                }) => Pending::Seek {
                    target: latest.saturating_sub(requested_delay),
                    revision,
                    requested_delay,
                },
                Some(Pending::Live { revision, .. }) => Pending::Live {
                    after: latest,
                    revision,
                    started: Instant::now(),
                },
                _ => match state.unapplied_revision {
                    Some(revision) if state.requested_delay > 0 => Pending::Seek {
                        target: latest.saturating_sub(state.requested_delay),
                        revision,
                        requested_delay: state.requested_delay,
                    },
                    Some(revision) => Pending::Live {
                        after: latest,
                        revision,
                        started: Instant::now(),
                    },
                    None => Pending::Reconnect {
                        requested_delay: state.requested_delay,
                        after: latest,
                        started: Instant::now(),
                    },
                },
            });
        }
        let connected_at = Instant::now();
        // If a transport write is stalled, cancellation drops the whole
        // connection rather than resuming a partially written RTMP message.
        let result = tokio::select! {
            biased;
            _=cancel.cancelled()=>return Ok(()),
            result=play_connected(&service,&route,generation,&media,&notify,&mut commands,&cancel,&mut upstream,&mut state)=>result,
        };
        if cancel.is_cancelled() {
            return Ok(());
        }
        match result {
            Ok(()) => return Ok(()),
            Err(PlaybackFailure::Terminal(fault)) => return Err(fault),
            Err(PlaybackFailure::Transport) => {
                if connected_at.elapsed() >= Duration::from_secs(30) {
                    retry = 0;
                }
                recovering(&service, &route.id, generation, "Upstream transport failed");
                // Previously sent source stays selected across retries. A fresh
                // keyframe is selected only at the explicitly requested delay.
                wait_retry(&cancel, &mut retry).await?;
            }
        }
    }
}

fn recovering(service: &Service, id: &str, generation: u64, detail: &'static str) {
    service.record("warning", &format!("Route {id}: {detail}; reconnecting"));
    if let Ok(mut controller) = service.controller.lock() {
        let _ = controller.set_error(
            id,
            generation,
            Some(format!(
                "{detail}; reconnecting with delay protection retained"
            )),
            true,
        );
    }
    service.publish();
}
async fn wait_retry(cancel: &CancellationToken, retry: &mut usize) -> EngineResult<()> {
    let seconds = RETRY_SECONDS[(*retry).min(RETRY_SECONDS.len() - 1)];
    *retry = retry.saturating_add(1);
    tokio::select! {_=cancel.cancelled()=>Ok(()),_=tokio::time::sleep(Duration::from_secs(seconds))=>Ok(())}
}

enum PlaybackFailure {
    Terminal(Fault),
    Transport,
}
impl From<Fault> for PlaybackFailure {
    fn from(fault: Fault) -> Self {
        Self::Terminal(fault)
    }
}

#[allow(clippy::too_many_arguments)]
async fn play_connected(
    service: &Service,
    route: &Route,
    generation: u64,
    media: &Arc<Mutex<SessionMedia>>,
    notify: &Arc<Notify>,
    commands: &mut mpsc::Receiver<TimelineAction>,
    cancel: &CancellationToken,
    upstream: &mut Upstream,
    state: &mut Playback,
) -> std::result::Result<(), PlaybackFailure> {
    let mut tick = tokio::time::interval(Duration::from_millis(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            _=cancel.cancelled()=>return Ok(()),
            action=commands.recv()=>{
                if let Some(action)=action {if action.generation==generation {
                    let latest=media.lock().map_err(|_|Fault("Media state unavailable"))?.latest_source;
                    state.command(action,latest)?;
                }} else{return Ok(());}
            }
            message=upstream.reader.read_message()=>{
                let message=message.map_err(|_|PlaybackFailure::Transport)?;
                upstream.controls(&message).await.map_err(|_|PlaybackFailure::Transport)?;
            }
            _=notify.notified()=>{},
            _=tick.tick()=>{},
        }
        // Coalesce pending commands to the latest controller revision before
        // choosing a frame; obsolete seeks never overwrite a newer live command.
        while let Ok(action) = commands.try_recv() {
            if action.generation == generation {
                let latest = media
                    .lock()
                    .map_err(|_| Fault("Media state unavailable"))?
                    .latest_source;
                state.command(action, latest)?;
            }
        }
        let latest = media
            .lock()
            .map_err(|_| Fault("Media state unavailable"))?
            .latest_source;
        let (selected, reanchor) = select_packet(service, &route.id, generation, state, latest)?;
        let Some(packet) = selected else {
            continue;
        };
        let apply = if reanchor {
            let pending = state
                .pending
                .take()
                .ok_or(Fault("Missing playback transition"))?;
            let revision = match pending {
                Pending::Seek {
                    revision,
                    requested_delay,
                    ..
                } => {
                    state.requested_delay = requested_delay;
                    Some(revision)
                }
                Pending::Live { revision, .. } => {
                    state.requested_delay = 0;
                    Some(revision)
                }
                Pending::Reconnect {
                    requested_delay, ..
                } => {
                    state.requested_delay = requested_delay;
                    None
                }
                Pending::Initial => None,
            };
            state.unapplied_revision = revision;
            state.anchor_source = packet.timestamp_ms;
            state.anchor_output = state.next_anchor(
                &media
                    .lock()
                    .map_err(|_| Fault("Media state unavailable"))?
                    .cadence,
            );
            state.anchor_wall = Instant::now();
            let configs = {
                let shared = media.lock().map_err(|_| Fault("Media state unavailable"))?;
                if !shared.configs.contains_key(&rtmp::VIDEO) {
                    return Err(Fault("Missing AVC decoder configuration").into());
                }
                [rtmp::DATA_AMF0, rtmp::DATA_AMF3, rtmp::AUDIO, rtmp::VIDEO]
                    .into_iter()
                    .filter_map(|kind| shared.configs.get(&kind).cloned())
                    .collect::<Vec<_>>()
            };
            for mut context in configs {
                context.timestamp = state.anchor_output as u32;
                context.stream_id = upstream.stream_id;
                write(&mut upstream.writer, &context)
                    .await
                    .map_err(|_| PlaybackFailure::Transport)?;
            }
            revision
        } else {
            None
        };
        let delta = packet.timestamp_ms.saturating_sub(state.anchor_source);
        let deadline = state.anchor_wall + Duration::from_millis(delta);
        if !reanchor && deadline > Instant::now() {
            continue;
        }
        let stored = service
            .history
            .lock()
            .map_err(|_| Fault("History unavailable"))?
            .read(packet.sequence)
            .map_err(|_| Fault("Selected media expired; delay protection stopped"))?;
        let timestamp = state
            .anchor_output
            .saturating_add(delta)
            .max(*state.last_by_type.get(&packet.type_id).unwrap_or(&0));
        let message = Message {
            csid: packet.csid,
            timestamp: timestamp as u32,
            type_id: packet.type_id,
            stream_id: upstream.stream_id,
            payload: Bytes::copy_from_slice(&stored.payload),
        };
        write(&mut upstream.writer, &message)
            .await
            .map_err(|_| PlaybackFailure::Transport)?;
        state.cursor = Some(packet.sequence);
        state.last_output = state.last_output.max(timestamp);
        state.last_by_type.insert(packet.type_id, timestamp);
        if matches!(packet.type_id, rtmp::AUDIO | rtmp::VIDEO) && !packet.configuration {
            state.last_output_source = Some(packet.timestamp_ms);
        }
        if let Ok(mut shared) = media.lock() {
            shared.bytes_sent = shared
                .bytes_sent
                .saturating_add(message.payload.len() as u64);
            shared.output_source = state.last_output_source;
            shared.output_sequence = state.cursor;
        }
        if let Some(revision) = apply {
            let applied = service
                .controller
                .lock()
                .map_err(|_| Fault("Controller unavailable"))?
                .applied(&route.id, generation, revision, packet.timestamp_ms);
            // A command accepted while the write was in flight may supersede
            // this frame; never publish the obsolete revision as applied.
            if applied.is_ok() {
                state.unapplied_revision = None;
                service.publish();
            }
        }
    }
}

fn select_packet(
    service: &Service,
    id: &str,
    generation: u64,
    state: &Playback,
    latest: u64,
) -> EngineResult<(Option<PacketMeta>, bool)> {
    let history = service
        .history
        .lock()
        .map_err(|_| Fault("History unavailable"))?;
    if let Some(pending) = &state.pending {
        let selected = match pending {
            Pending::Initial => history.first_keyframe_after(id, generation, 0),
            Pending::Seek { target, .. } => {
                Some(history.seek_keyframe(id, generation, *target).ok_or(Fault(
                    "No retained keyframe can sustain the requested delay",
                ))?)
            }
            Pending::Live { after, started, .. } => {
                let keyframe = history.first_keyframe_after(id, generation, *after);
                if keyframe.is_none() && started.elapsed() > Duration::from_secs(15) {
                    return Err(Fault("No fresh keyframe for live return within 15 seconds"));
                }
                keyframe
            }
            Pending::Reconnect {
                requested_delay,
                after,
                started,
            } => {
                if *requested_delay == 0 {
                    let keyframe = history.first_keyframe_after(id, generation, *after);
                    if keyframe.is_none() && started.elapsed() > Duration::from_secs(15) {
                        return Err(Fault("No fresh keyframe after upstream reconnect"));
                    }
                    keyframe
                } else {
                    Some(history.seek_keyframe(id,generation,latest.saturating_sub(*requested_delay)).ok_or(Fault("Retained history cannot protect the requested delay after reconnect"))?)
                }
            }
        };
        if selected.is_some() {
            return Ok((selected, true));
        }
        // Continue the current segment while waiting for a future live keyframe.
        // Initial/reconnect sessions have no valid output context yet.
        if !matches!(pending, Pending::Live { .. }) {
            return Ok((None, false));
        }
    }
    let Some(cursor) = state.cursor else {
        return Ok((None, false));
    };
    if let Some(oldest) = history.oldest(id, generation) {
        if cursor < oldest.sequence {
            return Err(Fault("Playback cursor expired; delay protection stopped"));
        }
    }
    Ok((history.next(id, generation, cursor), false))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_refuses_a_publisher_waiting_for_admission_without_mutating_media_state() {
        let controller = Arc::new(Mutex::new(
            open_delay_core::Controller::new(Config::initial()).unwrap(),
        ));
        let shutdown = CancellationToken::new();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let guard = controller.lock().unwrap();
        let before = serde_json::to_value(guard.snapshot()).unwrap();
        let pending = {
            let controller = controller.clone();
            let shutdown = shutdown.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                admit_route(&controller, &shutdown, "twitch")
            })
        };
        barrier.wait();
        // This is the same critical section as the updater's final idle check.
        assert!(!guard
            .snapshot()
            .routes
            .iter()
            .any(|route| route.is_connected));
        shutdown.cancel();
        drop(guard);
        assert!(matches!(
            pending.join().unwrap(),
            Err(Fault("Local engine is stopping"))
        ));
        let after = serde_json::to_value(controller.lock().unwrap().snapshot()).unwrap();
        assert_eq!(after, before);
    }
    #[tokio::test]
    async fn a_different_publish_name_cannot_supersede_an_active_route() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = Config::initial();
        let route = config
            .routes
            .iter_mut()
            .find(|route| route.id == "custom")
            .unwrap();
        route.enabled = true;
        route.upstream_url = "rtmp://127.0.0.1:1/test".into();
        let mut controller = open_delay_core::Controller::new(config.clone()).unwrap();
        let generation = controller.connect("custom").unwrap();
        let (events, _) = tokio::sync::broadcast::channel(8);
        let (wake, _) = tokio::sync::watch::channel(0);
        let service = Service {
            controller: Arc::new(Mutex::new(controller)),
            history: Arc::new(Mutex::new(
                DiskHistory::new(directory.path().join("history.ring"), 1024 * 1024, 1024).unwrap(),
            )),
            token: Arc::new("local-test".into()),
            config_path: Arc::new(directory.path().join("config.json")),
            ui_dir: Arc::new(directory.path().into()),
            events,
            wake,
            shutdown: CancellationToken::new(),
            api_port: 1,
            rtmp_port: 0,
            overlay: Arc::new(Mutex::new(
                crate::overlay::OverlayControl::load(directory.path().join("overlay.json"))
                    .unwrap(),
            )),
            diagnostics: Arc::new(Mutex::new(std::collections::VecDeque::new())),
        };
        let original_cancel = CancellationToken::new();
        let (commands, _) = mpsc::channel(1);
        let digest: [u8; 32] = Sha256::digest(b"original-publish-name").into();
        let registry = Arc::new(Mutex::new(HashMap::from([(
            "custom".into(),
            SessionHandle {
                generation,
                commands,
                cancel: original_cancel.clone(),
                publish_digest: digest,
            },
        )])));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_service = service.clone();
        let sessions = registry.clone();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            publisher(socket, server_service, sessions).await
        });
        let mut socket = TcpStream::connect(address).await.unwrap();
        rtmp::client_handshake(&mut socket).await.unwrap();
        let (read, write_half) = socket.into_split();
        let mut reader = ChunkReader::new(read);
        let mut writer = ChunkWriter::new(write_half);
        write(
            &mut writer,
            &rtmp::command_message(
                "connect",
                1.0,
                0,
                vec![Value::object([("app", Value::String("custom".into()))])],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        loop {
            let message = timeout(Duration::from_secs(2), reader.read_message())
                .await
                .unwrap()
                .unwrap();
            if message.type_id == rtmp::COMMAND_AMF0
                && amf0::command(&message).unwrap().name == "_result"
            {
                break;
            }
        }
        write(
            &mut writer,
            &rtmp::command_message("createStream", 2.0, 0, vec![Value::Null]).unwrap(),
        )
        .await
        .unwrap();
        loop {
            let message = timeout(Duration::from_secs(2), reader.read_message())
                .await
                .unwrap()
                .unwrap();
            if message.type_id == rtmp::COMMAND_AMF0
                && amf0::command(&message).unwrap().transaction_id == 2.0
            {
                break;
            }
        }
        write(
            &mut writer,
            &rtmp::command_message(
                "publish",
                0.0,
                1,
                vec![
                    Value::Null,
                    Value::String("different-publish-name".into()),
                    Value::String("live".into()),
                ],
            )
            .unwrap(),
        )
        .await
        .unwrap();
        let message = timeout(Duration::from_secs(2), reader.read_message())
            .await
            .unwrap()
            .unwrap();
        let command = amf0::command(&message).unwrap();
        assert_eq!(
            command
                .arguments
                .get(1)
                .and_then(|value| value.get("code"))
                .and_then(Value::as_str),
            Some("NetStream.Publish.BadName")
        );
        assert!(timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        let status = service.controller.lock().unwrap().snapshot();
        let route = status
            .routes
            .iter()
            .find(|route| route.id == "custom")
            .unwrap();
        assert_eq!(route.generation, generation);
        assert!(route.is_connected);
        assert!(!original_cancel.is_cancelled());
        assert!(same_publish_name(
            &registry.lock().unwrap()["custom"].publish_digest,
            &digest
        ));
        assert!(!same_publish_name(
            &digest,
            &Sha256::digest(b"different-publish-name").into()
        ));
    }
    #[test]
    fn segment_anchor_preserves_whole_frame_spacing_and_shared_av_clock() {
        let mut state = Playback::new();
        state.cursor = Some(100);
        state.last_output = 7990;
        state.last_by_type.insert(rtmp::VIDEO, 7980);
        state.last_by_type.insert(rtmp::AUDIO, 7990);
        let mut video = TrackCadence::default();
        for source in [0, 33, 67, 100, 133, 167] {
            video.observe(source);
        }
        let mut audio = TrackCadence::default();
        for source in [0, 21, 43, 64, 85, 107] {
            audio.observe(source);
        }
        let cadence = HashMap::from([(rtmp::VIDEO, video), (rtmp::AUDIO, audio)]);
        let next = state.next_anchor(&cadence);
        assert_eq!(next, 8014);
        assert!(next - state.last_by_type[&rtmp::VIDEO] >= 34);
        assert!(next - state.last_by_type[&rtmp::AUDIO] >= 22);
        // Unlike last_output+1, both video frames map to distinct 30fps ticks.
        assert!((next as f64 * 30.0 / 1000.0).round() > (7980.0_f64 * 30.0 / 1000.0).round());
        assert_eq!(Playback::new().next_anchor(&cadence), 0);
    }
    #[test]
    fn timestamp_wrap_and_audio_video_interleave_preserve_source_clock() {
        let mut clock = SourceClock::default();
        assert_eq!(clock.unwrap(0xfffffff0).unwrap(), 0xfffffff0);
        assert_eq!(clock.unwrap(5).unwrap(), (1u64 << 32) + 5);
        assert_eq!(clock.unwrap(3).unwrap(), (1u64 << 32) + 3);
        assert_eq!(clock.unwrap(0xfffffff5).unwrap(), 0xfffffff5);
        assert_eq!(clock.unwrap(20).unwrap(), (1u64 << 32) + 20);
    }
    #[test]
    fn unsupported_codec_and_enhanced_headers_are_explicit_errors() {
        let message = |kind, payload: Vec<u8>| Message {
            csid: 6,
            timestamp: 0,
            type_id: kind,
            stream_id: 1,
            payload: payload.into(),
        };
        assert!(classify(&message(rtmp::VIDEO, vec![0x1c, 1, 0, 0, 0])).is_err());
        assert!(classify(&message(rtmp::VIDEO, vec![0x91, 0, 0, 0, 0])).is_err());
        assert!(classify(&message(rtmp::AUDIO, vec![0x2f, 0])).is_err());
        assert_eq!(
            classify(&message(rtmp::VIDEO, vec![0x17, 1, 0, 0, 0])).unwrap(),
            (false, true)
        );
    }
    #[test]
    fn newer_live_action_replaces_queued_seek_without_implicit_toggle() {
        let mut state = Playback::new();
        state
            .command(
                TimelineAction {
                    route_id: "r".into(),
                    generation: 1,
                    revision: 1,
                    kind: TimelineActionKind::Seek {
                        target_source_ms: 1000,
                        requested_delay_ms: 15000,
                    },
                },
                16000,
            )
            .unwrap();
        state
            .command(
                TimelineAction {
                    route_id: "r".into(),
                    generation: 1,
                    revision: 2,
                    kind: TimelineActionKind::Live,
                },
                17000,
            )
            .unwrap();
        assert!(matches!(
            state.pending,
            Some(Pending::Live {
                after: 17000,
                revision: 2,
                ..
            })
        ));
    }
    #[tokio::test]
    async fn upstream_publish_negotiates_nondefault_stream_id_and_fragmented_connect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            rtmp::server_handshake(&mut stream).await.unwrap();
            let (read, write_half) = stream.into_split();
            let mut reader = ChunkReader::new(read);
            let mut writer = ChunkWriter::new(write_half);
            let connect = reader.read_message().await.unwrap();
            assert!(connect.payload.len() > 128);
            let command = amf0::command(&connect).unwrap();
            assert_eq!(command.name, "connect");
            assert_eq!(
                command.arguments[0].get("app").and_then(Value::as_str),
                Some("test/nested")
            );
            write(
                &mut writer,
                &rtmp::connect_result(command.transaction_id).unwrap(),
            )
            .await
            .unwrap();
            loop {
                let message = reader.read_message().await.unwrap();
                if !matches!(message.type_id, rtmp::COMMAND_AMF0 | rtmp::COMMAND_AMF3) {
                    continue;
                }
                let command = amf0::command(&message).unwrap();
                if command.name == "FCPublish" {
                    write(
                        &mut writer,
                        &Message {
                            csid: 3,
                            timestamp: 0,
                            type_id: rtmp::COMMAND_AMF0,
                            stream_id: 0,
                            payload: amf0::encode(&[Value::String("onFCPublish".into())])
                                .unwrap()
                                .into(),
                        },
                    )
                    .await
                    .unwrap();
                }
                if command.name == "createStream" {
                    write(
                        &mut writer,
                        &rtmp::create_stream_result(command.transaction_id, 7).unwrap(),
                    )
                    .await
                    .unwrap();
                }
                if command.name == "publish" {
                    assert_eq!(message.stream_id, 7);
                    assert_eq!(command.arguments[1].as_str(), Some("test-key?temporary=1"));
                    write(
                        &mut writer,
                        &rtmp::on_status(7, "NetStream.Publish.Start", "Ready").unwrap(),
                    )
                    .await
                    .unwrap();
                    break;
                }
            }
            let packet = reader.read_message().await.unwrap();
            assert_eq!(packet.stream_id, 7);
            assert_eq!(packet.payload[..], [0x17, 1, 0, 0, 0, 42]);
        });
        let route = Route {
            id: "test".into(),
            name: "Test".into(),
            platform: "custom".into(),
            local_app: "test".into(),
            upstream_url: format!("rtmp://127.0.0.1:{port}/test/nested"),
            enabled: true,
            include_in_group: true,
        };
        let mut upstream = connect_upstream(&route, "test-key?temporary=1")
            .await
            .unwrap();
        assert_eq!(upstream.stream_id, 7);
        write(
            &mut upstream.writer,
            &Message {
                csid: 6,
                timestamp: 0,
                type_id: rtmp::VIDEO,
                stream_id: upstream.stream_id,
                payload: vec![0x17, 1, 0, 0, 0, 42].into(),
            },
        )
        .await
        .unwrap();
        server.await.unwrap();
    }
}
