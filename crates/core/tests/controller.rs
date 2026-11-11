use open_delay_core::*;

fn controller(mode: Mode) -> Controller {
    let mut config = Config::initial();
    config.preferences.mode = mode;
    Controller::new(config).unwrap()
}
fn command(id: &str, action: Action) -> Command {
    Command {
        id: id.into(),
        action,
        route_ids: None,
    }
}

#[test]
fn overlay_countdown_uses_construction_progress_not_preexisting_history() {
    let mut c = controller(Mode::Overlay);
    let generation = c.connect("twitch").unwrap();
    progress(&mut c, generation, 60_000, 0);
    assert!(
        c.execute(command(
            "overlay-countdown",
            Action::SetDelay { delay_ms: 15_000 }
        ))
        .success
    );
    assert_eq!(c.snapshot().routes[0].buffer.fill_ms, 0);
    progress(&mut c, generation, 62_000, 0);
    assert_eq!(c.snapshot().routes[0].buffer.fill_ms, 2_000);
    assert!(!c.snapshot().routes[0].buffer.ready);
    c.set_error(
        "twitch",
        generation,
        Some("Upstream unavailable".into()),
        true,
    )
    .unwrap();
    assert_eq!(c.snapshot().effective_mode, "recovering");
}
fn progress(c: &mut Controller, g: u64, latest: u64, oldest: u64) {
    c.progress(
        "twitch",
        g,
        MediaProgress {
            latest_source_ms: latest,
            oldest_source_ms: oldest,
            bytes_received: 1000,
            bytes_sent: 1000,
            stored_bytes: 1000,
        },
    )
    .unwrap();
}

#[test]
fn replay_then_new_target_does_not_toggle_off() {
    let mut c = controller(Mode::Instant);
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 60_000, 0);
    assert!(
        c.execute(command("one", Action::SetDelay { delay_ms: 15_000 }))
            .success
    );
    let a = c.drain_actions().pop().unwrap();
    assert!(matches!(
        a.kind,
        TimelineActionKind::Seek {
            target_source_ms: 45_000,
            ..
        }
    ));
    c.applied("twitch", g, a.revision, 45_000).unwrap();
    assert!(
        c.execute(command("two", Action::SetDelay { delay_ms: 30_000 }))
            .success
    );
    let a = c.drain_actions().pop().unwrap();
    assert!(matches!(
        a.kind,
        TimelineActionKind::Seek {
            target_source_ms: 30_000,
            ..
        }
    ));
    c.applied("twitch", g, a.revision, 30_000).unwrap();
    assert_eq!(c.snapshot().effective_delay_ms, 30_000);
}

#[test]
fn go_live_interrupts_overlay_building_and_invalidates_stale_ack() {
    let mut c = controller(Mode::Overlay);
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 40_000, 0);
    c.execute(command("delay", Action::SetDelay { delay_ms: 15_000 }));
    assert!(c.drain_actions().is_empty());
    assert!(c.snapshot().transition_in_progress);
    progress(&mut c, g, 55_000, 0);
    let old = c.drain_actions().pop().unwrap();
    let result = c.execute(command("live", Action::GoLive));
    assert!(result.success);
    let live = c.drain_actions().pop().unwrap();
    assert!(matches!(live.kind, TimelineActionKind::Live));
    assert!(matches!(
        c.applied("twitch", g, old.revision, 40_000),
        Err(CoreError::StaleSession)
    ));
    c.applied("twitch", g, live.revision, 55_000).unwrap();
    assert!(!c.snapshot().transition_in_progress);
    assert_eq!(c.snapshot().effective_delay_ms, 0);
}

#[test]
fn disconnect_clears_pending_and_old_generation_cannot_mutate_reconnect() {
    let mut c = controller(Mode::Overlay);
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 1000, 0);
    c.execute(command("delay", Action::PrepareDelay { delay_ms: 15_000 }));
    c.disconnect("twitch", g).unwrap();
    assert!(!c.snapshot().transition_in_progress);
    assert_eq!(c.snapshot().desired_delay_ms, 0);
    let fresh = c.connect("twitch").unwrap();
    assert_ne!(fresh, g);
    assert!(matches!(
        c.progress("twitch", g, MediaProgress::default()),
        Err(CoreError::StaleSession)
    ));
    assert_eq!(
        c.snapshot()
            .routes
            .iter()
            .find(|r| r.id == "twitch")
            .unwrap()
            .generation,
        fresh
    );
}

