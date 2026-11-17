// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import Foundation

extension String {
    var trimmed: String { trimmingCharacters(in: .whitespacesAndNewlines) }
}

func enc(_ s: String) -> String {
    let ok = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")
    return s.addingPercentEncoding(withAllowedCharacters: ok) ?? s
}

let STATUSES: [(key: String, label: String)] = [
    ("friend", "Friend"),
    ("unfriended", "I unfriended them"),
    ("unfriended_me", "They unfriended me"),
    ("blocked", "I blocked them"),
    ("blocked_me", "They blocked me"),
    ("acquaintance", "Acquaintance"),
    ("other", "Other"),
]

func statusLabel(_ key: String) -> String {
    STATUSES.first(where: { $0.key == key })?.label ?? key
}

func normTag(_ t: String) -> String {
    let s = t.trimmed.lowercased().replacingOccurrences(of: "\\s+", with: "-", options: .regularExpression)
    return String(s.prefix(32))
}

func parseIds(_ text: String?) -> [Int64] {
    let t = text ?? ""
    guard let re = try? NSRegularExpression(pattern: "\\d{15,20}") else { return [] }
    var seen = Set<Int64>()
    var out: [Int64] = []
    for m in re.matches(in: t, range: NSRange(t.startIndex..., in: t)) {
        guard let r = Range(m.range, in: t), let v = Int64(t[r]) else { continue }
        if seen.insert(v).inserted { out.append(v) }
    }
    return out
}

func humanSize(_ bytes: Int64) -> String {
    var n = Double(bytes)
    for unit in ["B", "KB", "MB", "GB"] {
        if n < 1024 || unit == "GB" {
            return unit == "B" ? String(format: "%.0f B", n) : String(format: "%.1f", n) + " " + unit
        }
        n /= 1024
    }
    return "\(bytes) B"
}

func calendar(_ zone: TimeZone) -> Calendar {
    var c = Calendar(identifier: .gregorian)
    c.timeZone = zone
    return c
}

struct BirthdayParts {
    var year: Int? = nil
    var month: Int? = nil
    var day: Int? = nil
}

struct BadBirthday: Error {}

private func validDate(_ y: Int, _ m: Int, _ d: Int) -> Bool {
    var c = DateComponents()
    c.year = y; c.month = m; c.day = d
    let cal = Calendar(identifier: .gregorian)
    guard let date = cal.date(from: c) else { return false }
    let r = cal.dateComponents([.year, .month, .day], from: date)
    return r.year == y && r.month == m && r.day == d
}

func parseBirthday(_ text: String) throws -> BirthdayParts {
    let t = text.trimmed
    if t.isEmpty { return BirthdayParts() }
    guard let re = try? NSRegularExpression(pattern: "^(?:(\\d{4})-)?(\\d{1,2})-(\\d{1,2})$"),
          let m = re.firstMatch(in: t, range: NSRange(t.startIndex..., in: t)) else { throw BadBirthday() }
    func grp(_ i: Int) -> Int? {
        let r = m.range(at: i)
        guard r.location != NSNotFound, let rr = Range(r, in: t) else { return nil }
        return Int(t[rr])
    }
    let y = grp(1)
    guard let mo = grp(2), let d = grp(3) else { throw BadBirthday() }
    guard validDate(y ?? 2000, mo, d) else { throw BadBirthday() }
    return BirthdayParts(year: y, month: mo, day: d)
}

func nextBirthday(_ mo: Int, _ d: Int, zone: TimeZone, now: Date = Date()) -> Date {
    let cal = calendar(zone)
    let start = cal.startOfDay(for: now)
    let y = cal.component(.year, from: start)
    for yr in [y, y + 1] {
        var comps = DateComponents()
        comps.year = yr; comps.month = mo; comps.day = d
        var cand = cal.date(from: comps)
        if cand == nil || cal.component(.month, from: cand!) != mo {
            comps.month = 2; comps.day = 28
            cand = cal.date(from: comps)
        }
        if let c = cand, c >= start { return c }
    }
    return start
}

func daysUntilBirthday(_ mo: Int, _ d: Int, zone: TimeZone, now: Date = Date()) -> Int {
    let cal = calendar(zone)
    let start = cal.startOfDay(for: now)
    let nxt = nextBirthday(mo, d, zone: zone, now: now)
    return cal.dateComponents([.day], from: start, to: nxt).day ?? 0
}

func fmtBirthday(_ mo: Int, _ d: Int, _ y: Int?, zone: TimeZone) -> String {
    let cal = calendar(zone)
    let nxt = nextBirthday(mo, d, zone: zone)
    let days = daysUntilBirthday(mo, d, zone: zone)
    let whenText = days == 0 ? "today" : "in \(days) day\(days != 1 ? "s" : "")"
    let age = y.map { " · turns \(cal.component(.year, from: nxt) - $0)" } ?? ""
    let names = Calendar.current.monthSymbols
    let name = (mo >= 1 && mo <= names.count) ? names[mo - 1] : "\(mo)"
    return "\(name) \(d)\(age) (\(whenText))"
}

func birthdayInput(_ mo: Int?, _ d: Int?, _ y: Int?) -> String {
    guard let mo = mo, let d = d else { return "" }
    return (y.map { "\($0)-" } ?? "") + String(format: "%02d-%02d", mo, d)
}

private let isoFrac: ISO8601DateFormatter = {
    let f = ISO8601DateFormatter()
    f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return f
}()
private let isoPlain = ISO8601DateFormatter()

func parseISODate(_ s: String) -> Date? {
    if let d = isoFrac.date(from: s) { return d }
    if let d = isoPlain.date(from: s) { return d }
    let stripped = s.replacingOccurrences(of: "\\.\\d+", with: "", options: .regularExpression)
    if let d = isoPlain.date(from: stripped) { return d }
    return isoPlain.date(from: stripped + "Z")
}

extension Date {
    func pretty(_ zone: TimeZone) -> String {
        let f = DateFormatter()
        f.dateStyle = .medium; f.timeStyle = .short; f.timeZone = zone
        return f.string(from: self)
    }
    func prettyDate(_ zone: TimeZone) -> String {
        let f = DateFormatter()
        f.dateStyle = .medium; f.timeStyle = .none; f.timeZone = zone
        return f.string(from: self)
    }
}
