import AppKit
import SwiftUI
import VixenBridge
import VixenKit

/// Scheduler > Schedules, `SetupForm`: F49 to F53. The rules are Rust's `vixen_session::scheduler`.
struct SchedulesRoot: View {
    @Environment(AppModel.self) private var app
    @Environment(\.dismiss) private var dismiss
    @State private var model: SchedulesModel?

    var body: some View {
        Group {
            if let model {
                SchedulesView(model: model, close: { dismiss() })
            } else {
                ProgressView().frame(width: 760, height: 430)
            }
        }
        .task {
            await app.start()
            for _ in 0..<400 where app.core == nil || SchedulerHub.current == nil {
                try? await Task.sleep(for: .milliseconds(50))
            }
            guard let hub = SchedulerHub.current else { return }
            let made = SchedulesModel(hub: hub)
            SchedulesModel.current = made
            model = made
        }
    }
}

struct ScheduleRowItem: Identifiable {
    let id: Int
    let row: ScheduleRow
}

@Observable
final class SchedulesModel {
    let hub: SchedulerHub
    let texts: SchedulerTexts
    var selection: Int?
    var sheet: ScheduleSheetModel?
    var editor: ShowEditorModel?
    @ObservationIgnored weak var window: NSWindow?
    @ObservationIgnored static weak var current: SchedulesModel?

    init(hub: SchedulerHub) {
        self.hub = hub
        texts = hub.session.texts()
        hub.refresh()
    }

    var items: [ScheduleRowItem] {
        hub.rows.enumerated().map { ScheduleRowItem(id: $0.offset, row: $0.element) }
    }

    func add() {
        sheet = ScheduleSheetModel(hub: hub, index: nil) { [weak self] in self?.sheet = nil }
    }

    func edit() {
        guard let selection else { return }
        sheet = ScheduleSheetModel(hub: hub, index: selection) { [weak self] in self?.sheet = nil }
    }

    /// F51.
    func delete() async {
        guard let selection, hub.rows.indices.contains(selection) else { return }
        guard await SchedulerAlerts.confirm(texts.delete.title, texts.delete.message, style: .warning, confirm: "Delete", cancel: "Cancel", destructive: true, window: window) else { return }
        hub.session.delete(index: UInt32(selection))
        self.selection = nil
        hub.refresh()
    }

    /// F52.
    func editShow() async {
        guard let selection else { return }
        guard let show = hub.session.showOf(index: UInt32(selection)), let session = hub.core.showEditor(index: show) else {
            await SchedulerAlerts.tell(texts.noShow.title, texts.noShow.message, style: .error, window: window)
            return
        }
        editor = ShowEditorModel(session: session) { [weak self] in
            self?.editor = nil
            self?.hub.refresh()
            ShowsModel.current?.refresh()
        }
    }

    /// F49: a change is saved as the window closes.
    func closed() {
        if let failure = hub.session.windowClosed() {
            Task { await SchedulerAlerts.tell("Scheduler", "The schedule could not be saved.\n\n\(failure)", style: .error, window: nil) }
        }
    }
}

