// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use chrono::{DateTime, Utc};
use serde_json::Value;

pub trait JsonExt {
    fn str_(&self, k: &str) -> Option<String>;
    fn int(&self, k: &str) -> Option<i64>;
    fn flag(&self, k: &str) -> bool;
    fn obj(&self, k: &str) -> Option<&Value>;
    fn objs(&self, k: &str) -> Vec<&Value>;
    fn strings(&self, k: &str) -> Vec<String>;
    fn id64(&self, k: &str) -> Option<i64>;
    fn date(&self, k: &str) -> Option<DateTime<Utc>>;
}

impl JsonExt for Value {
    fn str_(&self, k: &str) -> Option<String> {
        self.get(k)?.as_str().map(str::to_owned)
    }
    fn int(&self, k: &str) -> Option<i64> {
        self.get(k)?.as_i64()
    }
    fn flag(&self, k: &str) -> bool {
        self.get(k).and_then(Value::as_bool).unwrap_or(false)
    }
    fn obj(&self, k: &str) -> Option<&Value> {
        self.get(k).filter(|v| v.is_object())
    }
    fn objs(&self, k: &str) -> Vec<&Value> {
        self.get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter(|v| v.is_object()).collect())
            .unwrap_or_default()
    }
    fn strings(&self, k: &str) -> Vec<String> {
        self.get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
            .unwrap_or_default()
    }
    fn id64(&self, k: &str) -> Option<i64> {
        match self.get(k)? {
            Value::String(s) => s.parse().ok(),
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }
    fn date(&self, k: &str) -> Option<DateTime<Utc>> {
        self.str_(k).and_then(|s| crate::util::parse_iso(&s))
    }
}
