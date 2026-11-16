import AppKit
import UniformTypeIdentifiers
import VixenBridge
import VixenKit

/// Tools menu actions, and the audio association flows: F78 to F86, F92, F302, F303.
extension SequencerModel {
    /// An alert with the app's usual follow-through.
    func showAlert(_ alert: ShellAlert) {
        afterCommand(ShellOutcome(alert: alert, request: nil, clipboardJson: nil, clipboardCount: 0, handled: true))
    }

    /// F245's OK. The grid repaints from the timeline's own update.
    func applyEditTime(clicked: UInt64, startMs: Double, durationMs: Double) {
        shell?.applyEditTime(clicked: clicked, startMs: startMs, durationMs: durationMs)
        refresh()
    }

    func openEvenMarks() {
        guard let shell else { return }
        sheet = .evenMarks(shell.evenMarksValues(), divisions: 1)
    }

    /// F82: the message, and the dialog opens again.
    func createEvenMarks(startMs: Double, endMs: Double, divisions: Int, range: ShellTimeRange) {
        guard let shell else { return }
        if let problem = shell.createEvenMarks(startMs: startMs, endMs: endMs, divisions: Int32(divisions)) {
            Task {
                await SequencerAlerts.tell(title: "Mark Length Error", message: problem, style: .error, window: window)
                sheet = .evenMarks(ShellTimeRange(startMs: startMs, endMs: endMs, lengthMs: range.lengthMs), divisions: divisions)
            }
            return
        }
        refresh()
    }

    func openBulkMove() {
        guard let shell else { return }
        sheet = .bulkMove(cursorMs: shell.cursorMs(), lengthMs: shell.lengthMs())
    }

    func openSequenceLength() {
        guard let shell else { return }
        sheet = .sequenceLength(lengthMs: shell.lengthMs())
    }

    func openColorCollections() {
        guard let shell else { return }
        sheet = .colorCollections(shell.colorCollectionEditor())
    }

    /// F84, F85. The selectors are VixenKit's.
    func openCurveSelector() {
        guard let window, let core = app.core else { return }
        CurveLibrarySelector.present(on: window, core: core) { [weak self] item in
            guard let self, let window = self.window else { return }
            CurveEditorDialog.editLibraryEntry(on: window, core: core, name: item.name) { [weak self] in self?.librariesChanged() }
        }
    }

    func openGradientSelector() {
        guard let window, let core = app.core else { return }
        GradientLibrarySelector.present(on: window, core: core) { [weak self] item in
            guard let self, let window = self.window else { return }
            GradientEditorDialog.editLibraryEntry(on: window, core: core, name: item.name) { [weak self] in self?.librariesChanged() }
        }
    }

    /// F91.
    func papagayoImport() {
        guard let shell, let url = LibraryFilePanels.open(title: "Papagayo Import", ext: "pgo", typeName: "Papagayo files") else { return }
        afterCommand(shell.papagayoImport(path: url.path))
    }

    // MARK: Effect Defaults, F93 to F95, F403

    func exportEffectDefaults() {
        guard let shell else { return }
        let summaries = shell.effectDefaultSummaries()
        guard !summaries.isEmpty else {
            Task { await SequencerAlerts.tell(title: "Export Effect Defaults", message: "There are no saved effect defaults to export.", style: .information, window: window) }
            return
        }
        sheet = .effectDefaults(summaries)
    }

    /// F403's OK. Nothing checked returns silently.
    func finishExportingEffectDefaults(_ typeIds: [String]) {
        guard let shell, !typeIds.isEmpty else { return }
        guard let url = LibraryFilePanels.save(title: "Export Effect Defaults", name: "Effect Defaults", ext: "vfd", typeName: "Vixen 3 Effect Defaults") else { return }
        do {
            try shell.exportEffectDefaults(path: url.path, typeIds: typeIds)
        } catch {
            FileHandle.standardError.write(Data("effect defaults export: \(error.userMessage)\n".utf8))
            Task { await SequencerAlerts.tell(title: "Unable to Export", message: "Unable to export effect defaults, please check the error log for details", style: .warning, window: window) }
        }
    }

