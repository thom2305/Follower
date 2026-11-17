// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::config::Config;
use crate::json::JsonExt;
use serde_json::Value;
use std::io::Read;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ApiError(pub String);

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ApiError {}

fn err<T>(m: impl Into<String>) -> Result<T, ApiError> {
    Err(ApiError(m.into()))
}

#[derive(Clone)]
pub struct Api {
    agent: ureq::Agent,
    cfg: Arc<RwLock<Config>>,
}

impl Api {
    pub fn new(cfg: Arc<RwLock<Config>>) -> Api {
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(60)).build();
        Api { agent, cfg }
    }

    pub fn get(&self, path: &str) -> Result<Value, ApiError> {
        self.json("GET", path, None)
    }
    pub fn post(&self, path: &str, body: Option<Value>) -> Result<Value, ApiError> {
        self.json("POST", path, Some(body.unwrap_or_else(|| serde_json::json!({}))))
    }
    pub fn delete(&self, path: &str) -> Result<Value, ApiError> {
        self.json("DELETE", path, None)
    }

    fn json(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, ApiError> {
        let data = self.bytes(method, path, body)?;
        match serde_json::from_slice::<Value>(&data) {
            Ok(v) if v.is_object() => Ok(v),
            _ => err("The server sent something that isn't JSON."),
        }
    }

    pub fn bytes(&self, method: &str, path: &str, body: Option<Value>) -> Result<Vec<u8>, ApiError> {
        let c = self.cfg.read().unwrap().clone();
        if !c.ready() {
            return err("Set the server URL and API token in Settings first.");
        }
        let mut base = c.url.trim().to_owned();
        while base.ends_with('/') {
            base.pop();
        }
        if !base.starts_with("http://") && !base.starts_with("https://") {
            base = format!("https://{base}");
        }
        let url = format!("{base}{path}");

        let req = self
            .agent
            .request(method, &url)
            .set("Authorization", &format!("Bearer {}", c.token.trim()))
            .set("Accept", "application/json");
        let result = match body {
            Some(b) => req.set("Content-Type", "application/json; charset=utf-8").send_string(&b.to_string()),
            None => req.call(),
        };

        match result {
            Ok(resp) => read_all(resp),
            Err(ureq::Error::Status(code, resp)) => {
                let text = resp.into_string().unwrap_or_default();
                let msg = serde_json::from_str::<Value>(&text).ok().and_then(|v| v.str_("error"));
                if code == 401 {
                    err("Wrong API token (401).")
                } else if let Some(m) = msg {
                    err(m)
                } else {
                    err(format!("Server returned HTTP {code}."))
                }
            }
            Err(ureq::Error::Transport(t)) => err(format!("Couldn't reach the server: {t}")),
        }
    }
}

fn read_all(resp: ureq::Response) -> Result<Vec<u8>, ApiError> {
    let mut buf = Vec::new();
    resp.into_reader()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut buf)
        .map_err(|e| ApiError(format!("Download failed: {e}")))?;
    Ok(buf)
}

pub fn fetch_url(url: &str) -> Result<Vec<u8>, ApiError> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    match agent.get(url).call() {
        Ok(resp) => {
            let mut buf = Vec::new();
            resp.into_reader()
                .take(16 * 1024 * 1024)
                .read_to_end(&mut buf)
                .map_err(|e| ApiError(e.to_string()))?;
            Ok(buf)
        }
        Err(e) => err(e.to_string()),
    }
}
