// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

struct PersonScreen: View {
    let userId: Int64
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    @StateObject private var vm = PersonViewModel()
    @State private var showEdit = false
    @State private var confirmRemove = false
    @State private var tagText = ""

    var body: some View {
        let zone = app.config.zone
        List {
            headerSection(zone)
            ChangesSection(changes: vm.changes)
            relationSection(zone)
            if let person = vm.person {
                tagsSection(person)
                detailsSection(person, zone)
            }
            historySection(zone)
            if vm.person != nil {
                Section {
                    Button(role: .destructive) { confirmRemove = true } label: {
                        Label("Remove from list", systemImage: "trash")
                    }
                }
            }
        }
        .navigationTitle(vm.profile?.displayName ?? String(userId))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                if vm.refreshing {
                    ProgressView()
                } else {
                    Button { vm.refresh() } label: { Image(systemName: "arrow.clockwise") }
                        .accessibilityLabel("Refresh from Discord")
                }
            }
        }
        .task { vm.start(app: app, id: userId) }
        .sheet(isPresented: $showEdit) {
            if let person = vm.person {
                EditDetailsSheet(person: person) { notes, met, bday in
                    vm.saveDetails(notes: notes, met: met, birthday: bday)
                }
            }
        }
        .alert("Remove from list?", isPresented: $confirmRemove) {
            Button("Remove", role: .destructive) { vm.remove { dismiss() } }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Tags and details will be deleted. Profile snapshots are kept.")
        }
    }

    @ViewBuilder private func headerSection(_ zone: TimeZone) -> some View {
        let p = vm.profile
        Section {
            if let b = p?.banner, let u = URL(string: b) {
                AsyncImage(url: u) { phase in
                    if let img = phase.image {
                        img.resizable().scaledToFill()
                    } else {
                        Color(.secondarySystemFill)
                    }
                }
                .frame(height: 110)
                .frame(maxWidth: .infinity)
                .clipped()
                .listRowInsets(EdgeInsets())
            }
            HStack(spacing: 14) {
                Avatar(url: p?.avatar, name: p?.displayName ?? "?", size: 64)
                VStack(alignment: .leading, spacing: 2) {
                    Text(p?.displayName ?? String(userId)).font(.title2.weight(.semibold))
                    if let p = p { Text("@\(p.username)").font(.body).foregroundColor(.secondary) }
                    Text(String(userId)).font(.caption).foregroundColor(.secondary).textSelection(.enabled)
                }
            }
            if vm.gone {
                Text("Couldn't fetch this account from Discord (deleted or unavailable). Showing the last saved profile.")
                    .font(.footnote).foregroundColor(.red)
            }
            if let p = p {
                LabeledContent("Created", value: parseISODate(p.createdAt)?.pretty(zone) ?? p.createdAt)
                LabeledContent("Badges", value: p.badges.isEmpty ? "none" : p.badges.joined(separator: ", "))
                if p.bot { LabeledContent("Bot", value: "yes") }
            }
        }
    }

    @ViewBuilder private func relationSection(_ zone: TimeZone) -> some View {
        Section("Relation") {
            if let person = vm.person {
                HStack(spacing: 12) {
                    RelationPicker(current: person.relation, onPick: { vm.setRelation($0) })
                    Text("Tracked since \(person.firstSeen.prettyDate(zone))")
                        .font(.caption).foregroundColor(.secondary)
                }
            } else {
                Text("Not on your list yet.").foregroundColor(.secondary)
                Button { vm.addToList() } label: { Label("Add to list", systemImage: "plus") }
                    .buttonStyle(.borderedProminent)
            }
        }
    }

    @ViewBuilder private func tagsSection(_ person: Person) -> some View {
        Section("Tags") {
            if !person.tags.isEmpty {
                TagChips(tags: person.tags, onRemove: { vm.removeTag($0) })
            }
            HStack(spacing: 8) {
                TextField("Add tag(s), comma-separated", text: $tagText)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                Button {
                    vm.addTags(tagText)
                    tagText = ""
                } label: {
                    Label("Add", systemImage: "plus")
                }
                .buttonStyle(.borderless)
                .disabled(tagText.trimmed.isEmpty)
            }
            let suggestions = app.allTags.map { $0.tag }.filter { !person.tags.contains($0) }.prefix(12)
            if !suggestions.isEmpty {
                Text("Existing tags").font(.caption).foregroundColor(.secondary)
                TagChips(tags: Array(suggestions), onTap: { vm.addTags($0) })
            }
        }
    }

    @ViewBuilder private func detailsSection(_ person: Person, _ zone: TimeZone) -> some View {
        Section("Details") {
            if let mo = person.birthdayMonth, let d = person.birthdayDay {
                LabeledContent("Birthday", value: fmtBirthday(mo, d, person.birthdayYear, zone: zone))
            }
            if let met = person.metContext { LabeledContent("Know them from", value: met) }
            if let notes = person.notes { LabeledContent("Notes", value: notes) }
            Button { showEdit = true } label: { Label("Edit details", systemImage: "pencil") }
        }
    }

    @ViewBuilder private func historySection(_ zone: TimeZone) -> some View {
        Section("History") {
            if let h = vm.history {
                if !h.statuses.isEmpty {
                    Text("Relation log").font(.subheadline.weight(.semibold))
                    ForEach(h.statuses, id: \.self) { s in
                        Text("\(statusLabel(s.status)) · \(s.at.pretty(zone))").font(.footnote)
                    }
                }
                Text("Profile changes").font(.subheadline.weight(.semibold))
                if h.profileChanges.isEmpty {
                    Text("No snapshots yet.").font(.footnote).foregroundColor(.secondary)
                }
                ForEach(h.profileChanges, id: \.self) { c in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(c.at.pretty(zone) + (c.isFirst ? " · first snapshot" : ""))
                            .font(.footnote).foregroundColor(brand)
                        ForEach(c.changes, id: \.self) { ch in
                            Text("\(ch.key): \(ch.from.prefix(40)) → \(ch.to.prefix(40))").font(.footnote)
                        }
                    }
                    .padding(.vertical, 2)
                }
            } else {
                Button { vm.loadHistory() } label: { Label("Show history", systemImage: "clock.arrow.circlepath") }
            }
        }
    }
}

