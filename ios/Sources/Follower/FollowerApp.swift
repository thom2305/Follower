// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI

let brand = Color(UIColor { t in
    t.userInterfaceStyle == .dark
        ? UIColor(red: 0xD0 / 255, green: 0xBC / 255, blue: 0xFF / 255, alpha: 1)
        : UIColor(red: 0x66 / 255, green: 0x50 / 255, blue: 0xA4 / 255, alpha: 1)
})

@main
struct FollowerApp: App {
    @StateObject private var app = AppModel()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(app)
                .tint(brand)
        }
    }
}

struct RootView: View {
    @EnvironmentObject var app: AppModel
    @Environment(\.horizontalSizeClass) private var sizeClass

    var body: some View {
        Group {
            if sizeClass == .regular { sidebarLayout } else { tabLayout }
        }
        .overlay(alignment: .bottom) { toastView }
        .animation(.easeInOut(duration: 0.2), value: app.toast)
    }

    private var tabLayout: some View {
        TabView(selection: $app.tab) {
            ForEach(AppTab.allCases) { t in
                TabScreen(tab: t)
                    .tabItem { Label(t.label, systemImage: t.icon) }
                    .tag(t)
            }
        }
    }

    private var sidebarLayout: some View {
        NavigationSplitView {
            List(selection: sidebarSelection) {
                ForEach(AppTab.allCases) { t in
                    Label(t.label, systemImage: t.icon).tag(t)
                }
            }
            .navigationTitle("Follower")
        } detail: {
            TabScreen(tab: app.tab).id(app.tab)
        }
    }

    private var sidebarSelection: Binding<AppTab?> {
        Binding(get: { app.tab }, set: { if let t = $0 { app.tab = t } })
    }

    @ViewBuilder private var toastView: some View {
        if let t = app.toast {
            Text(t)
                .font(.subheadline)
                .multilineTextAlignment(.leading)
                .padding(.horizontal, 16).padding(.vertical, 12)
                .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
                .shadow(radius: 8, y: 2)
                .padding(.horizontal, 20)
                .padding(.bottom, sizeClass == .regular ? 24 : 70)
                .frame(maxWidth: 560)
                .transition(.move(edge: .bottom).combined(with: .opacity))
                .onTapGesture { app.toast = nil }
        }
    }
}

struct TabScreen: View {
    let tab: AppTab

    var body: some View {
        NavigationStack {
            content
                .navigationDestination(for: Route.self) { r in
                    switch r {
                    case .person(let id): PersonScreen(userId: id)
                    case .archive(let m): ArchiveDetailScreen(msg: m)
                    }
                }
        }
    }

    @ViewBuilder private var content: some View {
        switch tab {
        case .people: PeopleScreen()
        case .lookup: LookupScreen()
        case .birthdays: BirthdaysScreen()
        case .archive: ArchiveScreen()
        case .settings: SettingsScreen()
        }
    }
}
