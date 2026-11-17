// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

struct ArchiveScreen: View {
    @EnvironmentObject var app: AppModel
    @State private var text = ""
    @State private var author = ""
    @State private var randomMsg: ArchiveMsg? = nil
    @State private var showRandom = false

    private var authorOptions: [(key: String?, label: String)] {
        var opts: [(key: String?, label: String)] = [(key: nil, label: "Anyone")]
        for a in app.authors { opts.append((key: String(a.id), label: "\(a.name) (\(a.count))")) }
        return opts
    }

    var body: some View {
        let zone = app.config.zone
        List {
            Section {
                TextField("Text contains", text: $text)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                HStack(spacing: 8) {
                    TextField("Author (name or ID)", text: $author)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    PickerMenu(label: "Pick", options: authorOptions, onPick: { author = $0 ?? "" })
                }
                HStack(spacing: 10) {
                    Button { app.searchArchive(text, author) } label: {
                        Label(app.archiveBusy ? "Searching…" : "Search", systemImage: "magnifyingglass")
                    }
                    .buttonStyle(.borderedProminent)
                    Button {
                        app.randomArchive(author) { m in randomMsg = m; showRandom = true }
                    } label: {
                        Label("Random", systemImage: "shuffle")
                    }
                    .buttonStyle(.bordered)
                }
                .disabled(app.archiveBusy)
            }

            if let results = app.archiveResults {
                Section {
                    ForEach(results) { m in
                        NavigationLink(value: Route.archive(m)) {
                            VStack(alignment: .leading, spacing: 6) {
                                HStack(spacing: 8) {
                                    Avatar(url: m.authorAvatar, name: m.authorName, size: 28)
                                    Text(m.authorName).font(.subheadline.weight(.semibold)).lineLimit(1)
                                    Spacer(minLength: 4)
                                    Text(m.messageTs.pretty(zone)).font(.caption).foregroundColor(.secondary)
                                }
                                Text(m.content.flatMap { $0.trimmed.isEmpty ? nil : $0 } ?? "(no text)")
                                    .font(.body).lineLimit(4)
                                if !m.attachments.isEmpty {
                                    Label("\(m.attachments.count) attachment(s)", systemImage: "paperclip")
                                        .font(.caption).foregroundColor(.secondary)
                                }
                            }
                            .padding(.vertical, 2)
                        }
                    }
                } header: {
                    Text(results.isEmpty ? "Nothing found." : "\(results.count) result(s)")
                }
            }
        }
        .navigationTitle("Archive")
        .navigationDestination(isPresented: $showRandom) {
            if let m = randomMsg { ArchiveDetailScreen(msg: m) }
        }
        .task { if app.config.ready { app.loadAuthors() } }
    }
}

struct ArchiveDetailScreen: View {
    let msg: ArchiveMsg
    @EnvironmentObject var app: AppModel
    @Environment(\.dismiss) private var dismiss
    @Environment(\.openURL) private var openURL
    @State private var confirmDelete = false

    var body: some View {
        let zone = app.config.zone
        List {
            Section {
                HStack(spacing: 12) {
                    Avatar(url: msg.authorAvatar, name: msg.authorName)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(msg.authorName).font(.headline)
                        Text("Sent \(msg.messageTs.pretty(zone))").font(.caption).foregroundColor(.secondary)
                        Text("Saved \(msg.savedAt.pretty(zone))").font(.caption).foregroundColor(.secondary)
                    }
                }
                Text(msg.content.flatMap { $0.trimmed.isEmpty ? nil : $0 } ?? "(no text)")
                    .font(.body)
                    .textSelection(.enabled)
            }

            if !msg.attachments.isEmpty {
                Section("Attachments") {
                    ForEach(msg.attachments) { a in AttachmentCard(a: a) }
                }
            }

            Section {
                if let u = msg.jumpUrl, let url = URL(string: u) {
                    Button { openURL(url) } label: { Label("Open in Discord", systemImage: "arrow.up.right.square") }
                }
                Button(role: .destructive) { confirmDelete = true } label: { Label("Delete", systemImage: "trash") }
            } footer: {
                Text("ID \(String(msg.messageId))")
            }
        }
        .navigationTitle(msg.authorName)
        .navigationBarTitleDisplayMode(.inline)
        .alert("Delete from archive?", isPresented: $confirmDelete) {
            Button("Delete", role: .destructive) { app.deleteArchive(msg.messageId) { dismiss() } }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This also deletes the stored attachments.")
        }
    }
}

private struct AttachmentCard: View {
    let a: Attachment
    @EnvironmentObject var app: AppModel
    @Environment(\.openURL) private var openURL
    @State private var preview: UIImage? = nil
    @State private var loading = false
    @State private var share: ShareItem? = nil

    private var isImage: Bool { a.contentType?.hasPrefix("image/") == true }

    private var info: String {
        let whereText: String
        if !a.stored { whereText = "link only" }
        else if a.compressed { whereText = "stored (compressed to \(humanSize(a.storedSize)))" }
        else { whereText = "stored" }
        return "\(humanSize(a.size)) · \(a.contentType ?? "unknown") · \(whereText)"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(a.filename, systemImage: isImage ? "photo" : "doc")
                .font(.subheadline.weight(.semibold))
            Text(info).font(.caption).foregroundColor(.secondary)
            if let p = preview {
                Image(uiImage: p).resizable().scaledToFit().frame(maxWidth: .infinity)
                    .cornerRadius(8)
            }
            if loading { Text("Loading…").font(.caption).foregroundColor(.secondary) }
            HStack(spacing: 8) {
                if a.stored {
                    if isImage && preview == nil && !loading {
                        Button { Task { await loadPreview() } } label: { Label("Preview", systemImage: "eye") }
                            .buttonStyle(.bordered)
                    }
                    Button { Task { await saveToDevice() } } label: {
                        Label("Save to device", systemImage: "square.and.arrow.down")
                    }
                    .buttonStyle(.bordered)
                } else if let u = a.url, let url = URL(string: u) {
                    Button { openURL(url) } label: { Label("Open original link", systemImage: "link") }
                        .buttonStyle(.bordered)
                }
            }
        }
        .padding(.vertical, 4)
        .task {
            if a.stored && isImage && a.storedSize <= 4 * 1024 * 1024 { await loadPreview() }
        }
        .sheet(item: $share) { item in ShareSheet(items: [item.url]) }
    }

    private func loadPreview() async {
        loading = true
        if let data = await app.attachmentBytes(a.id) { preview = downsampledImage(data) }
        loading = false
    }

    private func saveToDevice() async {
        guard let data = await app.attachmentBytes(a.id) else { return }
        let safe = a.filename.replacingOccurrences(of: "/", with: "_")
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(safe.isEmpty ? "attachment" : safe)
        do {
            try data.write(to: url)
            share = ShareItem(url: url)
        } catch {
            app.say("Couldn't save: \(error.localizedDescription)")
        }
    }
}
