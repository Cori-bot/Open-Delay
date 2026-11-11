//! Visual-only browser-source settings and expiring tests. Never controls media.
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayConfig {
    pub theme: String,
    pub color: String,
    pub position: String,
    pub badge_size: String,
    pub popup_size: String,
    pub radius: u8,
    pub text: String,
    pub badge: bool,
    pub popup: bool,
    pub icon: bool,
    pub circle_icon: bool,
    pub icon_match_text: bool,
    pub hide_live: bool,
    pub instant_live: bool,
    pub live_green: bool,
    pub motion: bool,
    pub override_badge_text: bool,
    pub lang: String,
}
impl OverlayConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !["minimal", "studio", "saas", "signal", "glass", "light"].contains(&self.theme.as_str())
            || ![
                "red", "orange", "yellow", "lime", "green", "teal", "cyan", "blue", "indigo",
                "purple", "pink", "white",
            ]
            .contains(&self.color.as_str())
            || ![
                "top-left",
                "top-center",
                "top-right",
                "bottom-left",
                "bottom-center",
                "bottom-right",
            ]
            .contains(&self.position.as_str())
            || !["xs", "s", "m", "l"].contains(&self.badge_size.as_str())
            || !["s", "m", "l", "xl"].contains(&self.popup_size.as_str())
            || !["fr", "en"].contains(&self.lang.as_str())
            || self.radius > 100
            || self.text.encode_utf16().count() > 48
            || self
                .text
                .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
                != self.text
        {
            return Err("Invalid overlay configuration");
        }
        if (!self.icon && (self.circle_icon || self.icon_match_text))
            || (self.override_badge_text && self.text.trim().is_empty())
        {
            return Err("Overlay options require an icon or non-empty text");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestState {
    Live,
    Building,
    Delayed,
    Stop,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestRequest {
    pub state: TestState,
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
}
fn default_delay() -> u64 {
    15000
}
#[derive(Clone, Serialize)]
pub struct OverlayTest {
    pub id: String,
    pub state: TestState,
    pub delay_ms: u64,
    pub expires_at_ms: u64,
}
#[derive(Serialize)]
pub struct ControlSnapshot {
    pub config: Option<OverlayConfig>,
    pub revision: u64,
    pub source_recent: bool,
    pub test: Option<OverlayTest>,
}
pub struct OverlayControl {
    config: Option<OverlayConfig>,
    revision: u64,
    test: Option<(OverlayTest, Instant)>,
    last_source: Option<Instant>,
    path: PathBuf,
}
impl OverlayControl {
    pub fn load(path: PathBuf) -> Result<Self, crate::Error> {
        let config = match std::fs::read(&path) {
            Ok(bytes) => {
                let value: OverlayConfig = serde_json::from_slice(&bytes)?;
                value.validate()?;
                Some(value)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            config,
            revision: 0,
            test: None,
            last_source: None,
            path,
        })
    }
    pub fn heartbeat(&mut self) {
        self.last_source = Some(Instant::now());
    }
    pub fn snapshot(&mut self) -> ControlSnapshot {
        if self
            .test
            .as_ref()
            .is_some_and(|(_, deadline)| Instant::now() >= *deadline)
        {
            self.test = None;
        }
        ControlSnapshot {
            config: self.config.clone(),
            revision: self.revision,
            source_recent: self
                .last_source
                .is_some_and(|seen| seen.elapsed() < Duration::from_secs(3)),
            test: self.test.as_ref().map(|(value, _)| value.clone()),
        }
    }
    pub fn save(&mut self, config: OverlayConfig) -> Result<(), crate::Error> {
        config.validate()?;
        save_atomic(&self.path, &serde_json::to_vec_pretty(&config)?)?;
        self.config = Some(config);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn test(&mut self, request: TestRequest) -> Result<(), &'static str> {
        if request.delay_ms > 600000 {
            return Err("Test delay must be between 0 and 600000 ms");
        }
        self.test = if request.state == TestState::Stop {
            None
        } else {
            Some((
                OverlayTest {
                    id: uuid::Uuid::new_v4().to_string(),
                    state: request.state,
                    delay_ms: request.delay_ms,
                    expires_at_ms: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64
                        + 5000,
                },
                Instant::now() + Duration::from_secs(5),
            ))
        };
        Ok(())
    }
}
fn save_atomic(path: &Path, bytes: &[u8]) -> Result<(), crate::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let result = (|| -> Result<(), crate::Error> {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_validation_matches_browser_utf16_limit_and_trim() {
        let mut config: OverlayConfig = serde_json::from_value(serde_json::json!({"theme":"minimal","color":"purple","position":"top-right","badgeSize":"s","popupSize":"m","radius":64,"text":"","badge":true,"popup":true,"icon":true,"circleIcon":false,"iconMatchText":false,"hideLive":false,"instantLive":false,"liveGreen":false,"motion":true,"overrideBadgeText":false,"lang":"fr"})).unwrap();
        config.text = "🎮".repeat(24);
        assert!(config.validate().is_ok());
        config.text.push('a');
        assert!(config.validate().is_err());
        config.text = "hello world".into();
        assert!(config.validate().is_ok());
        for text in [
            " hello",
            "hello ",
            "\nhello",
            "hello\t",
            "\u{2003}hello",
            "\u{feff}hello",
        ] {
            config.text = text.into();
            assert!(
                config.validate().is_err(),
                "untrimmed input accepted: {text:?}"
            );
        }
    }

    #[test]
    fn failed_persistence_does_not_activate_config() {
        let dir = tempfile::tempdir().unwrap();
        let invalid_parent = dir.path().join("blocked");
        std::fs::write(&invalid_parent, "not a directory").unwrap();
        let mut control = OverlayControl {
            config: None,
            revision: 0,
            test: None,
            last_source: None,
            path: invalid_parent.join("overlay.json"),
        };
        let config: OverlayConfig = serde_json::from_value(serde_json::json!({"theme":"minimal","color":"purple","position":"top-right","badgeSize":"s","popupSize":"m","radius":64,"text":"","badge":true,"popup":true,"icon":true,"circleIcon":false,"iconMatchText":false,"hideLive":false,"instantLive":false,"liveGreen":false,"motion":true,"overrideBadgeText":false,"lang":"fr"})).unwrap();
        assert!(control.save(config).is_err());
        assert!(control.snapshot().config.is_none());
        assert_eq!(control.snapshot().revision, 0);
    }

    #[test]
    fn expired_test_and_source_heartbeat_clear() {
        let dir = tempfile::tempdir().unwrap();
        let mut control = OverlayControl::load(dir.path().join("overlay.json")).unwrap();
        control
            .test(TestRequest {
                state: TestState::Delayed,
                delay_ms: 15000,
            })
            .unwrap();
        control.heartbeat();
        assert!(control.snapshot().source_recent);
        assert!(control.snapshot().test.is_some());
        control.test.as_mut().unwrap().1 = Instant::now() - Duration::from_secs(1);
        control.last_source = Some(Instant::now() - Duration::from_secs(4));
        assert!(control.snapshot().test.is_none());
        assert!(!control.snapshot().source_recent);
        control
            .test(TestRequest {
                state: TestState::Live,
                delay_ms: 0,
            })
            .unwrap();
        control
            .test(TestRequest {
                state: TestState::Stop,
                delay_ms: 0,
            })
            .unwrap();
        assert!(control.snapshot().test.is_none());
        assert!(control
            .test(TestRequest {
                state: TestState::Delayed,
                delay_ms: 600001
            })
            .is_err());
    }
}
