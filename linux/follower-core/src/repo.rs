// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::api::{Api, ApiError};
use crate::json::JsonExt;
use crate::models::*;
use crate::util::{birthday_input, enc};
use serde_json::{json, Value};

type R<T> = Result<T, ApiError>;

#[derive(Clone)]
pub struct Repo {
    api: Api,
}

fn ids_json(ids: &[i64]) -> Vec<String> {
    ids.iter().map(|i| i.to_string()).collect()
}

fn snap_of(o: &Value) -> SnapResult {
    SnapResult {
        profile: if o.flag("found") { o.obj("profile").map(Profile::from_json) } else { None },
        changes: parse_changes(o.obj("changes")),
        is_first: o.flag("is_first"),
    }
}

impl Repo {
    pub fn new(api: Api) -> Repo {
        Repo { api }
    }

    pub fn list_people(&self) -> R<Vec<PersonEntry>> {
        let o = self.api.get("/api/people")?;
        Ok(o.objs("people")
            .into_iter()
            .filter_map(|p| {
                Some(PersonEntry { person: Person::from_json(p)?, profile: p.obj("profile").map(Profile::from_json) })
            })
            .collect())
    }

    pub fn get_detail(&self, id: i64) -> R<(Option<Person>, Option<Profile>)> {
        let o = self.api.get(&format!("/api/people/{id}"))?;
        Ok((o.obj("person").and_then(Person::from_json), o.obj("profile").map(Profile::from_json)))
    }

    pub fn get_person(&self, id: i64) -> R<Option<Person>> {
        Ok(self.get_detail(id)?.0)
    }

    pub fn ensure_person(&self, id: i64) -> R<()> {
        self.api.post(&format!("/api/people/{id}"), None).map(|_| ())
    }

    pub fn set_relation(&self, id: i64, relation: &str) -> R<()> {
        self.api.post(&format!("/api/people/{id}"), Some(json!({ "relation": relation }))).map(|_| ())
    }

    pub fn update_details(
        &self, id: i64, notes: Option<&str>, met: Option<&str>,
        year: Option<i32>, month: Option<u32>, day: Option<u32>,
    ) -> R<()> {
        self.api
            .post(
                &format!("/api/people/{id}"),
                Some(json!({
                    "notes": notes.unwrap_or(""),
                    "met_context": met.unwrap_or(""),
                    "birthday": birthday_input(month, day, year),
                })),
            )
            .map(|_| ())
    }

    pub fn add_tag(&self, id: i64, tag: &str) -> R<()> {
        self.api.post(&format!("/api/people/{id}/tags"), Some(json!({ "tag": tag }))).map(|_| ())
    }

    pub fn remove_tag(&self, id: i64, tag: &str) -> R<()> {
        self.api.delete(&format!("/api/people/{id}/tags/{}", enc(tag))).map(|_| ())
    }

    pub fn remove_person(&self, id: i64) -> R<()> {
        self.api.delete(&format!("/api/people/{id}")).map(|_| ())
    }

    pub fn save_many(&self, ids: &[i64]) -> R<()> {
        self.api.post("/api/people/save", Some(json!({ "ids": ids_json(ids) }))).map(|_| ())
    }

    pub fn all_tags(&self) -> R<Vec<TagCount>> {
        let o = self.api.get("/api/tags")?;
        Ok(o.objs("tags")
            .into_iter()
            .filter_map(|t| Some(TagCount { tag: t.str_("tag")?, count: t.int("count").unwrap_or(0) }))
            .collect())
    }

    pub fn counts(&self) -> R<(i64, i64)> {
        let o = self.api.get("/api/ping")?;
        Ok((o.int("people").unwrap_or(0), o.int("archive").unwrap_or(0)))
    }

    pub fn take_snapshot(&self, id: i64) -> R<SnapResult> {
        Ok(snap_of(&self.api.post(&format!("/api/people/{id}/refresh"), None)?))
    }

    pub fn history(&self, id: i64) -> R<PersonHistory> {
        let o = self.api.get(&format!("/api/people/{id}/history"))?;
        Ok(PersonHistory {
            statuses: o
                .objs("statuses")
                .into_iter()
                .map(|s| StatusEntry {
                    status: s.str_("status").unwrap_or_default(),
                    at: s.date("at").unwrap_or_else(chrono::Utc::now),
                })
                .collect(),
            profile_changes: o
                .objs("profile_changes")
                .into_iter()
                .map(|c| ProfileChange {
                    at: c.date("at").unwrap_or_else(chrono::Utc::now),
                    changes: parse_changes(c.obj("changes")),
                    is_first: c.flag("is_first"),
                })
                .collect(),
        })
    }

    pub fn lookup(&self, ids: &[i64], save: bool, relation: Option<&str>, tag: Option<&str>) -> R<Vec<LookupEntry>> {
        let mut body = json!({ "ids": ids_json(ids), "save": save });
        if let Some(r) = relation { body["relation"] = json!(r); }
        if let Some(t) = tag { body["tag"] = json!(t); }
        let o = self.api.post("/api/lookup", Some(body))?;
        Ok(o.objs("results")
            .into_iter()
            .filter_map(|r| {
                let user_id = r.id64("user_id")?;
                let s = snap_of(r);
                Some(LookupEntry {
                    user_id,
                    profile: s.profile,
                    changes: s.changes,
                    person: r.obj("person").and_then(Person::from_json),
                })
            })
            .collect())
    }

    pub fn archive_search(&self, text: &str, author: &str) -> R<Vec<ArchiveMsg>> {
        let o = self.api.get(&format!("/api/archive?text={}&author={}", enc(text), enc(author)))?;
        Ok(o.objs("messages").into_iter().filter_map(ArchiveMsg::from_json).collect())
    }

    pub fn archive_random(&self, author: &str) -> R<Option<ArchiveMsg>> {
        let o = self.api.get(&format!("/api/archive/random?author={}", enc(author)))?;
        Ok(o.obj("message").and_then(ArchiveMsg::from_json))
    }

    pub fn archive_delete(&self, message_id: i64) -> R<bool> {
        Ok(self.api.delete(&format!("/api/archive/{message_id}"))?.flag("deleted"))
    }

    pub fn authors(&self) -> R<Vec<AuthorChoice>> {
        let o = self.api.get("/api/archive/authors")?;
        Ok(o.objs("authors")
            .into_iter()
            .filter_map(|a| {
                Some(AuthorChoice {
                    id: a.id64("id")?,
                    name: a.str_("name").unwrap_or_else(|| "?".into()),
                    count: a.int("count").unwrap_or(0),
                })
            })
            .collect())
    }

    pub fn load_attachment(&self, file_id: i64) -> R<Vec<u8>> {
        self.api.bytes("GET", &format!("/api/archive/files/{file_id}"), None)
    }
}
