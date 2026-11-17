// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

struct PeopleScreen: View {
    @EnvironmentObject var app: AppModel
    @State private var query = ""
    @State private var relation: String? = nil
    @State private var tag: String? = nil

    private var tags: [String] {
        var counts: [String: Int] = [:]
        for e in app.people { for t in e.person.tags { counts[t, default: 0] += 1 } }
        return counts.sorted { $0.value != $1.value ? $0.value > $1.value : $0.key < $1.key }.map { $0.key }
    }

    private var filtered: [PersonEntry] {
        let q = query.trimmed.lowercased()
        return app.people.filter { e in
            (relation == nil || e.person.relation == relation) &&
            (tag == nil || e.person.tags.contains(tag!)) &&
            (q.isEmpty ||
                e.name.lowercased().contains(q) ||
                (e.profile?.username.lowercased().contains(q) ?? false) ||
                String(e.person.userId).contains(q) ||
                (e.person.notes?.lowercased().contains(q) ?? false) ||
                (e.person.metContext?.lowercased().contains(q) ?? false))
        }
    }

    var body: some View {
        let list = filtered
        VStack(spacing: 0) {
            chipRows
            if let p = app.progress {
                Text(p).font(.footnote).foregroundColor(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 16).padding(.vertical, 6)
            }
            content(list)
        }
        .navigationTitle("People (\(list.count))")
        .navigationBarTitleDisplayMode(.inline)
        .searchable(text: $query, prompt: "Search name, @username, ID, notes")
        .toolbar {
            ToolbarItemGroup(placement: .navigationBarTrailing) {
                Button("Update all") { app.refreshAll(list.map { $0.person.userId }) }
                    .disabled(app.progress != nil || list.isEmpty)
                Button { app.loadPeople() } label: { Image(systemName: "arrow.clockwise") }
                    .accessibilityLabel("Reload list")
            }
        }
    }

    private var chipRows: some View {
        VStack(spacing: 8) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    FilterChip(text: "All", selected: relation == nil) { relation = nil }
                    ForEach(STATUSES.indices, id: \.self) { i in
                        FilterChip(text: STATUSES[i].label, selected: relation == STATUSES[i].key) {
                            relation = (relation == STATUSES[i].key) ? nil : STATUSES[i].key
                        }
                    }
                }
                .padding(.horizontal, 16)
            }
            if !tags.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(tags, id: \.self) { t in
                            FilterChip(text: "#\(t)", selected: tag == t) { tag = (tag == t) ? nil : t }
                        }
                    }
                    .padding(.horizontal, 16)
                }
            }
        }
        .padding(.vertical, 8)
    }

    @ViewBuilder private func content(_ list: [PersonEntry]) -> some View {
        if app.peopleLoading && app.people.isEmpty {
            ProgressView().padding(24)
            Spacer()
        } else if list.isEmpty {
            Text(app.people.isEmpty ? "Nobody on your list yet. Use the Lookup tab to add people." : "Nobody matches that.")
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(24)
            Spacer()
        } else {
            List(list) { e in
                NavigationLink(value: Route.person(e.person.userId)) { PersonRow(e: e) }
            }
            .listStyle(.plain)
            .refreshable { await app.reloadAsync() }
        }
    }
}

struct PersonRow: View {
    let e: PersonEntry

    var body: some View {
        HStack(spacing: 12) {
            Avatar(url: e.profile?.avatar, name: e.name)
            VStack(alignment: .leading, spacing: 2) {
                Text(e.name).font(.headline).lineLimit(1)
                Text((e.profile.map { "@\($0.username) · " } ?? "") + statusLabel(e.person.relation))
                    .font(.subheadline).foregroundColor(.secondary).lineLimit(1)
                if !e.person.tags.isEmpty {
                    Text(e.person.tags.map { "#\($0)" }.joined(separator: " "))
                        .font(.caption).foregroundColor(brand).lineLimit(1)
                }
            }
        }
        .padding(.vertical, 2)
    }
}

struct LookupScreen: View {
    @EnvironmentObject var app: AppModel
    @State private var text = ""
    @State private var save = false
    @State private var relation: String? = nil
    @State private var tag = ""

