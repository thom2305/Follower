// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import Foundation

struct ApiError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

final class Api {
    private let config: () -> Config
    private let session: URLSession

    init(config: @escaping () -> Config) {
        self.config = config
        let cfg = URLSessionConfiguration.default
        cfg.timeoutIntervalForRequest = 60
        cfg.requestCachePolicy = .reloadIgnoringLocalCacheData
        self.session = URLSession(configuration: cfg)
    }

    func get(_ path: String) async throws -> JSON { try await json("GET", path) }
    func post(_ path: String, _ body: JSON = [:]) async throws -> JSON { try await json("POST", path, body) }
    func delete(_ path: String) async throws -> JSON { try await json("DELETE", path) }

    private func json(_ method: String, _ path: String, _ body: JSON? = nil) async throws -> JSON {
        let data = try await bytes(method, path, body)
        guard let o = (try? JSONSerialization.jsonObject(with: data)) as? JSON else {
            throw ApiError("The server sent something that isn't JSON.")
        }
        return o
    }

    func bytes(_ method: String, _ path: String, _ body: JSON? = nil) async throws -> Data {
        let c = config()
        guard c.ready else { throw ApiError("Set the server URL and API token in Settings first.") }
        var base = c.url.trimmed
        while base.hasSuffix("/") { base.removeLast() }
        if !base.hasPrefix("http://") && !base.hasPrefix("https://") { base = "https://" + base }
        guard let url = URL(string: base + path) else { throw ApiError("That server URL doesn't look valid.") }

        var req = URLRequest(url: url)
        req.httpMethod = method
        req.timeoutInterval = 60
        req.setValue("Bearer \(c.token.trimmed)", forHTTPHeaderField: "Authorization")
        req.setValue("application/json", forHTTPHeaderField: "Accept")
        var payload: JSON? = body
        if payload == nil && method == "POST" { payload = [:] }
        if let payload = payload {
            req.httpBody = try JSONSerialization.data(withJSONObject: payload)
            req.setValue("application/json; charset=utf-8", forHTTPHeaderField: "Content-Type")
        }

        let data: Data
        let resp: URLResponse
        do {
            (data, resp) = try await session.data(for: req)
        } catch let e as URLError {
            if e.code == .cancelled { throw CancellationError() }
            throw ApiError("Couldn't reach the server: \(e.localizedDescription)")
        }

        let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
        if (200...299).contains(code) { return data }

        let msg = ((try? JSONSerialization.jsonObject(with: data)) as? JSON)?.str("error")
        if code == 401 { throw ApiError("Wrong API token (401).") }
        if let msg = msg { throw ApiError(msg) }
        throw ApiError("Server returned HTTP \(code).")
    }
}
