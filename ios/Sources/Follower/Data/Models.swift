// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import Foundation

typealias JSON = [String: Any]

extension Dictionary where Key == String, Value == Any {
    func str(_ k: String) -> String? { self[k] as? String }
    func int(_ k: String) -> Int? { (self[k] as? NSNumber)?.intValue }
    func bool(_ k: String, _ def: Bool = false) -> Bool { (self[k] as? NSNumber)?.boolValue ?? def }
    func obj(_ k: String) -> JSON? { self[k] as? JSON }
    func objs(_ k: String) -> [JSON] { (self[k] as? [Any])?.compactMap { $0 as? JSON } ?? [] }
    func strings(_ k: String) -> [String] { (self[k] as? [Any])?.compactMap { $0 as? String } ?? [] }
    func id64(_ k: String) -> Int64? {
        if let s = self[k] as? String { return Int64(s) }
        if let n = self[k] as? NSNumber { return n.int64Value }
        return nil
    }
    func date(_ k: String) -> Date? { str(k).flatMap(parseISODate) }
}

private func stringify(_ x: Any?) -> String {
    guard let x = x, !(x is NSNull) else { return "" }
    if let s = x as? String { return s }
    if let n = x as? NSNumber {
        if CFGetTypeID(n) == CFBooleanGetTypeID() { return n.boolValue ? "true" : "false" }
        return n.stringValue
    }
    return "\(x)"
}

struct Change: Hashable {
    let key: String
    let from: String
    let to: String
}

func parseChanges(_ o: JSON?) -> [Change] {
    guard let o = o else { return [] }
    return o.keys.sorted().map { k in
        let a = o[k] as? [Any] ?? []
        return Change(key: k, from: stringify(a.count > 0 ? a[0] : nil), to: stringify(a.count > 1 ? a[1] : nil))
    }
}

struct Profile: Hashable {
    let username: String
    let globalName: String?
    let avatar: String?
    let banner: String?
    let accentColor: Int?
    let avatarDecoration: String?
    let bot: Bool
    let badges: [String]
    let createdAt: String

    var displayName: String { globalName ?? username }

    static func from(_ o: JSON) -> Profile {
        Profile(
            username: o.str("username") ?? "",
            globalName: o.str("global_name"),
            avatar: o.str("avatar"),
            banner: o.str("banner"),
            accentColor: o.int("accent_color"),
            avatarDecoration: o.str("avatar_decoration"),
            bot: o.bool("bot"),
            badges: o.strings("badges"),
            createdAt: o.str("created_at") ?? ""
        )
    }
}

struct Person: Hashable {
    let userId: Int64
    let relation: String
    let notes: String?
    let metContext: String?
    let birthdayMonth: Int?
    let birthdayDay: Int?
    let birthdayYear: Int?
    let firstSeen: Date
    let updatedAt: Date
    let tags: [String]

    static func from(_ o: JSON) -> Person? {
        guard let id = o.id64("user_id") else { return nil }
        return Person(
            userId: id,
            relation: o.str("relation") ?? "other",
            notes: o.str("notes"),
            metContext: o.str("met_context"),
            birthdayMonth: o.int("birthday_month"),
            birthdayDay: o.int("birthday_day"),
            birthdayYear: o.int("birthday_year"),
            firstSeen: o.date("first_seen") ?? Date(),
            updatedAt: o.date("updated_at") ?? Date(),
            tags: o.strings("tags")
        )
    }
}

struct PersonEntry: Identifiable, Hashable {
    let person: Person
    let profile: Profile?
    var id: Int64 { person.userId }
    var name: String { profile?.displayName ?? String(person.userId) }
}

struct SnapResult {
    let profile: Profile?
    let changes: [Change]
    let isFirst: Bool
}

struct StatusEntry: Hashable { let status: String; let at: Date }
struct ProfileChange: Hashable { let at: Date; let changes: [Change]; let isFirst: Bool }
struct PersonHistory { let statuses: [StatusEntry]; let profileChanges: [ProfileChange] }

struct Attachment: Hashable, Identifiable {
    let id: Int64
    let filename: String
    let contentType: String?
    let size: Int64
    let url: String?
    let compressed: Bool
    let storedSize: Int64
    let stored: Bool
}

struct ArchiveMsg: Hashable, Identifiable {
    let messageId: Int64
    let authorId: Int64
    let authorName: String
    let authorAvatar: String?
    let content: String?
    let jumpUrl: String?
    let messageTs: Date
    let savedAt: Date
    let attachments: [Attachment]
    var id: Int64 { messageId }

    static func from(_ o: JSON) -> ArchiveMsg? {
        guard let mid = o.id64("message_id") else { return nil }
        return ArchiveMsg(
            messageId: mid,
            authorId: o.id64("author_id") ?? 0,
            authorName: o.str("author_name") ?? "?",
            authorAvatar: o.str("author_avatar"),
            content: o.str("content"),
            jumpUrl: o.str("jump_url"),
            messageTs: o.date("message_ts") ?? Date(),
            savedAt: o.date("saved_at") ?? Date(),
            attachments: o.objs("attachments").compactMap { a in
                guard let aid = a.id64("id") else { return nil }
                return Attachment(
                    id: aid,
                    filename: a.str("filename") ?? "file",
                    contentType: a.str("content_type"),
                    size: a.id64("size") ?? 0,
                    url: a.str("url"),
                    compressed: a.bool("compressed"),
                    storedSize: a.id64("stored_size") ?? 0,
                    stored: a.bool("stored")
                )
            }
        )
    }
}

struct AuthorChoice: Hashable, Identifiable { let id: Int64; let name: String; let count: Int }
struct TagCount: Hashable { let tag: String; let count: Int }

struct LookupEntry: Identifiable {
    let userId: Int64
    var profile: Profile?
    var changes: [Change]
    var person: Person?
    var id: Int64 { userId }
}
