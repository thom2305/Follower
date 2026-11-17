// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::json::JsonExt;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub key: String,
    pub from: String,
    pub to: String,
}

fn stringify(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(), // numbers, bools
    }
}

pub fn parse_changes(o: Option<&Value>) -> Vec<Change> {
    let Some(Value::Object(m)) = o else { return vec![] };
    m.iter()
        .map(|(k, v)| {
            let a = v.as_array();
            Change {
                key: k.clone(),
                from: stringify(a.and_then(|a| a.first())),
                to: stringify(a.and_then(|a| a.get(1))),
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub accent_color: Option<i64>,
    pub bot: bool,
    pub badges: Vec<String>,
    pub created_at: String,
}

impl Profile {
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }
    pub fn from_json(o: &Value) -> Profile {
        Profile {
            username: o.str_("username").unwrap_or_default(),
            global_name: o.str_("global_name"),
            avatar: o.str_("avatar"),
            banner: o.str_("banner"),
            accent_color: o.int("accent_color"),
            bot: o.flag("bot"),
            badges: o.strings("badges"),
            created_at: o.str_("created_at").unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Person {
    pub user_id: i64,
    pub relation: String,
    pub notes: Option<String>,
    pub met_context: Option<String>,
    pub birthday_month: Option<u32>,
    pub birthday_day: Option<u32>,
    pub birthday_year: Option<i32>,
    pub first_seen: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Person {
    pub fn from_json(o: &Value) -> Option<Person> {
        let user_id = o.id64("user_id")?;
        Some(Person {
            user_id,
            relation: o.str_("relation").unwrap_or_else(|| "other".into()),
            notes: o.str_("notes"),
            met_context: o.str_("met_context"),
            birthday_month: o.int("birthday_month").and_then(|v| u32::try_from(v).ok()),
            birthday_day: o.int("birthday_day").and_then(|v| u32::try_from(v).ok()),
            birthday_year: o.int("birthday_year").and_then(|v| i32::try_from(v).ok()),
            first_seen: o.date("first_seen").unwrap_or_else(Utc::now),
            updated_at: o.date("updated_at").unwrap_or_else(Utc::now),
            tags: o.strings("tags"),
        })
    }
}

#[derive(Clone, Debug)]
pub struct PersonEntry {
    pub person: Person,
    pub profile: Option<Profile>,
}

impl PersonEntry {
    pub fn name(&self) -> String {
        self.profile.as_ref().map(|p| p.display_name().to_owned()).unwrap_or_else(|| self.person.user_id.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct SnapResult {
    pub profile: Option<Profile>,
    pub changes: Vec<Change>,
    pub is_first: bool,
}

#[derive(Clone, Debug)]
pub struct StatusEntry { pub status: String, pub at: DateTime<Utc> }
#[derive(Clone, Debug)]
pub struct ProfileChange { pub at: DateTime<Utc>, pub changes: Vec<Change>, pub is_first: bool }
#[derive(Clone, Debug)]
pub struct PersonHistory { pub statuses: Vec<StatusEntry>, pub profile_changes: Vec<ProfileChange> }

#[derive(Clone, Debug)]
pub struct Attachment {
    pub id: i64,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub url: Option<String>,
    pub compressed: bool,
    pub stored_size: i64,
    pub stored: bool,
}

#[derive(Clone, Debug)]
pub struct ArchiveMsg {
    pub message_id: i64,
    pub author_id: i64,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub content: Option<String>,
    pub jump_url: Option<String>,
    pub message_ts: DateTime<Utc>,
    pub saved_at: DateTime<Utc>,
    pub attachments: Vec<Attachment>,
}

impl ArchiveMsg {
    pub fn from_json(o: &Value) -> Option<ArchiveMsg> {
        let message_id = o.id64("message_id")?;
        Some(ArchiveMsg {
            message_id,
            author_id: o.id64("author_id").unwrap_or(0),
            author_name: o.str_("author_name").unwrap_or_else(|| "?".into()),
            author_avatar: o.str_("author_avatar"),
            content: o.str_("content"),
            jump_url: o.str_("jump_url"),
            message_ts: o.date("message_ts").unwrap_or_else(Utc::now),
            saved_at: o.date("saved_at").unwrap_or_else(Utc::now),
            attachments: o
                .objs("attachments")
                .into_iter()
                .filter_map(|a| {
                    Some(Attachment {
                        id: a.id64("id")?,
                        filename: a.str_("filename").unwrap_or_else(|| "file".into()),
                        content_type: a.str_("content_type"),
                        size: a.id64("size").unwrap_or(0),
                        url: a.str_("url"),
                        compressed: a.flag("compressed"),
                        stored_size: a.id64("stored_size").unwrap_or(0),
                        stored: a.flag("stored"),
                    })
                })
                .collect(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct AuthorChoice { pub id: i64, pub name: String, pub count: i64 }
#[derive(Clone, Debug)]
pub struct TagCount { pub tag: String, pub count: i64 }

#[derive(Clone, Debug)]
pub struct LookupEntry {
    pub user_id: i64,
    pub profile: Option<Profile>,
    pub changes: Vec<Change>,
    pub person: Option<Person>,
}
