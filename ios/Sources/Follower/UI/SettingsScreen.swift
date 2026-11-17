// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

struct SettingsScreen: View {
    @EnvironmentObject var app: AppModel
    @State private var url = ""
    @State private var token = ""
    @State private var tz = ""
    @State private var loaded = false

    private var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? ""
    }

    var body: some View {
        Form {
            Section {
                TextField("https://follower.example.com or http://100.x.y.z:8080", text: $url)
                    .keyboardType(.URL)
                    .textContentType(.URL)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                SecureField("API token", text: $token)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                TextField("Timezone (e.g. America/Toronto)", text: $tz)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
            } header: {
                Text("Bot server")
            } footer: {
                Text("The token is stored only on this device. Prefer an https:// URL, or a private network such as Tailscale, since http:// sends the token unencrypted.")
            }

            Section {
                Button {
                    app.saveConfig(Config(url: url, token: token, timezone: tz), test: true)
                } label: {
                    Label("Save & test", systemImage: "checkmark.circle")
                }
                Button {
                    app.saveConfig(Config(url: url, token: token, timezone: tz), test: false)
                } label: {
                    Label("Save", systemImage: "square.and.arrow.down")
                }
            }

            Section {
                LabeledContent("Version", value: version)
                Text("\u{00A9} 2026 Thom (thom2305) \u{00B7} Licensed under the Apache License 2.0")
                    .font(.footnote)
                Link(destination: URL(string: "https://github.com/thom2305/Follower")!) {
                    Label("Source code", systemImage: "chevron.left.forwardslash.chevron.right")
                }
                Link(destination: URL(string: "https://github.com/thom2305/Follower/blob/main/LICENSE")!) {
                    Label("License", systemImage: "doc.text")
                }
            } header: {
                Text("About")
            } footer: {
                Text("Follower uses only Apple's frameworks and bundles no third-party libraries.")
            }
        }
        .navigationTitle("Settings")
        .onAppear {
            if !loaded {
                loaded = true
                url = app.config.url
                token = app.config.token
                tz = app.config.timezone
            }
        }
    }
}
