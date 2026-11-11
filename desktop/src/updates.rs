//! Signed updates stay native: the webview never supplies a package URL or bytes.
use crate::{DesktopState, ServiceInfo};
use open_delay_core::Config;
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    configured: bool,
    phase: &'static str,
    current_version: String,
    version: Option<String>,
    notes: Option<String>,
    downloaded: u64,
    total: Option<u64>,
    error: Option<String>,
}
struct Inner {
    snapshot: Snapshot,
    pending: Option<Update>,
}
pub struct UpdaterState {
    inner: Mutex<Inner>,
    operation: tokio::sync::Mutex<()>,
    installing: AtomicBool,
    settings: Option<tauri_plugin_updater::Config>,
}
impl UpdaterState {
    pub fn new(app: &tauri::AppHandle) -> Self {
        let settings = app
            .config()
            .plugins
            .0
            .get("updater")
            .cloned()
            .map(serde_json::from_value::<tauri_plugin_updater::Config>)
            .transpose();
        let (settings, error) = match settings {
            Ok(Some(settings)) if secure_config(&settings) => (Some(settings), None),
            Ok(None) => (None, None),
            _ => (
                None,
                Some(
                    "The updater configuration must use a public key and secure HTTPS endpoints"
                        .into(),
                ),
            ),
        };
        Self {
            inner: Mutex::new(Inner {
                snapshot: Snapshot {
                    configured: settings.is_some(),
                    phase: if error.is_some() { "error" } else { "idle" },
                    current_version: app.package_info().version.to_string(),
                    version: None,
                    notes: None,
                    downloaded: 0,
                    total: None,
                    error,
                },
                pending: None,
            }),
            operation: tokio::sync::Mutex::new(()),
            installing: AtomicBool::new(false),
            settings,
        }
    }
    pub fn is_installing(&self) -> bool {
        self.installing.load(Ordering::Acquire)
    }
    fn snapshot(&self) -> Result<Snapshot, String> {
        self.inner
            .lock()
            .map(|inner| inner.snapshot.clone())
            .map_err(|_| "Updater state unavailable".into())
    }
    fn change(
        &self,
        app: &tauri::AppHandle,
        edit: impl FnOnce(&mut Inner),
    ) -> Result<Snapshot, String> {
        let snapshot = {
            let mut inner = self.inner.lock().map_err(|_| "Updater state unavailable")?;
            edit(&mut inner);
            inner.snapshot.clone()
        };
        app.emit("update-status", &snapshot)
            .map_err(|e| e.to_string())?;
        Ok(snapshot)
    }
    fn failure(&self, app: &tauri::AppHandle, error: impl ToString) -> Result<Snapshot, String> {
        self.change(app, |inner| {
            inner.snapshot.phase = "error";
            inner.snapshot.error = Some(error.to_string());
        })
    }
}
fn secure_config(config: &tauri_plugin_updater::Config) -> bool {
    !config.pubkey.trim().is_empty()
        && !config.endpoints.is_empty()
        && config.endpoints.iter().all(|url| {
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
        && !config.dangerous_insecure_transport_protocol
        && !config.dangerous_accept_invalid_certs
        && !config.dangerous_accept_invalid_hostnames
        && !config.allow_downgrades
}
fn any_route_connected(service: &open_delay_service::Service) -> Result<bool, String> {
    let controller = service
        .controller
        .lock()
        .map_err(|_| "Controller unavailable")?;
    Ok(active_output(&controller.snapshot()))
}
fn active_output(status: &open_delay_core::Status) -> bool {
    status.routes.iter().any(|route| route.is_connected)
}
fn ensure_idle(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<DesktopState>();
    let running = state
        .running
        .lock()
        .map_err(|_| "Desktop state unavailable")?;
    if let Some(running) = running.as_ref() {
        if any_route_connected(&running.service)? {
            return Err("Stop every streaming output before installing an update".into());
        }
    }
    Ok(())
}
struct Installing<'a>(&'a AtomicBool);
impl Drop for Installing<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub fn update_status(app: tauri::AppHandle) -> Result<Snapshot, String> {
    app.state::<UpdaterState>().snapshot()
}
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let state = app.state::<UpdaterState>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "An update operation is already running")?;
    let Some(settings) = state.settings.as_ref() else {
        return state.failure(&app, "Updates are not configured in this build");
    };
    state.change(&app, |inner| {
        inner.pending = None;
        inner.snapshot.phase = "checking";
        inner.snapshot.error = None;
        inner.snapshot.version = None;
        inner.snapshot.notes = None;
        inner.snapshot.downloaded = 0;
        inner.snapshot.total = None;
    })?;
    let result = async {
        let updater = app
            .updater_builder()
            .pubkey(settings.pubkey.clone())
            .endpoints(settings.endpoints.clone())
            .map_err(|e| e.to_string())?
            .timeout(Duration::from_secs(30))
            .configure_client(|client| client.https_only(true))
            .build()
            .map_err(|e| e.to_string())?;
        updater.check().await.map_err(|e| e.to_string())
    }
    .await;
    match result {
        Ok(update) => {
            if update.as_ref().is_some_and(|update| {
                update.download_url.scheme() != "https"
                    || !update.download_url.username().is_empty()
                    || update.download_url.password().is_some()
            }) {
                return state.failure(
                    &app,
                    "The update package must use a secure HTTPS URL without credentials",
                );
            }
            state.change(&app, |inner| {
                inner.snapshot.phase = if update.is_some() {
                    "available"
                } else {
                    "up_to_date"
                };
                inner.snapshot.version = update.as_ref().map(|update| update.version.clone());
                inner.snapshot.notes = update.as_ref().and_then(|update| update.body.clone());
                inner.pending = update;
            })
        }
        Err(error) => state.failure(&app, error),
    }
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let state = app.state::<UpdaterState>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "An update operation is already running")?;
    if state.settings.is_none() {
        return state.failure(&app, "Updates are not configured in this build");
    }
    if let Err(error) = ensure_idle(&app) {
        return state.failure(&app, error);
    }
    let update = state
        .inner
        .lock()
        .map_err(|_| "Updater state unavailable")?
        .pending
        .clone();
    let Some(mut update) = update else {
        return state.failure(&app, "Check for an update before installing it");
    };
    update.timeout = Some(Duration::from_secs(300));
    state.installing.store(true, Ordering::Release);
    let _installing = Installing(&state.installing);
    state.change(&app, |inner| {
        inner.snapshot.phase = "downloading";
        inner.snapshot.error = None;
        inner.snapshot.downloaded = 0;
        inner.snapshot.total = None;
    })?;
    let mut downloaded = 0u64;
    let mut last_progress = Instant::now() - Duration::from_secs(1);
    let bytes = match update
        .download(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                if last_progress.elapsed() >= Duration::from_millis(100)
                    || total == Some(downloaded)
                {
                    let _ = state.change(&app, |inner| {
                        inner.snapshot.downloaded = downloaded;
                        inner.snapshot.total = total;
                    });
                    last_progress = Instant::now();
                }
            },
            || {},
        )
        .await
    {
        Ok(bytes) => bytes,
        Err(error) => return state.failure(&app, error),
    };
    // Download verifies the signature. Recheck every route after the wait, then
    // cancel admission under the same controller lock before draining the engine.
    let desktop = app.state::<DesktopState>();
    let _lifecycle = desktop.lifecycle.lock().await;
    let previous = {
        let mut guard = desktop
            .running
            .lock()
            .map_err(|_| "Desktop state unavailable")?;
        if let Some(running) = guard.as_ref() {
            let controller = running
                .service
                .controller
                .lock()
                .map_err(|_| "Controller unavailable")?;
            if active_output(&controller.snapshot()) {
                return state.failure(&app, "A streaming output started during download. Stop every output and retry the installation");
            }
            running.service.shutdown.cancel();
        }
        guard.take()
    };
    let recovery_ports = previous
        .as_ref()
        .map(|running| (running.api_addr.port(), running.rtmp_addr.port()));
    // The snapshot remains readable even if a webview event cannot be delivered.
    // Never abandon a stopped engine solely because a UI notification failed.
    if let Err(error) = state.change(&app, |inner| {
        inner.snapshot.phase = "installing";
        inner.snapshot.downloaded = bytes.len() as u64;
    }) {
        eprintln!("Cannot notify the update view: {error}");
    }
    if let Some(previous) = previous {
        previous.stop().await;
    }
    let result = tauri::async_runtime::spawn_blocking(move || update.install(bytes)).await;
    let error = match result {
        Ok(Ok(())) => {
            app.restart();
        }
        Ok(Err(error)) => error.to_string(),
        Err(error) => format!("Update installer interrupted: {error}"),
    };
    let recovery = if let Some(ports) = recovery_ports {
        recover_service(&app, ports).await.err()
    } else {
        None
    };
    state.failure(
        &app,
        match recovery {
            Some(recovery) => format!("{error}. Local engine recovery failed: {recovery}"),
            None => error,
        },
    )
}