    var body: some View {
        let ids = parseIds(text)
        let untracked = app.lookupResults.filter { $0.person == nil }.map { $0.userId }

        List {
            Section {
                ZStack(alignment: .topLeading) {
                    TextEditor(text: $text)
                        .frame(minHeight: 90)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    if text.isEmpty {
                        Text("Paste IDs or <@mentions>")
                            .foregroundColor(Color(.placeholderText))
                            .padding(.top, 8).padding(.leading, 5)
                            .allowsHitTesting(false)
                    }
                }
                Text("\(ids.count) ID(s) found · paste as many as you like (max 25)")
                    .font(.caption).foregroundColor(.secondary)

                Toggle("Also add to my list", isOn: $save)
                if save {
                    HStack(spacing: 10) {
                        RelationPicker(current: relation, onPick: { relation = $0 })
                        TextField("Tag (optional)", text: $tag)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                    }
                }
                Button {
                    app.lookup(ids, save: save, relation: relation, tag: tag.trimmed.isEmpty ? nil : tag)
                } label: {
                    Label(app.lookupBusy ? "Looking up…" : "Look up", systemImage: "magnifyingglass")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(ids.isEmpty || app.lookupBusy)

                if untracked.count > 1 {
                    Button {
                        app.saveLookupToList(untracked)
                    } label: {
                        Label("Save all \(untracked.count) to list", systemImage: "person.badge.plus")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                }
            } header: {
                Text("User IDs or <@mentions>")
            }

            if !app.lookupResults.isEmpty {
                Section("Results") {
                    ForEach(app.lookupResults) { r in LookupCard(r: r) }
                }
            }
        }
        .navigationTitle("Lookup")
    }
}

private struct LookupCard: View {
    @EnvironmentObject var app: AppModel
    let r: LookupEntry

    var body: some View {
        NavigationLink(value: Route.person(r.userId)) {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 12) {
                    Avatar(url: r.profile?.avatar, name: r.profile?.displayName ?? "?")
                    VStack(alignment: .leading, spacing: 2) {
                        Text(r.profile?.displayName ?? "Not found").font(.headline)
                        Text((r.profile.map { "@\($0.username) · " } ?? "") + String(r.userId))
                            .font(.caption).foregroundColor(.secondary)
                        Text(r.person.map { "In your list: \(statusLabel($0.relation))" } ?? "Not tracked")
                            .font(.caption).foregroundColor(.secondary)
                    }
                    Spacer(minLength: 0)
                    if r.person == nil {
                        Button { app.saveLookupToList([r.userId]) } label: {
                            Label("Save", systemImage: "plus.circle")
                        }
                        .buttonStyle(.borderless)
                    }
                }
                if !r.changes.isEmpty {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Changed since last check").font(.footnote.weight(.semibold)).foregroundColor(brand)
                        ForEach(r.changes, id: \.self) { c in
                            Text("\(c.key): \(c.from.prefix(60)) → \(c.to.prefix(60))").font(.caption)
                        }
                    }
                }
            }
            .padding(.vertical, 4)
        }
    }
}

private struct Upcoming: Identifiable {
    let days: Int
    let entry: PersonEntry
    var id: Int64 { entry.person.userId }
}

struct BirthdaysScreen: View {
    @EnvironmentObject var app: AppModel
    @State private var days: Double = 60

    var body: some View {
        let zone = app.config.zone
        let upcoming: [Upcoming] = app.people.compactMap { e -> Upcoming? in
            guard let mo = e.person.birthdayMonth, let d = e.person.birthdayDay else { return nil }
            let n = daysUntilBirthday(mo, d, zone: zone)
            return n <= Int(days) ? Upcoming(days: n, entry: e) : nil
        }.sorted { $0.days < $1.days }

        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Next \(Int(days)) days").font(.subheadline.weight(.semibold))
                Slider(value: $days, in: 7...366, step: 1)
            }
            .padding(.horizontal, 16).padding(.vertical, 10)

            if upcoming.isEmpty {
                Text("No birthdays in that window.").foregroundColor(.secondary)
                    .padding(.horizontal, 16).padding(.top, 8)
                Spacer()
            } else {
                List(upcoming) { u in
                    NavigationLink(value: Route.person(u.entry.person.userId)) {
                        HStack(spacing: 12) {
                            Avatar(url: u.entry.profile?.avatar, name: u.entry.name)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(u.entry.name).font(.headline)
                                if let mo = u.entry.person.birthdayMonth, let d = u.entry.person.birthdayDay {
                                    Text(fmtBirthday(mo, d, u.entry.person.birthdayYear, zone: zone))
                                        .font(.subheadline).foregroundColor(.secondary)
                                }
                            }
                        }
                    }
                }
                .listStyle(.plain)
            }
        }
        .navigationTitle("Birthdays")
    }
}
