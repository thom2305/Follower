// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub url: String,
    pub token: String,
    pub timezone: String,
}

impl Default for Config {
    fn default() -> Self {
        Config { url: String::new(), token: String::new(), timezone: "America/Toronto".into() }
    }
}

impl Config {
    pub fn ready(&self) -> bool {
        !self.url.trim().is_empty() && !self.token.trim().is_empty()
    }
    pub fn zone(&self) -> chrono_tz::Tz {
        crate::util::zone(&self.timezone)
    }
}

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Self {
        let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("follower");
        ConfigStore { path: dir.join("config.json") }
    }

    pub fn load(&self) -> Config {
        let mut c = Config::default();
        if let Ok(text) = std::fs::read_to_string(&self.path) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                if let Some(s) = v.get("url").and_then(Value::as_str) { c.url = s.into(); }
                if let Some(s) = v.get("api_token").and_then(Value::as_str) { c.token = s.into(); }
                if let Some(s) = v.get("tz").and_then(Value::as_str) { c.timezone = s.into(); }
            }
        }
        c
    }

    pub fn save(&self, c: &Config) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = json!({ "url": c.url.trim(), "api_token": c.token.trim(), "tz": c.timezone.trim() });
        std::fs::write(&self.path, serde_json::to_string_pretty(&body).unwrap())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

impl Default for ConfigStore {
    fn default() -> Self { Self::new() }
}