#[test]
fn inactive_group_route_does_not_block_transition() {
    let mut c = controller(Mode::Instant);
    let mut custom = c
        .config()
        .routes
        .iter()
        .find(|r| r.id == "custom")
        .unwrap()
        .clone();
    custom.enabled = true;
    custom.upstream_url = "rtmp://127.0.0.1/test".into();
    c.save_route(custom).unwrap();
    let g = c.connect("custom").unwrap();
    c.progress(
        "custom",
        g,
        MediaProgress {
            latest_source_ms: 30_000,
            oldest_source_ms: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let result = c.execute(command("a", Action::SetDelay { delay_ms: 15_000 }));
    assert_eq!(result.results.len(), 1);
    let a = c.drain_actions().pop().unwrap();
    c.applied("custom", g, a.revision, 15_000).unwrap();
    assert!(!c.snapshot().transition_in_progress);
    assert_eq!(c.snapshot().effective_delay_ms, 15_000);
}

#[test]
fn idempotent_retry_does_not_double_toggle_and_mismatched_reuse_is_refused() {
    let mut c = controller(Mode::Instant);
    let cmd = command("toggle", Action::ToggleOverlay);
    let first = c.execute(cmd.clone());
    let second = c.execute(cmd);
    assert!(first.status.overlay_hidden);
    assert!(second.status.overlay_hidden);
    let refused = c.execute(command("toggle", Action::GoLive));
    assert!(!refused.success);
    assert_eq!(refused.reason.as_deref(), Some("command_id_conflict"));
}

#[test]
fn prepare_autoactivates_once_with_window_absent_and_zero_means_live() {
    let mut c = controller(Mode::Instant);
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 10_000, 0);
    c.execute(command(
        "prepare",
        Action::PrepareDelay { delay_ms: 30_000 },
    ));
    assert!(c.drain_actions().is_empty());
    progress(&mut c, g, 30_000, 0);
    assert_eq!(c.drain_actions().len(), 1);
    progress(&mut c, g, 31_000, 0);
    assert!(c.drain_actions().is_empty());
    c.execute(command("zero", Action::SetDelay { delay_ms: 0 }));
    assert!(matches!(
        c.drain_actions()[0].kind,
        TimelineActionKind::Live
    ));
}

#[test]
fn insufficient_shared_byte_budget_refuses_preparation() {
    let mut config = Config::initial();
    config.preferences.disk_bytes = 1024 * 1024;
    let mut c = Controller::new(config).unwrap();
    let g = c.connect("twitch").unwrap();
    c.progress(
        "twitch",
        g,
        MediaProgress {
            latest_source_ms: 1000,
            oldest_source_ms: 0,
            stored_bytes: 100_000,
            ..Default::default()
        },
    )
    .unwrap();
    let result = c.execute(command("big", Action::SetDelay { delay_ms: 60_000 }));
    assert!(!result.success);
    assert_eq!(result.results[0].reason.as_deref(), Some("byte_budget"));
    assert!(c.drain_actions().is_empty());
}

#[test]
fn stale_capacity_does_not_silently_resume_live() {
    let mut c = controller(Mode::Instant);
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 30_000, 0);
    c.execute(command("delay", Action::SetDelay { delay_ms: 15_000 }));
    let a = c.drain_actions().pop().unwrap();
    c.applied("twitch", g, a.revision, 15_000).unwrap();
    progress(&mut c, g, 40_000, 35_000);
    assert!(matches!(
        c.drain_actions()[0].kind,
        TimelineActionKind::Stop
    ));
    assert_eq!(c.snapshot().effective_mode, "degraded");
}

#[test]
fn config_roundtrip_does_not_accept_unknown_secret_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let config = Config::initial();
    config.save(&path).unwrap();
    config.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap(), config);
    let mut value = serde_json::to_value(&config).unwrap();
    value["routes"][0]["stream_key"] = serde_json::json!("not-allowed");
    assert!(serde_json::from_value::<Config>(value).is_err());
}

