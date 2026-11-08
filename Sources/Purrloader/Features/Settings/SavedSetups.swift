import PurrloaderCore
import SwiftUI

/// Presets, called Saved Setups in the UI: named snapshots of every download setting.
struct SavedSetupsMenu: View {
    @Environment(MainModel.self) private var model
    @State private var naming = false
    @State private var newName = ""
    @State private var managing = false
    private var store: PresetStore { .shared }

    var body: some View {
        Menu("Saved Setups") {
            ForEach(store.presets) { preset in
                Button(preset.name) { apply(preset) }
            }
            Divider()
            Button("Save Current Settings…") {
                newName = ""; naming = true
            }
            Button("Manage Saved Setups…") { managing = true }
        }
        .fixedSize()
        .alert("Save Setup", isPresented: $naming) {
            TextField("Name", text: $newName)
            Button("Save") {
                let name = newName.trimmed
                guard !name.isEmpty else { return }
                _ = store.add(name: name, options: model.options)
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("Saves every setting, including the quality and format in the main window.")
        }
        .sheet(isPresented: $managing) { ManageSetupsView() }
    }

    private func apply(_ preset: Preset) {
        let exact = model.options.format.exactFormat
        model.options = preset.options
        model.options.format.exactFormat = exact
    }
}

private struct ManageSetupsView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var selection: Preset.ID?
    @State private var renaming = ""
    private var store: PresetStore { .shared }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Saved Setups").font(.headline)
            List(selection: $selection) {
                ForEach(store.presets) { preset in
                    Text(preset.name).tag(preset.id)
                }
                .onMove { store.presets.move(fromOffsets: $0, toOffset: $1) }
            }
            .listStyle(.bordered(alternatesRowBackgrounds: true))
            HStack {
                TextField("Name", text: $renaming)
                    .onSubmit(rename)
                    .disabled(selection == nil)
                Button("Rename", action: rename).disabled(selection == nil || renaming.trimmed.isEmpty)
                Button("Delete") {
                    if let preset = store.presets.first(where: { $0.id == selection }) { store.delete(preset) }
                    selection = nil
                }
                .disabled(selection == nil)
            }
            HStack {
                Text("Drag to reorder.").font(.callout).foregroundStyle(.secondary)
                Spacer()
                Button("Restore Built-ins") {
                    for preset in PresetStore.builtIn where !store.presets.contains(where: { $0.name == preset.name }) {
                        store.presets.append(preset)
                    }
                }
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
            }
        }
        .padding()
        .frame(width: 440, height: 360)
        .onChange(of: selection) {
            renaming = store.presets.first { $0.id == selection }?.name ?? ""
        }
    }

    private func rename() {
        guard let index = store.presets.firstIndex(where: { $0.id == selection }), !renaming.trimmed.isEmpty else {
            return
        }
        store.presets[index].name = renaming.trimmed
    }
}