async fn recover_service(app: &tauri::AppHandle, ports: (u16, u16)) -> Result<(), String> {
    let desktop = app.state::<DesktopState>();
    let result = async {
        let path = desktop.data.join("config.json");
        let mut config = tokio::task::spawn_blocking(move || Config::load(path))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        config.preferences.api_port = ports.0;
        config.preferences.rtmp_port = ports.1;
        open_delay_service::start(
            config,
            desktop.data.join("config.json"),
            desktop.data.clone(),
            desktop.web.clone(),
        )
        .await
        .map_err(|e| e.to_string())
    }
    .await;
    match result {
        Ok(running) => {
            let info = ServiceInfo {
                api_base: format!("http://127.0.0.1:{}", running.api_addr.port()),
                error: None,
                native_errors: desktop
                    .info
                    .lock()
                    .map_err(|_| "Desktop state unavailable")?
                    .native_errors
                    .clone(),
            };
            *desktop
                .running
                .lock()
                .map_err(|_| "Desktop state unavailable")? = Some(running);
            *desktop
                .info
                .lock()
                .map_err(|_| "Desktop state unavailable")? = info;
            Ok(())
        }
        Err(error) => {
            let mut info = desktop
                .info
                .lock()
                .map_err(|_| "Desktop state unavailable")?;
            info.api_base.clear();
            info.error = Some(format!(
                "The local engine could not restart after the failed update: {error}"
            ));
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_requires_secure_endpoints_and_no_bypasses() {
        let value = serde_json::json!({ "pubkey":"public", "endpoints":["https://github.com/Cori-bot/Open-Delay/releases/latest/download/latest.json"] });
        let config: tauri_plugin_updater::Config = serde_json::from_value(value.clone()).unwrap();
        assert!(secure_config(&config));
        for key in [
            "dangerousInsecureTransportProtocol",
            "dangerousAcceptInvalidCerts",
            "dangerousAcceptInvalidHostnames",
            "allowDowngrades",
        ] {
            let mut invalid = value.clone();
            invalid[key] = true.into();
            assert!(!secure_config(&serde_json::from_value(invalid).unwrap()));
        }
        let mut invalid = value;
        invalid["pubkey"] = "".into();
        assert!(!secure_config(&serde_json::from_value(invalid).unwrap()));
    }
    #[test]
    fn installation_flag_resets_after_a_failed_operation() {
        let flag = AtomicBool::new(true);
        {
            let _guard = Installing(&flag);
            assert!(flag.load(Ordering::Acquire));
        }
        assert!(!flag.load(Ordering::Acquire));
    }
    #[test]
    fn connected_outputs_outside_the_control_group_still_block_installation() {
        let mut config = Config::initial();
        config.routes[0].include_in_group = false;
        let mut controller = open_delay_core::Controller::new(config).unwrap();
        assert!(!active_output(&controller.snapshot()));
        let generation = controller.connect("twitch").unwrap();
        assert!(!controller.snapshot().routes[0].include_in_group);
        assert!(active_output(&controller.snapshot()));
        controller.disconnect("twitch", generation).unwrap();
        assert!(!active_output(&controller.snapshot()));
    }
}
