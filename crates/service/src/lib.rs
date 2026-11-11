//! Local control service and independently implemented RTMP forwarding engine.
pub mod api;
pub mod engine;
pub mod overlay;

use fs2::FileExt;
use open_delay_core::{Config, Controller, DiskHistory};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{
    net::TcpListener,
    sync::{broadcast, watch},
};
use tokio_util::sync::CancellationToken;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, serde::Serialize)]
pub struct DiagnosticEntry {
    pub at: String,
    pub level: String,
    pub message: String,
}

#[derive(Clone)]
pub struct Service {
    pub controller: Arc<Mutex<Controller>>,
    pub history: Arc<Mutex<DiskHistory>>,
    pub token: Arc<String>,
    pub config_path: Arc<PathBuf>,
    pub ui_dir: Arc<PathBuf>,
    pub events: broadcast::Sender<String>,
    pub wake: watch::Sender<u64>,
    pub shutdown: CancellationToken,
    pub api_port: u16,
    pub rtmp_port: u16,
    pub overlay: Arc<Mutex<overlay::OverlayControl>>,
    pub diagnostics: Arc<Mutex<std::collections::VecDeque<DiagnosticEntry>>>,
}

pub struct RunningService {
    pub service: Service,
    pub api_addr: std::net::SocketAddr,
    pub rtmp_addr: std::net::SocketAddr,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    _instance_lock: std::fs::File,
    runtime_path: PathBuf,
}
impl RunningService {
    pub async fn stop(self) {
        self.service.shutdown.cancel();
        for task in self.tasks {
            let _ = task.await;
        }
        if let Err(error) = std::fs::remove_file(&self.runtime_path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Cannot remove local discovery file: {error}");
            }
        }
    }
}

impl Service {
    pub fn record(&self, level: &str, message: &str) {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            % 86400;
        let entry = DiagnosticEntry {
            at: format!(
                "{:02}:{:02}:{:02} UTC",
                seconds / 3600,
                (seconds / 60) % 60,
                seconds % 60
            ),
            level: level.chars().take(12).collect(),
            message: message.chars().take(512).collect(),
        };
        if let Ok(mut logs) = self.diagnostics.lock() {
            if logs.len() >= 256 {
                logs.pop_front();
            }
            logs.push_back(entry);
        }
    }
    pub fn publish(&self) {
        if let Some(snapshot) = self.snapshot() {
            if let Ok(json) = serde_json::to_string(&snapshot) {
                let _ = self.events.send(json);
            }
        }
        self.wake
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }
    pub fn snapshot(&self) -> Option<serde_json::Value> {
        let controller = self.controller.lock().ok()?;
        let mut value = serde_json::to_value(controller.snapshot()).ok()?;
        value["version"] = serde_json::json!(env!("CARGO_PKG_VERSION"));
        value["endpoints"] = serde_json::json!({"api_base":format!("http://127.0.0.1:{}",self.api_port),"rtmp_port":self.rtmp_port});
        Some(value)
    }
}

/// Bind both ports before starting work; a collision never opens another app's page.
pub async fn start(
    mut config: Config,
    config_path: PathBuf,
    data_dir: PathBuf,
    ui_dir: PathBuf,
) -> Result<RunningService, Error> {
    config.validate()?;
    std::fs::create_dir_all(&data_dir)?;
    let instance_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(data_dir.join("instance.lock"))?;
    instance_lock
        .try_lock_exclusive()
        .map_err(|_| "Open-Delay is already using this data directory")?;
    clean_abandoned_history(&data_dir)?;
    let api = bind_available(std::net::Ipv4Addr::LOCALHOST, config.preferences.api_port).await?;
    let ingest_ip = if config.preferences.lan_ingest {
        std::net::Ipv4Addr::UNSPECIFIED
    } else {
        std::net::Ipv4Addr::LOCALHOST
    };
    let rtmp = bind_available(ingest_ip, config.preferences.rtmp_port).await?;
    let api_addr = api.local_addr()?;
    let rtmp_addr = rtmp.local_addr()?;
    config.preferences.api_port = api_addr.port();
    config.preferences.rtmp_port = rtmp_addr.port();
    config.save(&config_path)?;
    let runtime_path = data_dir.join("runtime.json");
    std::fs::write(
        &runtime_path,
        serde_json::to_vec(
            &serde_json::json!({"name":"Open-Delay","api_base":format!("http://127.0.0.1:{}",api_addr.port()),"rtmp_port":rtmp_addr.port(),"pid":std::process::id()}),
        )?,
    )?;
    std::fs::create_dir_all(&data_dir)?;
    let history = engine::open_history(&data_dir, &config)?;
    let (events, _) = broadcast::channel(32);
    let (wake, _) = watch::channel(0);
    let service = Service {
        controller: Arc::new(Mutex::new(Controller::new(config)?)),
        history: Arc::new(Mutex::new(history)),
        token: Arc::new(uuid::Uuid::new_v4().to_string()),
        config_path: Arc::new(config_path),
        ui_dir: Arc::new(ui_dir),
        events,
        wake,
        shutdown: CancellationToken::new(),
        api_port: api_addr.port(),
        rtmp_port: rtmp_addr.port(),
        overlay: Arc::new(Mutex::new(overlay::OverlayControl::load(
            data_dir.join("overlay.json"),
        )?)),
        diagnostics: Arc::new(Mutex::new(std::collections::VecDeque::new())),
    };
    service.record(
        "info",
        "Local engine started; API and ingest ports allocated",
    );
    let api_service = service.clone();
    let api_shutdown = service.shutdown.clone();
    let api_task = tokio::spawn(async move {
        if let Err(error) = axum::serve(api, api::router(api_service))
            .with_graceful_shutdown(api_shutdown.cancelled_owned())
            .await
        {
            eprintln!("Control server stopped: {error}");
        }
    });
    let ingest_service = service.clone();
    let ingest_task = tokio::spawn(async move {
        engine::serve(rtmp, ingest_service).await;
    });
    Ok(RunningService {
        service,
        api_addr,
        rtmp_addr,
        tasks: vec![api_task, ingest_task],
        _instance_lock: instance_lock,
        runtime_path,
    })
}

async fn bind_available(ip: std::net::Ipv4Addr, preferred: u16) -> std::io::Result<TcpListener> {
    match TcpListener::bind((ip, preferred)).await {
        Ok(listener) => Ok(listener),
        Err(error)
            if preferred != 0
                && matches!(
                    error.kind(),
                    std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
                ) =>
        {
            TcpListener::bind((ip, 0)).await
        }
        Err(error) => Err(error),
    }
}

/// Only our strictly named ephemeral rings, while holding the data-directory lock.
fn clean_abandoned_history(directory: &std::path::Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(id) = name
            .strip_prefix("history-")
            .and_then(|n| n.strip_suffix(".ring"))
        else {
            continue;
        };
        if let Ok(uuid) = uuid::Uuid::parse_str(id) {
            if uuid.get_version_num() == 4 && uuid.to_string() == id {
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    Ok(())
}

pub fn default_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".local"))
        .join("Open-Delay")
}
