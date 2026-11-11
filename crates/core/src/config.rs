use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io::Write, path::Path};

pub const MAX_DELAY_MS: u64 = 600_000;
pub const DEFAULT_DISK_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const DEFAULT_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Invalid(String),
    #[error("Unknown route")]
    UnknownRoute,
    #[error("Route is currently connected")]
    Connected,
    #[error("Stale route session")]
    StaleSession,
    #[error("Storage I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Instant,
    Overlay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub language: String,
    pub mode: Mode,
    pub buffer_ms: u64,
    pub disk_bytes: u64,
    pub cache_bytes: usize,
    pub api_port: u16,
    pub rtmp_port: u16,
    pub lan_ingest: bool,
    pub startup: bool,
    pub quit_on_close: bool,
    pub onboarding_complete: bool,
    pub hotkeys: Hotkeys,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: "fr".into(),
            mode: Mode::Instant,
            buffer_ms: 60_000,
            disk_bytes: DEFAULT_DISK_BYTES,
            cache_bytes: DEFAULT_CACHE_BYTES,
            api_port: 0,
            rtmp_port: 0,
            lan_ingest: false,
            startup: false,
            quit_on_close: false,
            onboarding_complete: false,
            hotkeys: Hotkeys::default(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Hotkeys {
    pub start_delay: Option<String>,
    pub go_live: Option<String>,
    pub toggle_overlay: Option<String>,
    pub custom_delay: Option<String>,
    pub custom_delay_ms: u64,
}
impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            start_delay: None,
            go_live: None,
            toggle_overlay: None,
            custom_delay: None,
            custom_delay_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub local_app: String,
    pub upstream_url: String,
    pub enabled: bool,
    pub include_in_group: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub preferences: Preferences,
    pub routes: Vec<Route>,
}
impl Default for Config {
    fn default() -> Self {
        Self::initial()
    }
}

impl Config {
    pub fn initial() -> Self {
        let mut config = Self {
            preferences: Preferences::default(),
            routes: vec![],
        };
        for (id, name, upstream) in [
            ("twitch", "Twitch", "rtmp://live.twitch.tv/app"),
            ("tiktok", "TikTok", ""),
            ("kick", "Kick", ""),
            ("youtube", "YouTube", "rtmp://a.rtmp.youtube.com/live2"),
            ("restream", "Restream", ""),
            ("custom", "Custom RTMP", ""),
        ] {
            config.routes.push(Route {
                id: id.into(),
                name: name.into(),
                platform: id.into(),
                local_app: id.into(),
                upstream_url: upstream.into(),
                enabled: id == "twitch",
                include_in_group: true,
            });
        }
        config
    }
    pub fn validate(&self) -> Result<(), CoreError> {
        let p = &self.preferences;
        if !matches!(p.language.as_str(), "fr" | "en")
            || p.buffer_ms < 1_000
            || p.buffer_ms > MAX_DELAY_MS
            || p.hotkeys.custom_delay_ms < 1_000
            || p.hotkeys.custom_delay_ms > MAX_DELAY_MS
            || p.disk_bytes < 1024 * 1024
            || p.disk_bytes > 32 * 1024 * 1024 * 1024
            || p.cache_bytes > 256 * 1024 * 1024
            || (p.api_port != 0 && p.api_port == p.rtmp_port)
        {
            return Err(CoreError::Invalid("Invalid preferences".into()));
        }
        let mut hotkeys = HashSet::new();
        for key in [
            &p.hotkeys.start_delay,
            &p.hotkeys.go_live,
            &p.hotkeys.toggle_overlay,
            &p.hotkeys.custom_delay,
        ]
        .into_iter()
        .flatten()
        {
            if key.len() > 80 || key.trim().is_empty() || !hotkeys.insert(key.to_lowercase()) {
                return Err(CoreError::Invalid("Invalid or duplicate hotkey".into()));
            }
        }
        if self.routes.len() > 32 {
            return Err(CoreError::Invalid("Too many routes".into()));
        }
        let mut ids = HashSet::new();
        let mut apps = HashSet::new();
        for route in &self.routes {
            Self::validate_route(route)?;
            if !ids.insert(&route.id) || !apps.insert(&route.local_app) {
                return Err(CoreError::Invalid(
                    "Route ids and local paths must be unique".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn validate_route(route: &Route) -> Result<(), CoreError> {
        for value in [&route.id, &route.local_app] {
            if value.is_empty()
                || value.len() > 64
                || !value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
            {
                return Err(CoreError::Invalid(
                    "Invalid route identity or local app".into(),
                ));
            }
        }
        if route.name.is_empty() || route.name.chars().count() > 80 || route.platform.len() > 32 {
            return Err(CoreError::Invalid("Invalid route name".into()));
        }
        if route.upstream_url.is_empty() && !route.enabled {
            return Ok(());
        }
        let url = url::Url::parse(&route.upstream_url)
            .map_err(|_| CoreError::Invalid("Invalid upstream URL".into()))?;
        if !matches!(url.scheme(), "rtmp" | "rtmps")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path().len() > 512
        {
            return Err(CoreError::Invalid(
                "Upstream must be an RTMP(S) server URL without credentials, query or fragment"
                    .into(),
            ));
        }
        Ok(())
    }
    pub fn load(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        match fs::read(path) {
            Ok(bytes) => {
                let config: Self = serde_json::from_slice(&bytes)?;
                config.validate()?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::initial()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), CoreError> {
        self.validate()?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        let bytes = serde_json::to_vec_pretty(self)?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        let result = (|| -> Result<(), CoreError> {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}