struct SchedulesView: View {
    @Bindable var model: SchedulesModel
    let close: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            GroupBox("Shows") {
                Text(model.texts.showsBlurb)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(4)
            }
            GroupBox("Scheduled Shows") {
                VStack(spacing: 8) {
                    table
                    HStack(spacing: 6) {
                        iconButton("tablecells.badge.ellipsis", "Edit Selected Show") { Task { await model.editShow() } }
                            .disabled(model.selection == nil)
                        Spacer()
                        iconButton("plus", "Add a New Scheduled Show") { model.add() }
                        iconButton("pencil", "Edit the Selected Schedule") { model.edit() }
                            .disabled(model.selection == nil)
                        iconButton("minus", "Delete the Selected Schedule") { Task { await model.delete() } }
                            .disabled(model.selection == nil)
                    }
                }
                .padding(4)
            }
            HStack {
                Button("Help") { SchedulerLinks.open(model.texts.helpUrl) }
                Spacer()
                Button("Close", action: close)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 760, height: 430)
        .background(WindowReader { model.window = $0 })
        .onDisappear { model.closed() }
        .sheet(item: $model.sheet) { sheet in
            ScheduleSheet(model: sheet)
        }
        .sheet(item: $model.editor) { editor in
            ShowEditorSheet(model: editor)
        }
    }

    private var table: some View {
        Table(model.items, selection: $model.selection) {
            TableColumn("Show") { Text($0.row.show) }
                .width(min: 80, ideal: 100)
            TableColumn("Schedule") { Text($0.row.schedule) }
                .width(min: 200, ideal: 460)
            TableColumn("Status") { Text($0.row.status) }
                .width(min: 60, ideal: 80)
        }
        .contextMenu(forSelectionType: Int.self) { rows in
            Button("Add a Scheduled Show...") { model.add() }
            Button("Edit the Selected Schedule...") {
                select(rows)
                model.edit()
            }
            .disabled(rows.isEmpty && model.selection == nil)
            Button("Delete the Selected Schedule") {
                select(rows)
                Task { await model.delete() }
            }
            .disabled(rows.isEmpty && model.selection == nil)
            Divider()
            Button("Edit the Associated Show...") {
                select(rows)
                Task { await model.editShow() }
            }
            .disabled(rows.isEmpty && model.selection == nil)
        } primaryAction: { rows in
            // F50: a double-click edits.
            select(rows)
            model.edit()
        }
    }

    private func select(_ rows: Set<Int>) {
        if let row = rows.first { model.selection = row }
    }

    private func iconButton(_ symbol: String, _ help: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol).frame(width: 18, height: 16)
        }
        .help(help)
        .accessibilityLabel(help)
    }
}

/// `SetupScheduleForm`: F53.
@Observable
final class ScheduleSheetModel: Identifiable {
    let hub: SchedulerHub
    let index: Int?
    let showNames: [String]
    var show: Int
    var enabled: Bool
    var startDate: Date
    var endDate: Date
    var days: [Bool]
    var startTime: Date
    var endTime: Date
    private let finished: () -> Void
    @ObservationIgnored weak var window: NSWindow?
    @ObservationIgnored static weak var current: ScheduleSheetModel?

    static let dayNames = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]

    init(hub: SchedulerHub, index: Int?, finished: @escaping () -> Void) {
        self.hub = hub
        self.index = index
        self.finished = finished
        let values = hub.session.editValues(index: index.map(UInt32.init))
        showNames = values.showNames
        show = values.show.map(Int.init) ?? -1
        enabled = values.enabled
        startDate = Self.date(Int(values.startYear), Int(values.startMonth), Int(values.startDay))
        endDate = Self.date(Int(values.endYear), Int(values.endMonth), Int(values.endDay))
        days = (0..<7).map { $0 < values.days.count ? values.days[$0] : false }
        startTime = Self.time(values.startSeconds)
        endTime = Self.time(values.endSeconds)
        ScheduleSheetModel.current = self
    }

    private static func date(_ year: Int, _ month: Int, _ day: Int) -> Date {
        Calendar.current.date(from: DateComponents(year: year, month: month, day: day)) ?? Date()
    }

    private static func time(_ seconds: UInt32) -> Date {
        let s = Int(seconds)
        return Calendar.current.date(from: DateComponents(year: 2001, month: 1, day: 1, hour: s / 3600, minute: s / 60 % 60, second: s % 60)) ?? Date()
    }

    static func seconds(_ time: Date) -> UInt32 {
        let parts = Calendar.current.dateComponents([.hour, .minute, .second], from: time)
        return UInt32((parts.hour ?? 0) * 3600 + (parts.minute ?? 0) * 60 + (parts.second ?? 0))
    }

    var durationText: String {
        hub.session.durationText(startSeconds: Self.seconds(startTime), endSeconds: Self.seconds(endTime))
    }

    private var edit: ScheduleEdit {
        let start = Calendar.current.dateComponents([.year, .month, .day], from: startDate)
        let end = Calendar.current.dateComponents([.year, .month, .day], from: endDate)
        return ScheduleEdit(
            showNames: showNames,
            show: show >= 0 ? UInt32(show) : nil,
            enabled: enabled,
            startYear: Int32(start.year ?? 2000),
            startMonth: UInt8(start.month ?? 1),
            startDay: UInt8(start.day ?? 1),
            endYear: Int32(end.year ?? 2000),
            endMonth: UInt8(end.month ?? 1),
            endDay: UInt8(end.day ?? 1),
            days: days,
            startSeconds: Self.seconds(startTime),
            endSeconds: Self.seconds(endTime)
        )
    }

    func ok() async {
        if let alert = hub.session.applyEdit(index: index.map(UInt32.init), edit: edit) {
            await SchedulerAlerts.tell(alert.title, alert.message, style: .error, window: window)
            return
        }
        hub.refresh()
        finished()
    }

    func cancel() {
        finished()
    }
}

