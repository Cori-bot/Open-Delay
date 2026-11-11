use crate::{Config, CoreError, Mode, Preferences, Route, MAX_DELAY_MS};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    #[serde(default)]
    pub route_ids: Option<Vec<String>>,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    SetDelay {
        delay_ms: u64,
    },
    PrepareDelay {
        delay_ms: u64,
    },
    GoLive,
    SetMode {
        mode: Mode,
    },
    ToggleDelay {
        #[serde(default = "default_delay")]
        delay_ms: u64,
    },
    ToggleOverlay,
}
fn default_delay() -> u64 {
    15_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub reason: Option<String>,
    pub message: String,
    pub status: Status,
    pub results: Vec<RouteResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub route_id: String,
    pub success: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStatus {
    pub fill_ms: u64,
    pub target_ms: u64,
    pub ready: bool,
    pub degraded: bool,
    pub capacity_reason: Option<String>,
    pub size_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStatus {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub enabled: bool,
    pub include_in_group: bool,
    pub is_connected: bool,
    pub generation: u64,
    pub revision: u64,
    pub transport_state: String,
    pub effective_mode: String,
    pub desired_delay_ms: u64,
    pub effective_delay_ms: u64,
    /// Measured source-to-output lag, including ordinary live transport latency.
    pub actual_latency_ms: u64,
    pub buffer: BufferStatus,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub recovery: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub sequence: u64,
    pub mode: Mode,
    pub overlay_hidden: bool,
    pub buffer_ms: u64,
    pub is_connected: bool,
    pub effective_mode: String,
    pub desired_delay_ms: u64,
    pub effective_delay_ms: u64,
    pub transition_in_progress: bool,
    pub routes: Vec<RouteStatus>,
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MediaProgress {
    pub latest_source_ms: u64,
    pub oldest_source_ms: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub stored_bytes: u64,
}
#[derive(Debug, Clone)]
pub struct TimelineAction {
    pub route_id: String,
    pub generation: u64,
    pub revision: u64,
    pub kind: TimelineActionKind,
}
#[derive(Debug, Clone)]
pub enum TimelineActionKind {
    Seek {
        target_source_ms: u64,
        requested_delay_ms: u64,
    },
    Live,
    Stop,
}

#[derive(Debug, Clone, Default)]
struct Runtime {
    generation: u64,
    revision: u64,
    connected: bool,
    recovering: bool,
    progress: MediaProgress,
    has_media: bool,
    desired: u64,
    effective: u64,
    actual_latency: u64,
    pending: Option<Pending>,
    seeking: bool,
    error: Option<String>,
    degraded: bool,
}
#[derive(Debug, Clone)]
struct Pending {
    delay: u64,
    started_source: Option<u64>,
    overlay_wait: bool,
}

#[derive(Clone)]
pub struct Controller {
    config: Config,
    runtime: BTreeMap<String, Runtime>,
    sequence: u64,
    overlay_hidden: bool,
    actions: Vec<TimelineAction>,
    dedup: VecDeque<(Command, CommandResult)>,
}
impl Controller {
    pub fn new(config: Config) -> Result<Self, CoreError> {
        config.validate()?;
        let runtime = config
            .routes
            .iter()
            .map(|r| (r.id.clone(), Runtime::default()))
            .collect();
        Ok(Self {
            config,
            runtime,
            sequence: 0,
            overlay_hidden: false,
            actions: vec![],
            dedup: VecDeque::new(),
        })
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn snapshot(&self) -> Status {
        let routes: Vec<_> = self
            .config
            .routes
            .iter()
            .map(|r| {
                let s = &self.runtime[&r.id];
                let available = Self::available(s);
                let target = if s.desired > 0 {
                    s.desired
                } else {
                    self.config.preferences.buffer_ms
                };
                let displayed_fill = s
                    .pending
                    .as_ref()
                    .filter(|pending| pending.overlay_wait)
                    .map(|pending| {
                        pending
                            .started_source
                            .map(|start| s.progress.latest_source_ms.saturating_sub(start))
                            .unwrap_or(0)
                            .min(pending.delay)
                    })
                    .unwrap_or(available);
                RouteStatus {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    platform: r.platform.clone(),
                    enabled: r.enabled,
                    include_in_group: r.include_in_group,
                    is_connected: s.connected,
                    generation: s.generation,
                    revision: s.revision,
                    transport_state: if s.recovering {
                        "recovering"
                    } else if s.connected {
                        "connected"
                    } else if s.error.is_some() {
                        "error"
                    } else {
                        "disconnected"
                    }
                    .into(),
                    effective_mode: if s.degraded {
                        "degraded"
                    } else if s.pending.is_some() {
                        "building_delay"
                    } else if s.seeking {
                        "transitioning"
                    } else if s.effective > 0 {
                        if self.config.preferences.mode == Mode::Instant {
                            "instant_delayed"
                        } else {
                            "delayed"
                        }
                    } else {
                        "live"
                    }
                    .into(),
                    desired_delay_ms: s.desired,
                    effective_delay_ms: s.effective,
                    actual_latency_ms: s.actual_latency,
                    buffer: BufferStatus {
                        fill_ms: displayed_fill,
                        target_ms: target,
                        ready: s.has_media && displayed_fill >= target,
                        degraded: s.degraded,
                        capacity_reason: s.degraded.then(|| "byte_budget".into()),
                        size_bytes: s.progress.stored_bytes,
                    },
                    bytes_received: s.progress.bytes_received,
                    bytes_sent: s.progress.bytes_sent,
                    recovery: if s.recovering { "retrying" } else { "idle" }.into(),
                    error: s.error.clone(),
                }
            })
            .collect();
        let selected: Vec<_> = routes
            .iter()
            .filter(|r| r.is_connected && r.include_in_group)
            .collect();
        let transition = selected.iter().any(|r| {
            matches!(
                r.effective_mode.as_str(),
                "building_delay" | "transitioning"
            )
        });
        let effective = selected
            .iter()
            .map(|r| r.effective_delay_ms)
            .min()
            .unwrap_or(0);
        let desired = selected
            .iter()
            .map(|r| r.desired_delay_ms)
            .max()
            .unwrap_or(0);
        let mode = if selected.iter().any(|r| r.recovery == "retrying") {
            "recovering"
        } else if selected.iter().any(|r| r.buffer.degraded) {
            "degraded"
        } else if transition {
            "building_delay"
        } else if effective > 0 {
            if self.config.preferences.mode == Mode::Instant {
                "instant_delayed"
            } else {
                "delayed"
            }
        } else {
            "live"
        };
        Status {
            sequence: self.sequence,
            mode: self.config.preferences.mode,
            overlay_hidden: self.overlay_hidden,
            buffer_ms: self.config.preferences.buffer_ms,
            is_connected: routes.iter().any(|r| r.is_connected),
            effective_mode: mode.into(),
            desired_delay_ms: desired,
            effective_delay_ms: effective,
            transition_in_progress: transition,
            routes,
            preferences: self.config.preferences.clone(),
        }
    }
    pub fn connect(&mut self, id: &str) -> Result<u64, CoreError> {
        let route = self
            .config
            .routes
            .iter()
            .find(|r| r.id == id)
            .ok_or(CoreError::UnknownRoute)?;
        if !route.enabled {
            return Err(CoreError::Invalid("Route is disabled".into()));
        }
        let s = self.runtime.get_mut(id).ok_or(CoreError::UnknownRoute)?;
        let generation = s
            .generation
            .checked_add(1)
            .ok_or_else(|| CoreError::Invalid("Session counter exhausted".into()))?;
        *s = Runtime {
            generation,
            connected: true,
            ..Default::default()
        };
        self.actions.retain(|a| a.route_id != id);
        self.sequence += 1;
        Ok(generation)
    }
    pub fn disconnect(&mut self, id: &str, generation: u64) -> Result<(), CoreError> {
        let s = self.session(id, generation)?;
        let revision = s.revision.saturating_add(1);
        *s = Runtime {
            generation,
            revision,
            ..Default::default()
        };
        self.actions.retain(|a| a.route_id != id);
        self.sequence += 1;
        Ok(())
    }
    pub fn disconnect_with_error(
        &mut self,
        id: &str,
        generation: u64,
        message: String,
    ) -> Result<(), CoreError> {
        self.disconnect(id, generation)?;
        self.runtime
            .get_mut(id)
            .ok_or(CoreError::UnknownRoute)?
            .error = Some(message);
        Ok(())
    }
    pub fn set_error(
        &mut self,
        id: &str,
        generation: u64,
        message: Option<String>,
        recovering: bool,
    ) -> Result<(), CoreError> {
        let s = self.session(id, generation)?;
        s.error = message;
        s.recovering = recovering;
        self.sequence += 1;
        Ok(())
    }
    pub fn progress(
        &mut self,
        id: &str,
        generation: u64,
        progress: MediaProgress,
    ) -> Result<(), CoreError> {
        let s = self.session(id, generation)?;
        if progress.oldest_source_ms > progress.latest_source_ms {
            return Err(CoreError::Invalid("Invalid media bounds".into()));
        }
        s.progress = progress;
        s.has_media = true;
        if let Some(pending) = s.pending.as_mut() {
            if pending.started_source.is_none() {
                pending.started_source = Some(progress.latest_source_ms);
            }
        }
        self.maybe_activate(id)?;
        self.sequence += 1;
        Ok(())
    }
    pub fn applied(
        &mut self,
        id: &str,
        generation: u64,
        revision: u64,
        actual_source_ms: u64,
    ) -> Result<(), CoreError> {
        let s = self.session(id, generation)?;
        if s.revision != revision {
            return Err(CoreError::StaleSession);
        }
        s.actual_latency = s.progress.latest_source_ms.saturating_sub(actual_source_ms);
        s.effective = s.actual_latency;
        if s.desired == 0 {
            s.effective = 0;
        }
        s.seeking = false;
        s.pending = None;
        self.sequence += 1;
        Ok(())
    }
    pub fn observed_output(
        &mut self,
        id: &str,
        generation: u64,
        actual_source_ms: u64,
    ) -> Result<(), CoreError> {
        let s = self.session(id, generation)?;
        s.actual_latency = s.progress.latest_source_ms.saturating_sub(actual_source_ms);
        s.effective = if s.desired == 0 && !s.seeking {
            0
        } else {
            s.actual_latency
        };
        self.sequence += 1;
        Ok(())
    }
    pub fn drain_actions(&mut self) -> Vec<TimelineAction> {
        std::mem::take(&mut self.actions)
    }
    pub fn execute(&mut self, command: Command) -> CommandResult {
        if let Some((previous, result)) = self.dedup.iter().find(|(c, _)| c.id == command.id) {
            return if previous == &command {
                result.clone()
            } else {
                self.refusal(
                    "command_id_conflict",
                    "Command id was already used for a different action",
                )
            };
        }
        if command.id.is_empty() || command.id.len() > 128 {
            return self.refusal(
                "invalid_command_id",
                "Command id must contain 1–128 characters",
            );
        }
        let result = self.execute_new(&command);
        if self.dedup.len() == 1024 {
            self.dedup.pop_front();
        }
        self.dedup.push_back((command, result.clone()));
        result
    }
    fn execute_new(&mut self, command: &Command) -> CommandResult {
        let delay = match command.action {
            Action::SetDelay { delay_ms }
            | Action::PrepareDelay { delay_ms }
            | Action::ToggleDelay { delay_ms } => Some(delay_ms),
            _ => None,
        };
        if delay.is_some_and(|d| d > MAX_DELAY_MS) {
            return self.refusal("invalid_delay", "Delay must be between 0 and 600 seconds");
        }
        if let Action::ToggleOverlay = command.action {
            self.overlay_hidden = !self.overlay_hidden;
            self.sequence += 1;
            return self.ok(vec![]);
        }
        if let Action::SetMode { mode } = command.action {
            if self.config.preferences.mode == mode {
                return self.ok(vec![]);
            }
            let ids: Vec<_> = self
                .runtime
                .iter()
                .filter(|(_, s)| s.connected)
                .map(|(id, _)| id.clone())
                .collect();
            for id in ids {
                self.live(&id);
            }
            self.config.preferences.mode = mode;
            self.sequence += 1;
            return self.ok(vec![]);
        }
        let ids = command.route_ids.clone().unwrap_or_else(|| {
            self.config
                .routes
                .iter()
                .filter(|r| r.include_in_group && self.runtime[&r.id].connected)
                .map(|r| r.id.clone())
                .collect()
        });
        if ids.is_empty() {
            return self.refusal("stream_disconnected", "No connected routes selected");
        }
        if ids.len() > 32 {
            return self.refusal("invalid_routes", "Too many route selectors");
        }
        let mut results = vec![];
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Some(s) = self.runtime.get(&id) else {
                results.push(RouteResult {
                    route_id: id,
                    success: false,
                    reason: Some("unknown_route".into()),
                });
                continue;
            };
            if !s.connected {
                results.push(RouteResult {
                    route_id: id,
                    success: false,
                    reason: Some("stream_disconnected".into()),
                });
                continue;
            }
            let target = match command.action {
                Action::GoLive => 0,
                Action::ToggleDelay { delay_ms } => {
                    if s.desired > 0 || s.effective > 0 {
                        0
                    } else {
                        delay_ms
                    }
                }
                _ => delay.unwrap_or(0),
            };
            if target == 0 {
                self.live(&id);
                results.push(RouteResult {
                    route_id: id,
                    success: true,
                    reason: None,
                });
                continue;
            }
            if s.recovering || s.degraded {
                results.push(RouteResult {
                    route_id: id,
                    success: false,
                    reason: Some(
                        if s.recovering {
                            "recovering"
                        } else {
                            "buffer_degraded"
                        }
                        .into(),
                    ),
                });
                continue;
            }
            if !self.can_prepare(&id, target) {
                results.push(RouteResult {
                    route_id: id,
                    success: false,
                    reason: Some("byte_budget".into()),
                });
                continue;
            }
            let mode = self.config.preferences.mode;
            let s = self.runtime.get_mut(&id).expect("route checked");
            s.revision += 1;
            s.desired = target;
            s.seeking = false;
            let overlay_wait = mode == Mode::Overlay && s.effective == 0;
            s.pending = Some(Pending {
                delay: target,
                started_source: s.has_media.then_some(s.progress.latest_source_ms),
                overlay_wait,
            });
            self.actions.retain(|a| a.route_id != id);
            if target > self.config.preferences.buffer_ms {
                self.config.preferences.buffer_ms = target;
            }
            let _ = self.maybe_activate(&id);
            results.push(RouteResult {
                route_id: id,
                success: true,
                reason: None,
            });
        }
        self.sequence += 1;
        self.ok(results)
    }
    fn can_prepare(&self, id: &str, delay: u64) -> bool {
        let requested = &self.runtime[id];
        let span = Self::available(requested);
        if span < 1000 || requested.progress.stored_bytes == 0 {
            return true;
        }
        let projected = requested
            .progress
            .stored_bytes
            .saturating_mul(delay.saturating_add(10_000))
            / span;
        let others: u64 = self
            .runtime
            .iter()
            .filter(|(route, _)| route.as_str() != id)
            .map(|(_, s)| s.progress.stored_bytes)
            .sum();
        projected.saturating_add(others) <= self.config.preferences.disk_bytes
    }
    fn maybe_activate(&mut self, id: &str) -> Result<(), CoreError> {
        // The first command may arrive before any bitrate is observable. Recheck
        // that admission as media accumulates instead of preparing forever while
        // the bounded ring repeatedly expires the requested history.
        let pending_fits = self
            .runtime
            .get(id)
            .ok_or(CoreError::UnknownRoute)?
            .pending
            .as_ref()
            .map(|pending| self.can_prepare(id, pending.delay))
            .unwrap_or(true);
        let s = self.runtime.get_mut(id).ok_or(CoreError::UnknownRoute)?;
        let available = Self::available(s);
        if s.effective > 0 && available.saturating_add(2000) < s.effective {
            s.degraded = true;
            s.error = Some("Insufficient retained history to sustain the active delay".into());
            s.pending = None;
            s.seeking = false;
            s.revision += 1;
            self.actions.push(TimelineAction {
                route_id: id.into(),
                generation: s.generation,
                revision: s.revision,
                kind: TimelineActionKind::Stop,
            });
            return Ok(());
        }
        if !pending_fits {
            // Keep any previously applied delay selected. A failed larger
            // preparation must never silently switch the output to live.
            s.desired = s.effective;
            s.pending = None;
            s.seeking = false;
            s.degraded = true;
            s.error =
                Some("Requested delay cannot be sustained within the shared byte budget".into());
            s.revision += 1;
            self.actions.retain(|action| action.route_id != id);
            return Ok(());
        }
        if s.recovering {
            return Ok(());
        }
        if let Some(p) = &s.pending {
            let waited = p
                .started_source
                .map(|start| s.progress.latest_source_ms.saturating_sub(start))
                .unwrap_or(0);
            if available >= p.delay && (!p.overlay_wait || waited >= p.delay) {
                let delay = p.delay;
                s.pending = None;
                s.seeking = true;
                self.actions.push(TimelineAction {
                    route_id: id.into(),
                    generation: s.generation,
                    revision: s.revision,
                    kind: TimelineActionKind::Seek {
                        target_source_ms: s.progress.latest_source_ms.saturating_sub(delay),
                        requested_delay_ms: delay,
                    },
                });
            }
        }
        Ok(())
    }
    fn live(&mut self, id: &str) {
        if let Some(s) = self.runtime.get_mut(id) {
            s.revision += 1;
            s.desired = 0;
            s.pending = None;
            s.seeking = true;
            s.degraded = false;
            self.actions.retain(|a| a.route_id != id);
            self.actions.push(TimelineAction {
                route_id: id.into(),
                generation: s.generation,
                revision: s.revision,
                kind: TimelineActionKind::Live,
            });
        }
    }
    fn available(s: &Runtime) -> u64 {
        if s.has_media {
            s.progress
                .latest_source_ms
                .saturating_sub(s.progress.oldest_source_ms)
        } else {
            0
        }
    }
    fn session(&mut self, id: &str, generation: u64) -> Result<&mut Runtime, CoreError> {
        let s = self.runtime.get_mut(id).ok_or(CoreError::UnknownRoute)?;
        if !s.connected || s.generation != generation {
            return Err(CoreError::StaleSession);
        }
        Ok(s)
    }
    fn refusal(&self, reason: &str, message: &str) -> CommandResult {
        CommandResult {
            success: false,
            reason: Some(reason.into()),
            message: message.into(),
            status: self.snapshot(),
            results: vec![],
        }
    }
    fn ok(&self, results: Vec<RouteResult>) -> CommandResult {
        let success = results.iter().all(|r| r.success);
        CommandResult {
            success,
            reason: (!success).then(|| "partial_failure".into()),
            message: if success {
                "Command accepted"
            } else {
                "Some selected routes refused the command"
            }
            .into(),
            status: self.snapshot(),
            results,
        }
    }
    pub fn save_route(&mut self, route: Route) -> Result<(), CoreError> {
        Config::validate_route(&route)?;
        if self.runtime.get(&route.id).is_some_and(|s| s.connected) {
            return Err(CoreError::Connected);
        }
        let mut next = self.config.clone();
        if let Some(r) = next.routes.iter_mut().find(|r| r.id == route.id) {
            *r = route.clone();
        } else {
            next.routes.push(route.clone());
        }
        next.validate()?;
        self.config = next;
        self.runtime.entry(route.id).or_default();
        self.sequence += 1;
        Ok(())
    }
    pub fn delete_route(&mut self, id: &str) -> Result<(), CoreError> {
        if id == "twitch" {
            return Err(CoreError::Invalid(
                "Default Twitch route cannot be deleted".into(),
            ));
        }
        let runtime = self.runtime.get(id).ok_or(CoreError::UnknownRoute)?;
        if runtime.connected {
            return Err(CoreError::Connected);
        }
        self.runtime.remove(id);
        self.config.routes.retain(|r| r.id != id);
        self.actions.retain(|a| a.route_id != id);
        self.sequence += 1;
        Ok(())
    }
    pub fn set_preferences(&mut self, preferences: Preferences) -> Result<(), CoreError> {
        let mut next = self.config.clone();
        next.preferences = preferences;
        next.validate()?;
        if self.runtime.values().any(|s| s.connected)
            && (next.preferences.disk_bytes != self.config.preferences.disk_bytes
                || next.preferences.cache_bytes != self.config.preferences.cache_bytes
                || next.preferences.mode != self.config.preferences.mode
                || next.preferences.api_port != self.config.preferences.api_port
                || next.preferences.rtmp_port != self.config.preferences.rtmp_port
                || next.preferences.lan_ingest != self.config.preferences.lan_ingest)
        {
            return Err(CoreError::Connected);
        }
        self.config = next;
        self.sequence += 1;
        Ok(())
    }
    pub fn update_preferences(&mut self, preferences: Preferences) -> Result<(), CoreError> {
        self.set_preferences(preferences)
    }
    pub fn upsert_route(&mut self, route: Route) -> Result<(), CoreError> {
        self.save_route(route)
    }
    pub fn remove_route(&mut self, id: &str) -> Result<(), CoreError> {
        self.delete_route(id)
    }
    pub fn set_recovery(
        &mut self,
        id: &str,
        generation: u64,
        reason: &str,
    ) -> Result<(), CoreError> {
        self.set_error(id, generation, Some(reason.into()), true)
    }
}
