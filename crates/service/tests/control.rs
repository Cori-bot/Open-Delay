use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use open_delay_core::{Config, Controller, DiskHistory};
use open_delay_service::{api, start, Service};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, watch},
};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

fn local_service(dir: &std::path::Path) -> Service {
    let config = Config::initial();
    let (events, _) = broadcast::channel(8);
    let (wake, _) = watch::channel(0);
    Service {
        controller: Arc::new(Mutex::new(Controller::new(config).unwrap())),
        history: Arc::new(Mutex::new(
            DiskHistory::new(dir.join("test.ring"), 1024 * 1024, 4096).unwrap(),
        )),
        token: Arc::new("test-token".into()),
        config_path: Arc::new(dir.join("config.json")),
        ui_dir: Arc::new(dir.into()),
        events,
        wake,
        shutdown: CancellationToken::new(),
        api_port: 49155,
        rtmp_port: 49156,
        overlay: Arc::new(Mutex::new(
            open_delay_service::overlay::OverlayControl::load(dir.join("overlay.json")).unwrap(),
        )),
        diagnostics: Arc::new(Mutex::new(std::collections::VecDeque::new())),
    }
}
fn request(path: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .header("Host", "127.0.0.1:49155")
}

#[tokio::test]
async fn session_auth_origin_and_overlay_redaction() {
    let dir = tempfile::tempdir().unwrap();
    let s = local_service(dir.path());
    let router = api::router(s);
    for origin in [None, Some("https://evil.example")] {
        let mut req = request("/api/v1/session");
        if let Some(origin) = origin {
            req = req.header("Origin", origin);
        }
        assert_eq!(
            router
                .clone()
                .oneshot(req.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    let response = router
        .clone()
        .oneshot(
            request("/api/v1/session")
                .header("Sec-Fetch-Site", "same-origin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["token"], "test-token");
    let response = router
        .clone()
        .oneshot(
            request("/api/v1/session")
                .header("Origin", "http://tauri.localhost")
                .header("Sec-Fetch-Site", "cross-site")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Bundled Tauri window is a trusted explicit origin"
    );
    assert_eq!(
        response.headers()["Access-Control-Allow-Origin"],
        "http://tauri.localhost"
    );
    assert_eq!(
        router
            .clone()
            .oneshot(
                request("/api/v1/overlay-status")
                    .header("Sec-Fetch-Site", "cross-site")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        router
            .clone()
            .oneshot(request("/api/v1/routes").body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = router
        .oneshot(
            request("/api/v1/overlay-status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    for forbidden in ["hotkeys", "upstream_url", "token", "preferences", "routes"] {
        assert!(!text.contains(forbidden));
    }
}

#[tokio::test]
async fn unauthenticated_rate_limit_cannot_exhaust_control_commands() {
    let dir = tempfile::tempdir().unwrap();
    let s = local_service(dir.path());
    let router = api::router(s);
    for _ in 0..150 {
        let _ = router
            .clone()
            .oneshot(
                request("/api/v1/overlay-status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
    }
    let response = router
        .oneshot(
            request("/api/v1/routes")
                .header("X-OpenDelay-Token", "test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn failed_persistence_does_not_change_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = local_service(dir.path());
    let parent_file = dir.path().join("not-a-directory");
    std::fs::write(&parent_file, b"owned fixture").unwrap();
    s.config_path = Arc::new(parent_file.join("config.json"));
    let before = s.controller.lock().unwrap().config().clone();
    let mut route = before
        .routes
        .iter()
        .find(|r| r.id == "custom")
        .unwrap()
        .clone();
    route.name = "Changed".into();
    let router = api::router(s.clone());
    let response = router
        .oneshot(
            request("/api/v1/routes")
                .method("POST")
                .header("X-OpenDelay-Token", "test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&route).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!response.status().is_success());
    assert_eq!(s.controller.lock().unwrap().config(), &before);
}

#[tokio::test]
async fn ports_are_allocated_without_stealing_occupied_listeners_and_sse_stops() {
    let occupied_api = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let occupied_ingest = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = Config::initial();
    config.preferences.api_port = occupied_api.local_addr().unwrap().port();
    config.preferences.rtmp_port = occupied_ingest.local_addr().unwrap().port();
    config.preferences.disk_bytes = 1024 * 1024;
    let dir = tempfile::tempdir().unwrap();
    let running = start(
        config.clone(),
        dir.path().join("config.json"),
        dir.path().into(),
        PathBuf::from("nonexistent-ui"),
    )
    .await
    .unwrap();
    assert_ne!(running.api_addr.port(), config.preferences.api_port);
    assert_ne!(running.rtmp_addr.port(), config.preferences.rtmp_port);
    assert_ne!(running.api_addr.port(), running.rtmp_addr.port());
    let mut stream = TcpStream::connect(running.api_addr).await.unwrap();
    let req = format!(
        "GET /api/v1/events HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-OpenDelay-Token: {}\r\n\r\n",
        running.api_addr.port(),
        running.service.token
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut bytes = [0u8; 2048];
    let size = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes[..size]).contains("200 OK"));
    tokio::time::timeout(Duration::from_secs(3), running.stop())
        .await
        .expect("SSE must not keep shutdown alive");
    assert!(!dir.path().join("runtime.json").exists());
    assert!(TcpStream::connect(occupied_api.local_addr().unwrap())
        .await
        .is_ok());
    assert!(TcpStream::connect(occupied_ingest.local_addr().unwrap())
        .await
        .is_ok());
}

#[tokio::test]
async fn overlay_controls_are_authenticated_persistent_and_visual_only() {
    let dir = tempfile::tempdir().unwrap();
    let service = local_service(dir.path());
    let before = service.snapshot().unwrap();
    let router = api::router(service.clone());
    let config = serde_json::json!({"theme":"minimal","color":"purple","position":"top-right","badgeSize":"s","popupSize":"m","radius":64,"text":"","badge":true,"popup":true,"icon":true,"circleIcon":false,"iconMatchText":false,"hideLive":false,"instantLive":false,"liveGreen":false,"motion":true,"overrideBadgeText":false,"lang":"fr"});
    for path in ["/api/v1/overlay-control", "/api/v1/overlay-test"] {
        let response = router
            .clone()
            .oneshot(
                request(path)
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(config.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    for radius in [64, 65] {
        let mut valid = config.clone();
        valid["radius"] = serde_json::json!(radius);
        let response = router
            .clone()
            .oneshot(
                request("/api/v1/overlay-control")
                    .method("POST")
                    .header("X-OpenDelay-Token", "test-token")
                    .header("Content-Type", "application/json")
                    .body(Body::from(valid.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(value["config"], valid);
        let mut loaded =
            open_delay_service::overlay::OverlayControl::load(dir.path().join("overlay.json"))
                .unwrap();
        assert_eq!(
            serde_json::to_value(loaded.snapshot()).unwrap()["config"],
            valid
        );
    }
    let mut invalid = config.clone();
    invalid["radius"] = serde_json::json!(101);
    let response = router
        .clone()
        .oneshot(
            request("/api/v1/overlay-control")
                .method("POST")
                .header("X-OpenDelay-Token", "test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(invalid.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        service
            .overlay
            .lock()
            .unwrap()
            .snapshot()
            .config
            .unwrap()
            .radius,
        65
    );
    let response = router
        .clone()
        .oneshot(
            request("/api/v1/overlay-test")
                .method("POST")
                .header("X-OpenDelay-Token", "test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"state":"delayed","delay_ms":15000}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = router
        .clone()
        .oneshot(
            request("/api/v1/overlay-status?viewer=00000000-0000-4000-8000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["test"]["state"], "delayed");
    assert_eq!(value["source_recent"], true);
    assert!(value.get("routes").is_none());
    assert_eq!(
        service.snapshot().unwrap(),
        before,
        "visual tests never execute media commands"
    );
    let response = router
        .clone()
        .oneshot(
            request("/api/v1/overlay-test")
                .method("POST")
                .header("X-OpenDelay-Token", "test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"state":"stop"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(service.overlay.lock().unwrap().snapshot().test.is_none());
}
