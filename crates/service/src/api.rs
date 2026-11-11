use crate::Service;
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::{Stream, StreamExt};
use open_delay_core::{Command, Preferences, Route};
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_stream::wrappers::BroadcastStream;
use tower_http::services::{ServeDir, ServeFile};

fn failure(code: StatusCode, message: impl ToString) -> Response {
    (
        code,
        Json(json!({"success":false,"message":message.to_string()})),
    )
        .into_response()
}

fn allowed_origin(origin: &str, port: u16) -> bool {
    origin == format!("http://127.0.0.1:{port}")
        || origin == format!("http://localhost:{port}")
        || matches!(
            origin,
            "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost"
        )
        || cfg!(debug_assertions)
            && matches!(origin, "http://localhost:5173" | "http://127.0.0.1:5173")
}

#[derive(Clone)]
struct Guard {
    service: Service,
    rate: Arc<Mutex<(Instant, u32)>>,
    public_rate: Arc<Mutex<(Instant, u32)>>,
}

async fn guard(
    State(state): State<Guard>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let port = state.service.api_port;
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return failure(StatusCode::FORBIDDEN, "Invalid local host");
    }
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    if origin.as_deref().is_some_and(|o| !allowed_origin(o, port)) {
        return failure(StatusCode::FORBIDDEN, "Origin is not permitted");
    }
    if request.uri().path().starts_with("/api/")
        && origin.is_none()
        && request
            .headers()
            .get("Sec-Fetch-Site")
            .and_then(|h| h.to_str().ok())
            == Some("cross-site")
    {
        return failure(
            StatusCode::FORBIDDEN,
            "Cross-site local API access is not permitted",
        );
    }
    if request.uri().path().starts_with("/api/") {
        let authenticated = request
            .headers()
            .get("X-OpenDelay-Token")
            .and_then(|h| h.to_str().ok())
            == Some(state.service.token.as_str());
        let bucket = if authenticated {
            &state.rate
        } else {
            &state.public_rate
        };
        let mut rate = bucket.lock().unwrap_or_else(|e| e.into_inner());
        if rate.0.elapsed() >= Duration::from_secs(1) {
            *rate = (Instant::now(), 0);
        }
        rate.1 += 1;
        if rate.1 > 120 {
            return failure(
                StatusCode::TOO_MANY_REQUESTS,
                "Local request limit exceeded",
            );
        }
    }
    let path = request.uri().path();
    let public = matches!(
        path,
        "/api/v1/session" | "/api/v1/overlay-status" | "/api/v1/health"
    );
    let same_origin_browser = request
        .headers()
        .get("Sec-Fetch-Site")
        .and_then(|h| h.to_str().ok())
        == Some("same-origin");
    if path == "/api/v1/session" && origin.is_none() && !same_origin_browser {
        return failure(
            StatusCode::FORBIDDEN,
            "Session requires an explicit local Origin",
        );
    }
    if path.starts_with("/api/") && !public && request.method() != Method::OPTIONS {
        let token = request
            .headers()
            .get("X-OpenDelay-Token")
            .and_then(|h| h.to_str().ok());
        if token != Some(state.service.token.as_str()) {
            return failure(StatusCode::UNAUTHORIZED, "Invalid session token");
        }
    }
    if request.method() == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        cors(&mut response, origin.as_deref());
        return response;
    }
    let mut response = next.run(request).await;
    cors(&mut response, origin.as_deref());
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    response
}

fn cors(response: &mut Response, origin: Option<&str>) {
    if let Some(origin) = origin.and_then(|o| HeaderValue::from_str(o).ok()) {
        response
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Origin"));
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET,POST,DELETE,OPTIONS"),
        );
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("Content-Type,X-OpenDelay-Token"),
        );
    }
}

pub fn router(service: Service) -> Router {
    let ui = service.ui_dir.as_ref();
    let assets = ServeDir::new(ui).not_found_service(ServeFile::new(ui.join("index.html")));
    let guard_state = Guard {
        service: service.clone(),
        rate: Arc::new(Mutex::new((Instant::now(), 0))),
        public_rate: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    Router::new()
        .route(
            "/api/v1/health",
            get(|| async {
                Json(json!({"name":"Open-Delay","version":env!("CARGO_PKG_VERSION")}))
            }),
        )
        .route("/api/v1/session", get(session))
        .route("/api/v1/status", get(status))
        .route("/api/v1/overlay-status", get(overlay_status))
        .route(
            "/api/v1/overlay-control",
            get(overlay_control).post(save_overlay_control),
        )
        .route("/api/v1/overlay-test", post(test_overlay))
        .route("/api/v1/events", get(events))
        .route("/api/v1/commands", post(command))
        .route("/api/v1/routes", get(routes).post(upsert_route))
        .route("/api/v1/routes/{id}", delete(remove_route))
        .route(
            "/api/v1/preferences",
            get(preferences).post(save_preferences),
        )
        .route("/api/v1/diagnostics", get(diagnostics))
        .route("/api/v1/shutdown", post(shutdown))
        .fallback_service(assets)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(guard_state, guard))
        .with_state(service)
}

