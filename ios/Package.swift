// swift-tools-version: 5.9
// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import PackageDescription

let package = Package(
    name: "Follower",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "Follower", targets: ["Follower"]),
    ],
    targets: [
        .target(name: "Follower"),
    ]
)