struct ScheduleSheet: View {
    @Bindable var model: ScheduleSheetModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Schedule a Show").font(.headline)
            GroupBox("Show") {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Show to Start:")
                        Picker("", selection: $model.show) {
                            ForEach(Array(model.showNames.enumerated()), id: \.offset) { index, name in
                                Text(name).tag(index)
                            }
                        }
                        .labelsHidden()
                        .accessibilityLabel("Show to Start")
                    }
                    Toggle("Enable Schedule", isOn: $model.enabled)
                }
                .padding(4)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            GroupBox("Dates:") {
                HStack(spacing: 16) {
                    DatePicker("Start:", selection: $model.startDate, displayedComponents: .date)
                        .datePickerStyle(.field)
                    DatePicker("Stop:", selection: $model.endDate, displayedComponents: .date)
                        .datePickerStyle(.field)
                    Spacer()
                }
                .padding(4)
            }
            GroupBox("Days:") {
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 6) {
                    GridRow {
                        ForEach(0..<4, id: \.self) { day in
                            Toggle(ScheduleSheetModel.dayNames[day], isOn: $model.days[day])
                        }
                    }
                    GridRow {
                        ForEach(4..<7, id: \.self) { day in
                            Toggle(ScheduleSheetModel.dayNames[day], isOn: $model.days[day])
                        }
                    }
                }
                .padding(4)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            GroupBox("Time:") {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 16) {
                        Text("Start Time:")
                        TimeField(time: $model.startTime, label: "Start Time")
                        Text("End Time:")
                        TimeField(time: $model.endTime, label: "End Time")
                        Spacer()
                    }
                    Text(model.durationText)
                        .foregroundStyle(.secondary)
                        .accessibilityIdentifier("scheduleDuration")
                }
                .padding(4)
            }
            HStack {
                Button("Help") { SchedulerLinks.open(model.hub.session.texts().helpUrl) }
                Spacer()
                Button("OK") { Task { await model.ok() } }
                    .keyboardShortcut(.defaultAction)
                Button("Cancel") { model.cancel() }
                    .keyboardShortcut(.cancelAction)
            }
        }
        .padding(16)
        .frame(width: 470)
        .background(WindowReader { model.window = $0 })
    }
}

/// A time of day with seconds, which SwiftUI's `DatePicker` leaves out on the Mac.
struct TimeField: NSViewRepresentable {
    @Binding var time: Date
    let label: String

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    func makeNSView(context: Context) -> NSDatePicker {
        let picker = NSDatePicker()
        picker.datePickerStyle = .textFieldAndStepper
        picker.datePickerElements = [.hourMinuteSecond]
        picker.dateValue = time
        picker.target = context.coordinator
        picker.action = #selector(Coordinator.changed(_:))
        picker.setAccessibilityLabel(label)
        return picker
    }

    func updateNSView(_ picker: NSDatePicker, context: Context) {
        context.coordinator.parent = self
        if picker.dateValue != time { picker.dateValue = time }
    }

    final class Coordinator: NSObject {
        var parent: TimeField

        init(_ parent: TimeField) {
            self.parent = parent
        }

        @objc func changed(_ sender: NSDatePicker) {
            parent.time = sender.dateValue
        }
    }
}
