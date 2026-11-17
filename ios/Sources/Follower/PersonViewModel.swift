// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

@MainActor
final class PersonViewModel: ObservableObject {
    @Published var profile: Profile?
    @Published var person: Person?
    @Published var changes: [Change] = []
    @Published var refreshing = true
    @Published var gone = false
    @Published var history: PersonHistory?

    private weak var app: AppModel?
    private var id: Int64 = 0
    private var started = false

    func start(app: AppModel, id: Int64) {
        guard !started else { return }
        started = true
        self.app = app
        self.id = id
        app.run {
            do {
                let (p, pr) = try await app.repo.getDetail(id)
                self.person = p
                self.profile = pr
            } catch {
                self.refreshing = false
                throw error
            }
            await self.refreshNow()
        }
    }

    func refresh() { Task { await refreshNow() } }

    private func refreshNow() async {
        guard let app = app else { return }
        refreshing = true
        do {
            let r = try await app.repo.takeSnapshot(id)
            if let p = r.profile { profile = p }
            changes = r.changes
            gone = r.profile == nil
        } catch is CancellationError {
        } catch {
            app.say(error.localizedDescription + " (showing the last saved profile)")
        }
        refreshing = false
    }

    private func afterChange() async throws {
        guard let app = app else { return }
        person = try await app.repo.getPerson(id)
        try await app.reloadListQuiet()
    }

    func setRelation(_ r: String) {
        guard let app = app else { return }
        app.run { try await app.repo.setRelation(self.id, r); try await self.afterChange() }
    }

    func addTags(_ raw: String) {
        guard let app = app else { return }
        let tags = Array(Set(raw.split(separator: ",").map { normTag(String($0)) }.filter { !$0.isEmpty })).sorted()
        app.run {
            for t in tags { try await app.repo.addTag(self.id, t) }
            try await self.afterChange()
        }
    }

    func removeTag(_ t: String) {
        guard let app = app else { return }
        app.run { try await app.repo.removeTag(self.id, t); try await self.afterChange() }
    }

    func addToList() {
        guard let app = app else { return }
        app.run { try await app.repo.ensurePerson(self.id); try await self.afterChange() }
    }

    func saveDetails(notes: String, met: String, birthday: BirthdayParts) {
        guard let app = app else { return }
        let n = notes.trimmed
        let m = String(met.trimmed.prefix(60))
        app.run {
            try await app.repo.updateDetails(self.id, notes: n.isEmpty ? nil : n, met: m.isEmpty ? nil : m,
                                             year: birthday.year, month: birthday.month, day: birthday.day)
            try await self.afterChange()
        }
    }

    func remove(onDone: @escaping () -> Void) {
        guard let app = app else { return }
        app.run {
            try await app.repo.removePerson(self.id)
            onDone()
            try await app.reloadListQuiet()
            app.say("Removed from your list (profile snapshots kept).")
        }
    }

    func loadHistory() {
        guard let app = app else { return }
        app.run { self.history = try await app.repo.history(self.id) }
    }
}
