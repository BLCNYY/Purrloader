import SwiftUI

/// Capsule segmented control with a sliding white pill. Each segment is a real button,
/// so it works with Full Keyboard Access and VoiceOver.
struct PillSegment<Value: Hashable>: View {
    @Binding var selection: Value
    let options: [(Value, String)]
    var height: CGFloat = 32
    var label: String
    @Namespace private var namespace

    var body: some View {
        HStack(spacing: 2) {
            ForEach(options, id: \.0) { value, title in
                let selected = selection == value
                Button {
                    withAnimation(.snappy(duration: 0.25)) { selection = value }
                } label: {
                    Text(title)
                        .font(.system(size: 13, weight: selected ? .semibold : .medium))
                        .foregroundStyle(selected ? Color.black : Theme.secondary)
                        .padding(.horizontal, 14)
                        .frame(height: height - 4)
                        .background {
                            if selected {
                                Capsule().fill(.white).matchedGeometryEffect(id: "pill", in: namespace)
                            }
                        }
                        .contentShape(Capsule())
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(selected ? .isSelected : [])
            }
        }
        .padding(2)
        .background(Capsule().fill(Theme.surface))
        .overlay(Capsule().strokeBorder(Theme.stroke))
        .fixedSize()
        .accessibilityElement(children: .contain)
        .accessibilityLabel(label)
    }
}

/// The look shared by dropdown chips.
struct ChipLabel: View {
    let text: String
    var icon: String?
    var showsChevron = true

    var body: some View {
        HStack(spacing: 6) {
            if let icon { Image(systemName: icon).font(.system(size: 11, weight: .medium)) }
            Text(text).font(.system(size: 13, weight: .medium)).lineLimit(1)
            if showsChevron {
                Image(systemName: "chevron.down").font(.system(size: 8, weight: .bold)).foregroundStyle(Theme.tertiary)
            }
        }
        .foregroundStyle(.white.opacity(0.9))
        .padding(.horizontal, 13)
        .frame(height: 32)
        .background(Capsule().fill(Theme.surface))
        .overlay(Capsule().strokeBorder(Theme.stroke))
        .contentShape(Capsule())
    }
}

/// A Picker presented as a capsule chip that opens a menu.
struct ChipPicker<Value: Hashable, Content: View>: View {
    /// What VoiceOver calls the chip; the visible text is its value.
    let name: String
    let label: String
    var icon: String?
    @Binding var selection: Value
    @ViewBuilder var content: () -> Content

    var body: some View {
        Menu {
            Picker(label, selection: $selection, content: content)
                .pickerStyle(.inline)
                .labelsHidden()
        } label: {
            ChipLabel(text: label, icon: icon)
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .menuIndicator(.hidden)
        .fixedSize()
        .accessibilityLabel(name)
        .accessibilityValue(label)
    }
}

/// A chip that runs an action instead of opening a menu.
struct ChipButton: View {
    let text: String
    var icon: String?
    var showsChevron = false
    let action: () -> Void

    var body: some View {
        Button(action: action) { ChipLabel(text: text, icon: icon, showsChevron: showsChevron) }
            .buttonStyle(.plain)
            .fixedSize()
    }
}

struct CircleButtonStyle: ButtonStyle {
    var size: CGFloat = 32
    var filled = true

    func makeBody(configuration: Configuration) -> some View {
        CircleButton(configuration: configuration, size: size, filled: filled)
    }

    private struct CircleButton: View {
        let configuration: ButtonStyleConfiguration
        let size: CGFloat
        let filled: Bool
        @State private var hovering = false
        @Environment(\.isEnabled) private var isEnabled

        var body: some View {
            configuration.label
                .font(.system(size: size * 0.42, weight: .semibold))
                .foregroundStyle(.white.opacity(isEnabled ? 0.9 : 0.3))
                .frame(width: size, height: size)
                .background(
                    Circle().fill(.white.opacity(configuration.isPressed ? 0.18 : hovering ? 0.12 : filled ? 0.06 : 0))
                )
                .overlay(Circle().strokeBorder(filled ? Theme.stroke : .clear))
                .contentShape(Circle())
                .onHover { hovering = $0 }
                .animation(.easeOut(duration: 0.12), value: hovering)
        }
    }
}

struct PrimaryCapsuleStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        Primary(configuration: configuration)
    }

    private struct Primary: View {
        let configuration: ButtonStyleConfiguration
        @Environment(\.isEnabled) private var isEnabled

        var body: some View {
            configuration.label
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(.black)
                .padding(.horizontal, 18)
                .frame(height: 32)
                .background(Capsule().fill(.white.opacity(isEnabled ? 1 : 0.25)))
                .scaleEffect(configuration.isPressed ? 0.97 : 1)
                .animation(.easeOut(duration: 0.1), value: configuration.isPressed)
        }
    }
}

/// Thin capsule progress bar; nil shows an indeterminate sweep.
struct SlimProgress: View {
    let fraction: Double?
    var tint: Color = .white

    var body: some View {
        GeometryReader { geo in
            ZStack(alignment: .leading) {
                Capsule().fill(Color.white.opacity(0.08))
                if let fraction {
                    Capsule().fill(tint)
                        .frame(width: max(4, geo.size.width * fraction))
                        .animation(.easeOut(duration: 0.3), value: fraction)
                } else {
                    TimelineView(.animation) { timeline in
                        let t = timeline.date.timeIntervalSinceReferenceDate.truncatingRemainder(dividingBy: 1.4) / 1.4
                        Capsule().fill(tint.opacity(0.8))
                            .frame(width: geo.size.width * 0.3)
                            .offset(x: (geo.size.width * 1.3) * t - geo.size.width * 0.3)
                    }
                    .clipShape(Capsule())
                }
            }
        }
        .frame(height: 4)
    }
}

/// A quieter capsule for actions next to the primary one, like an error's fix-it button.
struct SecondaryCapsuleStyle: ButtonStyle {
    var height: CGFloat = 28

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(.white.opacity(0.92))
            .padding(.horizontal, 12)
            .frame(height: height)
            .background(Capsule().fill(.white.opacity(configuration.isPressed ? 0.2 : 0.1)))
            .overlay(Capsule().strokeBorder(Theme.stroke))
            .contentShape(Capsule())
    }
}

struct Tag: View {
    let text: String
    let color: Color

    var body: some View {
        Text(text)
            .font(.system(size: 10, weight: .semibold))
            .foregroundStyle(color)
            .padding(.horizontal, 7).padding(.vertical, 3)
            .background(Capsule().fill(color.opacity(0.15)))
    }
}

struct Thumbnail: View {
    let url: URL?
    var width: CGFloat

    var body: some View {
        AsyncImage(url: url) { image in
            image.resizable().aspectRatio(contentMode: .fill)
        } placeholder: {
            Image(systemName: "play.fill").font(.system(size: width * 0.14)).foregroundStyle(Theme.tertiary)
        }
        .frame(width: width, height: width * 9 / 16)
        .background(Color.white.opacity(0.05))
        .clipShape(RoundedRectangle(cornerRadius: max(6, width * 0.08), style: .continuous))
        .accessibilityHidden(true)
    }
}