    func importEffectDefaults() {
        guard let shell, let url = LibraryFilePanels.open(title: "Import Effect Defaults", ext: "vfd", typeName: "Vixen 3 Effect Defaults") else { return }
        Task {
            do {
                let message = try shell.importEffectDefaults(path: url.path)
                await SequencerAlerts.tell(title: "Import Effect Defaults", message: message, style: .information, window: window)
            } catch {
                FileHandle.standardError.write(Data("effect defaults import: \(error.userMessage)\n".utf8))
                await SequencerAlerts.tell(title: "Unable to Import", message: "Unable to import effect defaults, please check the error log for details", style: .warning, window: window)
            }
        }
    }

    func dumpEffectDefaults() {
        guard let shell, let url = LibraryFilePanels.save(title: "Dump Effect Defaults", name: "Effect Defaults", ext: "xml", typeName: "XML Files") else { return }
        do {
            try shell.dumpEffectDefaults(path: url.path)
        } catch {
            FileHandle.standardError.write(Data("effect defaults dump: \(error.userMessage)\n".utf8))
            Task { await SequencerAlerts.tell(title: "Unable to Write Dump", message: "Unable to write the diagnostic dump, please check the error log for details", style: .warning, window: window) }
        }
    }

    // MARK: Audio

    /// F302: "Associate Audio". The warning first, when audio's there.
    func associateAudio() {
        guard let shell else { return }
        Task {
            if let warning = shell.existingAudioWarning() {
                let go = await SequencerAlerts.confirm(
                    title: warning.title,
                    message: warning.message,
                    style: warning.style,
                    confirm: warning.confirmLabel ?? "OK",
                    cancel: warning.cancelLabel ?? "Cancel",
                    window: window
                )
                if !go {
                    return
                }
            }
            await pickAudioFile()
        }
    }

    /// F79, F303.
    func removeAudio() {
        guard let shell, let prompt = shell.removeAudioPrompt() else { return }
        showAlert(prompt)
    }

    /// The open panel, and everything after it.
    func pickAudioFile() async {
        guard let url = await chooseAudioFile() else { return }
        await associate(url)
    }

    private func chooseAudioFile() async -> URL? {
        if UITestHarness.isActive {
            return ProcessInfo.processInfo.environment["VIXEN_TEST_FILE"].map { URL(fileURLWithPath: $0) }
        }
        let panel = NSOpenPanel()
        panel.title = "Select an Audio File"
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.allowedContentTypes = sequencerAudioExtensions().compactMap { UTType(filenameExtension: $0) }
        if let window {
            return await panel.beginSheetModal(for: window) == .OK ? panel.url : nil
        }
        return panel.runModal() == .OK ? panel.url : nil
    }

    private func associate(_ url: URL) async {
        guard let shell else { return }
        var replace = false
        if shell.audioImportConflict(path: url.path) {
            replace = await SequencerAlerts.confirm(
                title: "Replace existing Media file?",
                message: "A Media file with the same name already exists. Do you want to replace it?",
                style: .warning,
                confirm: "Replace",
                cancel: "Use Existing",
                window: window
            )
        }
        switch shell.addAudio(path: url.path, replaceExistingCopy: replace) {
        case .unsupported:
            await SequencerAlerts.tell(title: "Warning", message: "The selected file is not a supported type.", style: .warning, window: window)
        case .unprocessable:
            await SequencerAlerts.tell(title: "Error", message: "The selected audio file cannot be processed.\n\nFile: \(url.path)", style: .error, window: window)
        case .added(let resizePrompt):
            if resizePrompt {
                let resize = await SequencerAlerts.confirm(
                    title: "Resize sequence?",
                    message: "Do you want to resize the sequence to the size of the audio?",
                    style: .information,
                    confirm: "Resize",
                    cancel: "Keep Length",
                    window: window
                )
                if resize {
                    shell.resizeToAudio()
                }
            }
        }
        refresh()
    }

    /// F9, F383.
    func askAboutMissingAudio(path: String) async {
        let answer: MissingAudioAnswer = await withCheckedContinuation { continuation in
            sheet = .missingAudio(path: path) { continuation.resume(returning: $0) }
        }
        guard let shell else { return }
        switch answer {
        case .keep:
            break
        case .remove:
            _ = shell.runAlertAction(action: .removeAudio)
        case .locate:
            await pickAudioFile()
        }
        refresh()
    }
}