#[test]
fn automatic_ports_are_valid_but_same_explicit_port_is_not() {
    let mut config = Config::initial();
    assert_eq!(config.preferences.api_port, 0);
    assert_eq!(config.preferences.rtmp_port, 0);
    assert!(config.validate().is_ok());
    config.preferences.api_port = 49155;
    config.preferences.rtmp_port = 49155;
    assert!(config.validate().is_err());
    config.preferences.rtmp_port = 0;
    assert!(config.validate().is_ok());
}

#[test]
fn early_preparation_is_cancelled_when_observed_bitrate_exceeds_capacity() {
    let mut config = Config::initial();
    config.preferences.disk_bytes = 1024 * 1024;
    let mut c = Controller::new(config).unwrap();
    let g = c.connect("twitch").unwrap();
    assert!(
        c.execute(command("early", Action::PrepareDelay { delay_ms: 60_000 }))
            .success
    );
    assert!(c.snapshot().transition_in_progress);
    c.progress(
        "twitch",
        g,
        MediaProgress {
            latest_source_ms: 2000,
            oldest_source_ms: 0,
            stored_bytes: 200_000,
            ..Default::default()
        },
    )
    .unwrap();
    let snapshot = c.snapshot();
    let route = snapshot.routes.iter().find(|r| r.id == "twitch").unwrap();
    assert!(!snapshot.transition_in_progress);
    assert_eq!(route.desired_delay_ms, 0);
    assert!(route.buffer.degraded);
    assert_eq!(route.buffer.capacity_reason.as_deref(), Some("byte_budget"));
    assert!(route
        .error
        .as_deref()
        .unwrap()
        .contains("shared byte budget"));
    assert!(c.drain_actions().is_empty());
}

#[test]
fn live_return_reports_existing_delay_until_new_keyframe_is_applied() {
    let mut c = controller(Mode::Instant);
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 60_000, 0);
    c.execute(command("delay", Action::SetDelay { delay_ms: 15_000 }));
    let seek = c.drain_actions().pop().unwrap();
    c.applied("twitch", g, seek.revision, 45_000).unwrap();
    c.execute(command("live", Action::GoLive));
    let live = c.drain_actions().pop().unwrap();
    c.observed_output("twitch", g, 45_000).unwrap();
    assert!(c.snapshot().transition_in_progress);
    assert_eq!(c.snapshot().effective_delay_ms, 15_000);
    c.applied("twitch", g, live.revision, 59_990).unwrap();
    c.observed_output("twitch", g, 59_990).unwrap();
    let snapshot = c.snapshot();
    assert!(!snapshot.transition_in_progress);
    assert_eq!(snapshot.effective_delay_ms, 0);
    assert_eq!(snapshot.effective_mode, "live");
    assert_eq!(
        snapshot
            .routes
            .iter()
            .find(|r| r.id == "twitch")
            .unwrap()
            .actual_latency_ms,
        10
    );
}

#[test]
fn failed_larger_preparation_keeps_the_already_applied_delay() {
    let mut config = Config::initial();
    config.preferences.disk_bytes = 1024 * 1024;
    let mut c = Controller::new(config).unwrap();
    let g = c.connect("twitch").unwrap();
    progress(&mut c, g, 30_000, 0);
    c.execute(command("initial", Action::SetDelay { delay_ms: 15_000 }));
    let seek = c.drain_actions().pop().unwrap();
    c.applied("twitch", g, seek.revision, 15_000).unwrap();
    c.execute(command("larger", Action::PrepareDelay { delay_ms: 60_000 }));
    c.progress(
        "twitch",
        g,
        MediaProgress {
            latest_source_ms: 31_000,
            oldest_source_ms: 0,
            stored_bytes: 1_000_000,
            ..Default::default()
        },
    )
    .unwrap();
    let status = c.snapshot();
    assert!(!status.transition_in_progress);
    assert_eq!(status.desired_delay_ms, 15_000);
    assert_eq!(status.effective_delay_ms, 15_000);
    assert!(c.drain_actions().is_empty());
    c.observed_output("twitch", g, 16_000).unwrap();
    assert_eq!(c.snapshot().effective_delay_ms, 15_000);
}
