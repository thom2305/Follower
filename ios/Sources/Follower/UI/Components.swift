// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

import SwiftUI
import UIKit
import ImageIO

struct Avatar: View {
    let url: String?
    let name: String
    var size: CGFloat = 44

    var body: some View {
        Group {
            if let s = url, let u = URL(string: s) {
                AsyncImage(url: u) { phase in
                    if let img = phase.image {
                        img.resizable().scaledToFill()
                    } else {
                        initial
                    }
                }
            } else {
                initial
            }
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
    }

    private var initial: some View {
        ZStack {
            Circle().fill(brand.opacity(0.18))
            Text(String(name.prefix(1)).uppercased())
                .font(.system(size: size * 0.42, weight: .semibold))
                .foregroundColor(brand)
        }
    }
}

struct FilterChip: View {
    let text: String
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(text)
                .font(.subheadline)
                .lineLimit(1)
                .padding(.horizontal, 12).padding(.vertical, 7)
                .background(Capsule().fill(selected ? brand : Color(.secondarySystemFill)))
                .foregroundColor(selected ? Color(.systemBackground) : .primary)
        }
        .buttonStyle(.plain)
    }
}

struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxW = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowH: CGFloat = 0, width: CGFloat = 0
        for s in subviews {
            let sz = s.sizeThatFits(.unspecified)
            if x > 0 && x + sz.width > maxW { x = 0; y += rowH + spacing; rowH = 0 }
            x += sz.width + spacing
            rowH = max(rowH, sz.height)
            width = max(width, x - spacing)
        }
        return CGSize(width: width, height: y + rowH)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowH: CGFloat = 0
        for s in subviews {
            let sz = s.sizeThatFits(.unspecified)
            if x > bounds.minX && x + sz.width > bounds.maxX { x = bounds.minX; y += rowH + spacing; rowH = 0 }
            s.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(sz))
            x += sz.width + spacing
            rowH = max(rowH, sz.height)
        }
    }
}

struct TagChips: View {
    let tags: [String]
    var onRemove: ((String) -> Void)? = nil
    var onTap: ((String) -> Void)? = nil

    var body: some View {
        FlowLayout(spacing: 8) {
            ForEach(tags, id: \.self) { t in
                Button {
                    if let r = onRemove { r(t) } else { onTap?(t) }
                } label: {
                    HStack(spacing: 4) {
                        Text(t)
                        if onRemove != nil {
                            Image(systemName: "xmark.circle.fill").imageScale(.small)
                        }
                    }
                    .font(.footnote)
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(Capsule().fill(brand.opacity(0.15)))
                    .foregroundColor(brand)
                }
                .buttonStyle(.borderless)
            }
        }
    }
}

struct PickerMenu: View {
    let label: String
    let options: [(key: String?, label: String)]
    let onPick: (String?) -> Void

    var body: some View {
        Menu {
            ForEach(options.indices, id: \.self) { i in
                Button(options[i].label) { onPick(options[i].key) }
            }
        } label: {
            HStack(spacing: 6) {
                Text(label).lineLimit(1)
                Image(systemName: "chevron.up.chevron.down").font(.caption2)
            }
            .padding(.horizontal, 12).padding(.vertical, 8)
            .background(RoundedRectangle(cornerRadius: 10).stroke(brand.opacity(0.5)))
        }
    }
}

struct RelationPicker: View {
    let current: String?
    let onPick: (String) -> Void
    var placeholder: String = "Relation"

    var body: some View {
        PickerMenu(
            label: current.map(statusLabel) ?? placeholder,
            options: STATUSES.map { (key: $0.key as String?, label: $0.label) },
            onPick: { if let k = $0 { onPick(k) } }
        )
    }
}

private func shorten(_ s: String, _ max: Int = 60) -> String {
    s.count > max ? String(s.prefix(max)) + "…" : s
}

struct ChangesSection: View {
    let changes: [Change]

    var body: some View {
        if !changes.isEmpty {
            Section("Changed since last check") {
                ForEach(changes, id: \.self) { c in
                    Text("\(c.key): \(shorten(c.from)) → \(shorten(c.to))").font(.footnote)
                }
            }
        }
    }
}

struct ShareItem: Identifiable {
    let id = UUID()
    let url: URL
}

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }
    func updateUIViewController(_ vc: UIActivityViewController, context: Context) {}
}

func downsampledImage(_ data: Data, maxPixel: Int = 2048) -> UIImage? {
    let srcOpts: [CFString: Any] = [kCGImageSourceShouldCache: false]
    guard let src = CGImageSourceCreateWithData(data as CFData, srcOpts as CFDictionary) else { return nil }
    let opts: [CFString: Any] = [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceShouldCacheImmediately: true,
        kCGImageSourceThumbnailMaxPixelSize: maxPixel,
    ]
    guard let cg = CGImageSourceCreateThumbnailAtIndex(src, 0, opts as CFDictionary) else { return nil }
    return UIImage(cgImage: cg)
}