async fn session(State(service): State<Service>) -> Json<Value> {
    Json(json!({"token": service.token.as_str()}))
}
async fn status(State(service): State<Service>) -> Response {
    match service.snapshot() {
        Some(snapshot) => Json(snapshot).into_response(),
        None => failure(StatusCode::INTERNAL_SERVER_ERROR, "Controller unavailable"),
    }
}
async fn shutdown(State(service): State<Service>) -> Response {
    service.shutdown.cancel();
    Json(json!({"success":true})).into_response()
}
#[derive(serde::Deserialize)]
struct OverlayQuery {
    viewer: Option<String>,
}
// Disk writes and every acquisition of the visual-control mutex stay off Tokio's
// worker threads, including readers waiting behind an in-progress atomic save.
async fn access_overlay<F>(
    service: &Service,
    work: F,
) -> Result<crate::overlay::ControlSnapshot, (StatusCode, &'static str)>
where
    F: FnOnce(
            &mut crate::overlay::OverlayControl,
        ) -> Result<crate::overlay::ControlSnapshot, (StatusCode, &'static str)>
        + Send
        + 'static,
{
    let overlay = service.overlay.clone();
    match tokio::task::spawn_blocking(move || {
        let mut control = overlay.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Overlay controls unavailable",
            )
        })?;
        work(&mut control)
    })
    .await
    {
        Ok(Ok(snapshot)) => Ok(snapshot),
        Ok(Err(error)) => Err(error),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Overlay operation interrupted",
        )),
    }
}
fn overlay_response(
    result: Result<crate::overlay::ControlSnapshot, (StatusCode, &'static str)>,
) -> Response {
    match result {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err((status, message)) => failure(status, message),
    }
}
async fn overlay_control(State(service): State<Service>) -> Response {
    overlay_response(access_overlay(&service, |control| Ok(control.snapshot())).await)
}
async fn save_overlay_control(
    State(service): State<Service>,
    Json(config): Json<crate::overlay::OverlayConfig>,
) -> Response {
    if let Err(error) = config.validate() {
        return failure(StatusCode::BAD_REQUEST, error);
    }
    overlay_response(
        access_overlay(&service, move |control| {
            control.save(config).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cannot save overlay settings; previous settings are still active",
                )
            })?;
            Ok(control.snapshot())
        })
        .await,
    )
}
async fn test_overlay(
    State(service): State<Service>,
    Json(request): Json<crate::overlay::TestRequest>,
) -> Response {
    overlay_response(
        access_overlay(&service, move |control| {
            control
                .test(request)
                .map_err(|error| (StatusCode::BAD_REQUEST, error))?;
            Ok(control.snapshot())
        })
        .await,
    )
}
async fn overlay_status(
    State(service): State<Service>,
    Query(query): Query<OverlayQuery>,
) -> Response {
    if query
        .viewer
        .as_ref()
        .is_some_and(|viewer| viewer.len() > 36 || uuid::Uuid::parse_str(viewer).is_err())
    {
        return failure(StatusCode::BAD_REQUEST, "Invalid source identifier");
    }
    let control = match access_overlay(&service, move |control| {
        if query.viewer.is_some() {
            control.heartbeat();
        }
        Ok(control.snapshot())
    })
    .await
    {
        Ok(control) => control,
        Err((status, message)) => return failure(status, message),
    };
    match service.controller.lock() {
        Ok(c) => {
            let snapshot = c.snapshot();
            let selected: Vec<_> = snapshot
                .routes
                .iter()
                .filter(|route| route.is_connected && route.include_in_group)
                .collect();
            let buffer = json!({"fill_ms":selected.iter().map(|route|route.buffer.fill_ms).min().unwrap_or(0),"target_ms":selected.iter().map(|route|route.buffer.target_ms).max().unwrap_or(0)});
            let value = serde_json::to_value(snapshot).unwrap_or(Value::Null);
            let keys = [
                "sequence",
                "mode",
                "effective_mode",
                "is_connected",
                "desired_delay_ms",
                "effective_delay_ms",
                "transition_in_progress",
                "buffer",
                "overlay_hidden",
                "routes",
            ];
            let mut result = serde_json::Map::new();
            for key in keys {
                if key != "routes" {
                    if let Some(v) = value.get(key) {
                        result.insert(key.into(), v.clone());
                    }
                }
            }
            result.insert("buffer".into(), buffer);
            if let Ok(Value::Object(fields)) = serde_json::to_value(control) {
                result.extend(fields);
            }
            Json(Value::Object(result)).into_response()
        }
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR, "Controller unavailable"),
    }
}
async fn command(State(service): State<Service>, Json(command): Json<Command>) -> Response {
    let result = match service.controller.lock() {
        Ok(mut c) => {
            let mut next = c.clone();
            let result = next.execute(command);
            if next.config() != c.config() {
                if let Err(error) = next.config().save(service.config_path.as_ref()) {
                    return failure(StatusCode::INTERNAL_SERVER_ERROR, error);
                }
            }
            *c = next;
            result
        }
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR, "Controller unavailable"),
    };
    service.record(
        if result.success { "info" } else { "warning" },
        if result.success {
            "Control command accepted"
        } else {
            "Control command refused; inspect the route status"
        },
    );
    service.publish();
    Json(result).into_response()
}
async fn routes(State(service): State<Service>) -> Response {
    match service.controller.lock() {
        Ok(c) => Json(json!({"routes": c.config().routes})).into_response(),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR, "Controller unavailable"),
    }
}
async fn upsert_route(State(service): State<Service>, Json(route): Json<Route>) -> Response {
    let result = (|| {
        let mut c = service
            .controller
            .lock()
            .map_err(|_| "Controller unavailable".to_owned())?;
        let mut next = c.clone();
        next.upsert_route(route).map_err(|e| e.to_string())?;
        next.config()
            .save(service.config_path.as_ref())
            .map_err(|e| e.to_string())?;
        *c = next;
        Ok::<_, String>(json!({"success":true,"routes":c.config().routes}))
    })();
    match result {
        Ok(value) => {
            service.publish();
            Json(value).into_response()
        }
        Err(e) => failure(StatusCode::CONFLICT, e),
    }
}
async fn remove_route(State(service): State<Service>, Path(id): Path<String>) -> Response {
    let result = (|| {
        let mut c = service
            .controller
            .lock()
            .map_err(|_| "Controller unavailable".to_owned())?;
        let mut next = c.clone();
        next.remove_route(&id).map_err(|e| e.to_string())?;
        next.config()
            .save(service.config_path.as_ref())
            .map_err(|e| e.to_string())?;
        *c = next;
        Ok::<_, String>(json!({"success":true,"routes":c.config().routes}))
    })();
    match result {
        Ok(value) => {
            service.publish();
            Json(value).into_response()
        }
        Err(e) => failure(StatusCode::CONFLICT, e),
    }
}
async fn preferences(State(service): State<Service>) -> Response {
    match service.controller.lock() {
        Ok(c) => Json(c.config().preferences.clone()).into_response(),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR, "Controller unavailable"),
    }
}
async fn save_preferences(
    State(service): State<Service>,
    Json(preferences): Json<Preferences>,
) -> Response {
    let result = (|| {
        let mut c = service
            .controller
            .lock()
            .map_err(|_| "Controller unavailable".to_owned())?;
        let old = c.config().preferences.clone();
        let restart = old.api_port != preferences.api_port
            || old.rtmp_port != preferences.rtmp_port
            || old.lan_ingest != preferences.lan_ingest
            || old.disk_bytes != preferences.disk_bytes
            || old.cache_bytes != preferences.cache_bytes;
        let mut next = c.clone();
        next.update_preferences(preferences)
            .map_err(|e| e.to_string())?;
        next.config()
            .save(service.config_path.as_ref())
            .map_err(|e| e.to_string())?;
        *c = next;
        Ok::<_, String>(
            json!({"success":true,"restart_required":restart,"preferences":c.config().preferences}),
        )
    })();
    match result {
        Ok(value) => {
            service.publish();
            Json(value).into_response()
        }
        Err(e) => failure(StatusCode::CONFLICT, e),
    }
}
async fn diagnostics(State(service): State<Service>) -> Response {
    match service.controller.lock() { Ok(c) => Json(json!({"version":env!("CARGO_PKG_VERSION"),"status":c.snapshot(),"capabilities":{"h264_aac":true,"enhanced_rtmp":false,"updates":"desktop_only"},"logs":service.diagnostics.lock().ok().map(|logs|logs.clone()).unwrap_or_default()})).into_response(),Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR,"Controller unavailable") }
}
async fn events(
    State(service): State<Service>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let subscription = service.events.subscribe();
    let initial = service
        .snapshot()
        .and_then(|s| serde_json::to_string(&s).ok())
        .unwrap_or_else(|| "{}".into());
    let first =
        futures_util::stream::once(
            async move { Ok(Event::default().event("status").data(initial)) },
        );
    let rest = BroadcastStream::new(subscription).filter_map(|event| async move {
        event
            .ok()
            .map(|data| Ok(Event::default().event("status").data(data)))
    });
    Sse::new(
        first
            .chain(rest)
            .take_until(service.shutdown.cancelled_owned()),
    )
    .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
