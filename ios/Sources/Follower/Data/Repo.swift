// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import Foundation

final class Repo {
    private let api: Api
    init(_ api: Api) { self.api = api }

    func listPeople() async throws -> [PersonEntry] {
        try await api.get("/api/people").objs("people").compactMap { o in
            guard let p = Person.from(o) else { return nil }
            return PersonEntry(person: p, profile: o.obj("profile").map(Profile.from))
        }
    }

    func getDetail(_ id: Int64) async throws -> (Person?, Profile?) {
        let o = try await api.get("/api/people/\(id)")
        return (o.obj("person").flatMap(Person.from), o.obj("profile").map(Profile.from))
    }

    func getPerson(_ id: Int64) async throws -> Person? { try await getDetail(id).0 }

    func ensurePerson(_ id: Int64) async throws { _ = try await api.post("/api/people/\(id)") }

    func setRelation(_ id: Int64, _ relation: String) async throws {
        _ = try await api.post("/api/people/\(id)", ["relation": relation])
    }

    func updateDetails(_ id: Int64, notes: String?, met: String?, year: Int?, month: Int?, day: Int?) async throws {
        _ = try await api.post("/api/people/\(id)", [
            "notes": notes ?? "",
            "met_context": met ?? "",
            "birthday": birthdayInput(month, day, year),
        ])
    }

    func addTag(_ id: Int64, _ tag: String) async throws {
        _ = try await api.post("/api/people/\(id)/tags", ["tag": tag])
    }

    func removeTag(_ id: Int64, _ tag: String) async throws {
        _ = try await api.delete("/api/people/\(id)/tags/\(enc(tag))")
    }

    func removePerson(_ id: Int64) async throws { _ = try await api.delete("/api/people/\(id)") }

    func saveMany(_ ids: [Int64]) async throws {
        _ = try await api.post("/api/people/save", ["ids": ids.map { String($0) }])
    }

    func allTags() async throws -> [TagCount] {
        try await api.get("/api/tags").objs("tags").compactMap { o in
            guard let t = o.str("tag") else { return nil }
            return TagCount(tag: t, count: o.int("count") ?? 0)
        }
    }

    func counts() async throws -> (people: Int, archive: Int) {
        let o = try await api.get("/api/ping")
        return (o.int("people") ?? 0, o.int("archive") ?? 0)
    }

    private func snapOf(_ o: JSON) -> SnapResult {
        SnapResult(
            profile: o.bool("found") ? o.obj("profile").map(Profile.from) : nil,
            changes: parseChanges(o.obj("changes")),
            isFirst: o.bool("is_first")
        )
    }

    func takeSnapshot(_ id: Int64) async throws -> SnapResult {
        snapOf(try await api.post("/api/people/\(id)/refresh"))
    }

    func history(_ id: Int64) async throws -> PersonHistory {
        let o = try await api.get("/api/people/\(id)/history")
        return PersonHistory(
            statuses: o.objs("statuses").map { StatusEntry(status: $0.str("status") ?? "", at: $0.date("at") ?? Date()) },
            profileChanges: o.objs("profile_changes").map {
                ProfileChange(at: $0.date("at") ?? Date(), changes: parseChanges($0.obj("changes")), isFirst: $0.bool("is_first"))
            }
        )
    }

    func lookup(_ ids: [Int64], save: Bool, relation: String?, tag: String?) async throws -> [LookupEntry] {
        var body: JSON = ["ids": ids.map { String($0) }, "save": save]
        if let r = relation { body["relation"] = r }
        if let t = tag { body["tag"] = t }
        return try await api.post("/api/lookup", body).objs("results").compactMap { o in
            guard let uid = o.id64("user_id") else { return nil }
            let s = snapOf(o)
            return LookupEntry(userId: uid, profile: s.profile, changes: s.changes, person: o.obj("person").flatMap(Person.from))
        }
    }

    func archiveSearch(_ text: String?, _ author: String?) async throws -> [ArchiveMsg] {
        try await api.get("/api/archive?text=\(enc(text ?? ""))&author=\(enc(author ?? ""))")
            .objs("messages").compactMap(ArchiveMsg.from)
    }

    func archiveRandom(_ author: String?) async throws -> ArchiveMsg? {
        try await api.get("/api/archive/random?author=\(enc(author ?? ""))").obj("message").flatMap(ArchiveMsg.from)
    }

    func archiveDelete(_ messageId: Int64) async throws -> Bool {
        try await api.delete("/api/archive/\(messageId)").bool("deleted")
    }

    func authors() async throws -> [AuthorChoice] {
        try await api.get("/api/archive/authors").objs("authors").compactMap { o in
            guard let id = o.id64("id") else { return nil }
            return AuthorChoice(id: id, name: o.str("name") ?? "?", count: o.int("count") ?? 0)
        }
    }

    func loadAttachment(_ fileId: Int64) async throws -> Data {
        try await api.bytes("GET", "/api/archive/files/\(fileId)")
    }
}
