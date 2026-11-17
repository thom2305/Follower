// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;

pub const STATUSES: [(&str, &str); 7] = [
    ("friend", "Friend"),
    ("unfriended", "I unfriended them"),
    ("unfriended_me", "They unfriended me"),
    ("blocked", "I blocked them"),
    ("blocked_me", "They blocked me"),
    ("acquaintance", "Acquaintance"),
    ("other", "Other"),
];

pub fn status_label(key: &str) -> String {
    STATUSES.iter().find(|(k, _)| *k == key).map(|(_, l)| (*l).to_owned()).unwrap_or_else(|| key.to_owned())
}

pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

pub fn norm_tag(t: &str) -> String {
    let joined = t.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(32).collect()
}

pub fn parse_ids(text: &str) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut Vec<i64>| {
        let mut rest: &str = run.as_str();
        while rest.len() >= 15 {
            let take = rest.len().min(20);
            if let Ok(v) = rest[..take].parse::<i64>() {
                if !out.contains(&v) {
                    out.push(v);
                }
            }
            rest = &rest[take..];
        }
        run.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            run.push(ch);
        } else {
            flush(&mut run, &mut out);
        }
    }
    flush(&mut run, &mut out);
    out
}

pub fn human_size(bytes: i64) -> String {
    let mut n = bytes as f64;
    for unit in ["B", "KB", "MB", "GB"] {
        if n < 1024.0 || unit == "GB" {
            return if unit == "B" { format!("{:.0} B", n) } else { format!("{:.1} {}", n, unit) };
        }
        n /= 1024.0;
    }
    format!("{} B", bytes)
}

pub fn zone(name: &str) -> Tz {
    name.trim().parse::<Tz>().unwrap_or(chrono_tz::UTC)
}

pub fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(n.and_utc());
        }
    }
    None
}

pub fn pretty(d: &DateTime<Utc>, tz: Tz) -> String {
    d.with_timezone(&tz).format("%b %-d, %Y, %-I:%M %p").to_string()
}

pub fn pretty_date(d: &DateTime<Utc>, tz: Tz) -> String {
    d.with_timezone(&tz).format("%b %-d, %Y").to_string()
}

pub fn pretty_str(s: &str, tz: Tz) -> String {
    parse_iso(s).map(|d| pretty(&d, tz)).unwrap_or_else(|| s.to_owned())
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BirthdayParts {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
}

pub fn parse_birthday(text: &str) -> Result<BirthdayParts, ()> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(BirthdayParts::default());
    }
    let parts: Vec<&str> = t.split('-').collect();
    let num = |s: &str, max_len: usize| -> Result<u32, ()> {
        if s.is_empty() || s.len() > max_len || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }
        s.parse().map_err(|_| ())
    };
    let (year, mo, d) = match parts.as_slice() {
        [m, d] => (None, num(*m, 2)?, num(*d, 2)?),
        [y, m, d] if y.len() == 4 => (Some(num(*y, 4)? as i32), num(*m, 2)?, num(*d, 2)?),
        _ => return Err(()),
    };
    NaiveDate::from_ymd_opt(year.unwrap_or(2000), mo, d).ok_or(())?;
    Ok(BirthdayParts { year, month: Some(mo), day: Some(d) })
}

pub fn birthday_input(mo: Option<u32>, d: Option<u32>, y: Option<i32>) -> String {
    match (mo, d) {
        (Some(mo), Some(d)) => match y {
            Some(y) => format!("{y}-{mo:02}-{d:02}"),
            None => format!("{mo:02}-{d:02}"),
        },
        _ => String::new(),
    }
}

pub fn today_in(tz: Tz) -> NaiveDate {
    Utc::now().with_timezone(&tz).date_naive()
}

pub fn next_birthday(mo: u32, d: u32, today: NaiveDate) -> NaiveDate {
    for yr in [today.year(), today.year() + 1] {
        let cand = NaiveDate::from_ymd_opt(yr, mo, d)
            .or_else(|| if mo == 2 && d == 29 { NaiveDate::from_ymd_opt(yr, 2, 28) } else { None });
        if let Some(c) = cand {
            if c >= today {
                return c;
            }
        }
    }
    today
}

pub fn days_until_birthday(mo: u32, d: u32, tz: Tz) -> i64 {
    let today = today_in(tz);
    (next_birthday(mo, d, today) - today).num_days()
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

pub fn fmt_birthday(mo: u32, d: u32, y: Option<i32>, tz: Tz) -> String {
    let today = today_in(tz);
    let nxt = next_birthday(mo, d, today);
    let days = (nxt - today).num_days();
    let when = if days == 0 { "today".to_owned() } else { format!("in {} day{}", days, if days != 1 { "s" } else { "" }) };
    let age = y.map(|y| format!(" · turns {}", nxt.year() - y)).unwrap_or_default();
    let name = MONTHS.get((mo as usize).wrapping_sub(1)).map(|s| (*s).to_owned()).unwrap_or_else(|| mo.to_string());
    format!("{name} {d}{age} ({when})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(parse_ids("<@123456789012345678> and 987654321098765432, dup 123456789012345678"),
                   vec![123456789012345678, 987654321098765432]);
        assert!(parse_ids("12345").is_empty());
    }

    #[test]
    fn birthdays() {
        assert_eq!(parse_birthday("").unwrap(), BirthdayParts::default());
        assert_eq!(parse_birthday("03-05").unwrap().month, Some(3));
        assert_eq!(parse_birthday("1999-03-05").unwrap().year, Some(1999));
        assert!(parse_birthday("02-30").is_err());
        assert!(parse_birthday("nope").is_err());
        assert!(parse_birthday("02-29").is_ok());
        assert_eq!(birthday_input(Some(3), Some(5), Some(1999)), "1999-03-05");
    }

    #[test]
    fn tags() {
        assert_eq!(norm_tag("  Hello   World "), "hello-world");
    }

    #[test]
    fn leap_day_rolls_to_28() {
        let today = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert_eq!(next_birthday(2, 29, today), NaiveDate::from_ymd_opt(2027, 2, 28).unwrap());
    }
}
