// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

enum AppTab: String, CaseIterable, Identifiable {
    case people, lookup, birthdays, archive, settings
    var id: String { rawValue }

    var label: String {
        switch self {
        case .people: return "People"
        case .lookup: return "Lookup"
        case .birthdays: return "Birthdays"
        case .archive: return "Archive"
        case .settings: return "Settings"
        }
    }

    var icon: String {
        switch self {
        case .people: return "person.2.fill"
        case .lookup: return "magnifyingglass"
        case .birthdays: return "gift.fill"
        case .archive: return "archivebox.fill"
        case .settings: return "gearshape.fill"
        }
    }
}

enum Route: Hashable {
    case person(Int64)
    case archive(ArchiveMsg)
}

@MainActor
final class AppModel: ObservableObject {
    private let store = ConfigStore()
    private let box: ConfigBox
    let repo: Repo

    @Published private(set) var config: Config
    @Published var tab: AppTab

    @Published private(set) var people: [PersonEntry] = []
    @Published private(set) var peopleLoading = false
    @Published private(set) var progress: String?
    @Published private(set) var allTags: [TagCount] = []

    @Published private(set) var lookupResults: [LookupEntry] = []
    @Published private(set) var lookupBusy = false

    @Published private(set) var archiveResults: [ArchiveMsg]?
    @Published private(set) var archiveBusy = false
    @Published private(set) var authors: [AuthorChoice] = []

    @Published var toast: String?
    private var toastTask: Task<Void, Never>?

    init() {
        let c = ConfigStore().load()
        let b = ConfigBox(c)
        self.box = b
        self.config = c
        self.repo = Repo(Api(config: { b.value }))
        self.tab = c.ready ? .people : .settings
        if c.ready { loadPeople() }
    }

    func run(_ block: @escaping @MainActor () async throws -> Void) {
        Task {
            do { try await block() }
            catch is CancellationError { }
            catch {
                let m = error.localizedDescription
                say(m.isEmpty ? "Something went wrong" : m)
            }
        }
    }

    func say(_ s: String) {
        toast = s
        toastTask?.cancel()
        toastTask = Task {
            try? await Task.sleep(nanoseconds: 3_500_000_000)
            if !Task.isCancelled { toast = nil }
        }
    }

    func saveConfig(_ input: Config, test: Bool) {
        var c = input
        c.url = c.url.trimmed
        c.token = c.token.trimmed
        c.timezone = c.timezone.trimmed
        store.save(c)
        config = c
        box.value = c
        run {
            if test {
                let n = try await self.repo.counts()
                self.say("Connected: \(n.people) people, \(n.archive) archived messages")
                try await self.loadPeopleNow()
            } else {
                self.say("Saved")
            }
        }
    }

    func loadPeople() { run { try await self.loadPeopleNow() } }

    func loadPeopleNow() async throws {
        peopleLoading = true
        defer { peopleLoading = false }
        people = try await repo.listPeople()
        allTags = try await repo.allTags()
    }

    func reloadAsync() async {
        do { try await loadPeopleNow() }
        catch is CancellationError { }
        catch { say(error.localizedDescription) }
    }

    func refreshAll(_ ids: [Int64]) {
        if ids.isEmpty { return }
        run {
            var changed = 0, gone = 0
            do {
                for (i, id) in ids.enumerated() {
                    self.progress = "Refreshing \(i + 1)/\(ids.count)…"
                    let r = try await self.repo.takeSnapshot(id)
                    if r.profile == nil { gone += 1 } else if !r.changes.isEmpty { changed += 1 }
                }
            } catch {
                self.progress = nil
                throw error
            }
            self.progress = nil
            try await self.loadPeopleNow()
            self.say("Refreshed \(ids.count) · \(changed) changed" + (gone > 0 ? " · \(gone) not found" : ""))
        }
    }

    func reloadListQuiet() async throws {
        people = try await repo.listPeople()
        allTags = try await repo.allTags()
    }

    func lookup(_ ids: [Int64], save: Bool, relation: String?, tag: String?) {
        run {
            self.lookupBusy = true
            defer { self.lookupBusy = false }
            let t = tag.map(normTag).flatMap { $0.isEmpty ? nil : $0 }
            self.lookupResults = try await self.repo.lookup(Array(ids.prefix(25)), save: save, relation: relation, tag: t)
            if save { try await self.loadPeopleNow() }
        }
    }

    func saveLookupToList(_ ids: [Int64]) {
        run {
            try await self.repo.saveMany(ids)
            var updated = self.lookupResults
            for i in updated.indices where ids.contains(updated[i].userId) {
                updated[i].person = try await self.repo.getPerson(updated[i].userId)
            }
            self.lookupResults = updated
            try await self.loadPeopleNow()
            self.say("Saved \(ids.count) to your list")
        }
    }

    func loadAuthors() { run { self.authors = try await self.repo.authors() } }

    func searchArchive(_ text: String, _ author: String) {
        run {
            self.archiveBusy = true
            defer { self.archiveBusy = false }
            self.archiveResults = try await self.repo.archiveSearch(text, author)
        }
    }

    func randomArchive(_ author: String, onFound: @escaping (ArchiveMsg) -> Void) {
        run {
            self.archiveBusy = true
            defer { self.archiveBusy = false }
            if let m = try await self.repo.archiveRandom(author) { onFound(m) } else { self.say("Archive is empty.") }
        }
    }

    func deleteArchive(_ id: Int64, onDone: @escaping () -> Void) {
        run {
            _ = try await self.repo.archiveDelete(id)
            onDone()
            self.archiveResults = self.archiveResults?.filter { $0.messageId != id }
            self.say("Deleted (including stored attachments).")
        }
    }

    func attachmentBytes(_ fileId: Int64) async -> Data? {
        do { return try await repo.loadAttachment(fileId) }
        catch is CancellationError { return nil }
        catch { say(error.localizedDescription); return nil }
    }
}
