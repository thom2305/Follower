// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import Foundation

struct Config: Equatable {
    var url: String = ""
    var token: String = ""
    var timezone: String = "America/Toronto"

    var ready: Bool { !url.trimmed.isEmpty && !token.trimmed.isEmpty }
    var zone: TimeZone { TimeZone(identifier: timezone.trimmed) ?? .current }
}

final class ConfigBox {
    var value: Config
    init(_ value: Config) { self.value = value }
}

final class ConfigStore {
    private let d = UserDefaults.standard

    func load() -> Config {
        Config(
            url: d.string(forKey: "url") ?? "",
            token: d.string(forKey: "api_token") ?? "",
            timezone: d.string(forKey: "tz") ?? "America/Toronto"
        )
    }

    func save(_ c: Config) {
        d.set(c.url.trimmed, forKey: "url")
        d.set(c.token.trimmed, forKey: "api_token")
        d.set(c.timezone.trimmed, forKey: "tz")
    }
}