private struct EditDetailsSheet: View {
    let person: Person
    let onSave: (String, String, BirthdayParts) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var notes: String
    @State private var met: String
    @State private var bd: String
    @State private var errorText: String? = nil

    init(person: Person, onSave: @escaping (String, String, BirthdayParts) -> Void) {
        self.person = person
        self.onSave = onSave
        _notes = State(initialValue: person.notes ?? "")
        _met = State(initialValue: person.metContext ?? "")
        _bd = State(initialValue: birthdayInput(person.birthdayMonth, person.birthdayDay, person.birthdayYear))
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Notes") {
                    TextEditor(text: $notes).frame(minHeight: 110)
                        .onChange(of: notes) { v in if v.count > 1000 { notes = String(v.prefix(1000)) } }
                }
                Section("Where you know them from") {
                    TextField("e.g. school, a Discord server", text: $met)
                        .onChange(of: met) { v in if v.count > 60 { met = String(v.prefix(60)) } }
                }
                Section {
                    TextField("YYYY-MM-DD or MM-DD", text: $bd)
                        .keyboardType(.numbersAndPunctuation)
                        .onChange(of: bd) { v in
                            errorText = nil
                            if v.count > 10 { bd = String(v.prefix(10)) }
                        }
                } header: {
                    Text("Birthday")
                } footer: {
                    if let e = errorText { Text(e).foregroundColor(.red) }
                }
            }
            .navigationTitle("Person details")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        do {
                            let parts = try parseBirthday(bd)
                            onSave(notes, met, parts)
                            dismiss()
                        } catch {
                            errorText = "Use 1999-03-05 or 03-05"
                        }
                    }
                }
            }
        }
    }
}
