//! What the Installed Modules window lists, and the sequence types behind New and Open. The original reads these from each module's
//! descriptor at run time. Here modules are compiled in, so effects come from the effect registry and the rest from this table.

use uuid::Uuid;

/// One row of Installed Modules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleRow {
    pub type_name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    /// The upstream assembly's name without `.dll`, since there is no such file here.
    pub file: String,
    /// `TypeId.ToString()`: lower case with hyphens. What Copy Module Id copies.
    pub type_id_text: String,
}

/// A module type and its modules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleGroup {
    pub name: String,
    pub rows: Vec<ModuleRow>,
}

/// (group, name, description, author, version, file, type id), in the original's module file order within each group.
const STATIC_MODULES: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    ("Analysis", "Beats and Bars", "Beats and Bars Autodetection", "Ed Brady", "1.0", "Module.Analysis.BeatsAndBars", "cf6326f5-424e-4c04-bf7b-ef511d9ca4e5"),
    ("App", "ColorGradients", "Provides a data type which represents an arbitrary color gradient/transition over time, and additional functionality (such as a gradient editor and library).", "Vixen Team", "1.0", "Module.App.ColorGradients", "64f4ab26-3ed4-49a3-a004-23656ed0424a"),
    ("App", "Curves", "Provides a data type which represents a curve and additional functionality (such as a curve editor and library).", "Vixen Team", "1.0", "Module.App.Curves", "4e258de2-7a75-4f0f-aa43-c8182e7f3400"),
    ("App", "Custom Prop Editor", "Enables creation of custom props for Previews", "Vixen Team", "1.0", "Module.App.CustomPropEditor", "dc844055-e978-47fe-8359-dbcd5b568e80"),
    ("App", "Export Wizard", "Wizard to export sequences to other formats.", "Jeff Uchitjil", "1.0", "Module.App.ExportWizard", "8bf8d0c5-b7d6-417c-9eff-ec1c5e1ef9ab"),
    ("App", "FPP Client", "REST API client for Falcon Player (FPP) instances", "Vixen Team", "1.0", "Module.App.FPPClient", "c9eda00c-e5ac-4880-8bb5-00230a426703"),
    ("App", "Instrumentation Panel", "Text display of a few stats.", "Vixen Team", "1.0", "Module.App.Instrumentation", "cbc07bf2-7e1b-4a29-aeed-9ee235cb0dee"),
    ("App", "LipSync App", "Breaks down Phoneme mappings into discrete animation components.", "Ed Brady", "1.0", "Module.App.LipSyncApp", "a475c535-73fa-4d31-a1bc-ed2b50ad21cf"),
    ("App", "Live Preview", "Shared live element control service for broadcast-driven previews", "Vixen Team", "1.0", "Module.App.LivePreview", "6f857027-6cbc-4729-bd00-6769c326e2cc"),
    ("App", "Shows", "Create and edit a massive show extravaganza", "Derek Backus", "1.0", "Module.App.Shows", "67a6dfdb-ddec-4a5f-a158-129421e2366f"),
    ("App", "Super Scheduler", "Expansive Show Scheduler.", "Derek Backus", "1.0", "Module.App.SuperScheduler", "985315e8-2b83-4d1a-89d1-58bfea7835a0"),
    ("App", "TimedSequencePackager", "Import / Export / Map Timed Sequences between profiles.", "Jeff Uchitjil", "1.0", "Module.App.TimedSequenceMapper", "b13997c2-d4b5-4b5a-9047-1dced97ad99b"),
    ("App", "Vixen WebServer", "Web interface to Vixen 3", "Darren McDaniel", "1.0", "Module.App.WebServer", "8ac1cdf9-8d57-43b6-81de-93003ed2f907"),
    ("Controller", "DDP", "DDP Output Controller Module", "Jon Chuchla", "1.0", "Module.Controller.DDP", "110b5fe9-cd4d-4e73-bb5b-67552a1ad6d1"),
    ("Controller", "Debugging Values", "A debugging controller that will display raw values in an output window.", "Vixen Team", "1.0", "Module.Controller.DebugController", "f32bda77-28f0-46c4-a6bd-ba0ba73a1c13"),
    ("Controller", "DMX Pro", "DMX USB Pro hardware controller module", "Vixen Team", "1.0", "Module.Controller.DMXUsbPro", "8abf27a7-3156-4f4c-9d43-ada45cdd6d0c"),
    ("Controller", "Dummy Lighting (for testing)", "A dummy/test output module that will display lighting commands in a new window.", "Vixen Team", "1.0", "Module.Controller.DummyLighting", "b6ee6308-189c-4268-8996-32a4bab8ab5f"),
    ("Controller", "Streaming ACN (E1.31)", "An output plugin that emits E1.31 data.", "Erik Mathisen and Joshua 1 Systems Inc.", "0.1", "Module.Controller.E131", "771d7eba-662a-48d5-aedb-445c8708e878"),
    ("Controller", "Elexol Ethernet I/O 24", "Elexol Ether I/O 24", "Tony Eberle", "1.0", "Module.Controller.ElexolEtherIO", "2565e617-89db-4761-be8d-4d5574cafba9"),
    ("Controller", "Generic Serial", "Generic Serial hardware module", "John McAdams (macebobo)", "1.0", "Module.Controller.GenericSerial", "d22c9233-16e9-4008-a215-e577f78a7a2e"),
    ("Controller", "Launcher", "Generic Windows Command / App Launcher", "Jon Chuchla / Darren McDaniel / Steve Dupuis", "1.0", "Module.Controller.LauncherController", "1c4bfd64-3172-43a7-a7fc-0a77c88b0489"),
    ("Controller", "DMX Open", "Enttec Open DMX Output Module", "Jon Chuchla - based on the original work of Joshua Moyerman & piense.", "2.0", "Module.Controller.OpenDMX", "7911568a-8eda-4d1d-9e72-be41d7a843e4"),
    ("Controller", "RDS", "Generic Windows Command and RDS hardware module", "Darren McDaniel / Steve Dupuis", "1.0", "Module.Controller.RDSController", "692b66b9-e831-4f39-bb73-7653fa62b053"),
    ("Controller", "Renard", "Renard hardware module", "Vixen Team", "1.0", "Module.Controller.Renard", "f66130a6-ff8a-48a7-bf1e-fac6cec2fc2c"),
    ("Editor", "Timed Sequence Editor", "An editor for a standard timed sequence", "Vixen Team", "1.0", "Module.Editor.TimedSequenceEditor", "d342eedd-ae39-4b30-b557-b9329e6d3a7c"),
    ("LayerMixingFilter", "Chroma Key", "Creates a Chroma mask based on the selected brightness and fills with the higher layer.", "Jon Chuchla", "1.0", "Module.LayerMixingFilter.ChromaKey", "2d467733-1a64-45ad-bcdc-84bf074747e6"),
    ("LayerMixingFilter", "Color Change", "Replaces the current color with the effect color at the same intensity.", "Jeff Uchitjil", "1.0", "Module.LayerMixingFilter.ColorChange", "93d4169e-5138-403a-b71c-a5a1ab089801"),
    ("LayerMixingFilter", "Highest Value (Default)", "Combines two layers by passing the higher value", "Jon Chuchla", "1.0", "Module.LayerMixingFilter.HighestValue", "01d6d526-9586-44bb-82f4-10abacf58b61"),
    ("LayerMixingFilter", "Intensity Overlay", "Uses the higher layer intensity to apply to the lower layer.", "Jeff Uchitjil", "1.0", "Module.LayerMixingFilter.IntensityOverlay", "43f48bf5-4088-403f-ad7a-d0ff2114ca31"),
    ("LayerMixingFilter", "Luma Key", "Creates a luma mask based on the selected brightness and fills with the higher layer.", "Jon Chuchla", "1.0", "Module.LayerMixingFilter.LumaKey", "7583f7d4-df99-4492-8d10-257c2677ff7b"),
    ("LayerMixingFilter", "Mask", "Uses the higher layer to mask the next lower layer.", "Jeff Uchitjil", "1.0", "Module.LayerMixingFilter.Mask", "ace0f097-7982-4edc-b5dd-8989dce60e1d"),
    ("LayerMixingFilter", "Mask And Fill", "Uses the higher layer to mask and fill in the next lower layer.", "Jeff Uchitjil", "1.0", "Module.LayerMixingFilter.MaskFill", "24878b18-14f8-458e-abaa-3a9c9a34931c"),
    ("LayerMixingFilter", "Multiply Color", "Multiplies the colors between layers.  Useful for modifying a color or brightness with another layer as input.", "Jon Chuchla", "1.0", "Module.LayerMixingFilter.MultiplyColor", "0c9dfc0b-8e39-4167-8036-79408c6b0b35"),
    ("LayerMixingFilter", "Proportional Mix", "Combines two layers by using an inverse proportion of the intensity of the higher layer applied to the lower layer", "Jeff Uchitjil", "1.0", "Module.LayerMixingFilter.ProportionalMix", "1f16d67a-1fff-485c-8e76-27ff7e7d072b"),
    ("Media", "Audio", "Allows an audio file to be used as a media item in a sequence.", "Vixen Team", "1.0", "Module.Media.Audio", "fe460392-3fab-4c63-99dd-d76c48354150"),
    ("OutputFilter", "Coarse Fine Breakdown", "An output filter that breaks an intent value into a coarse (high byte) and a fine (low byte).", "Vixen Team", "1.0", "Module.OutputFilter.CoarseFineBreakdown", "7c1465df-054c-4875-ab61-d7e7f5236897"),
    ("OutputFilter", "Color Breakdown", "An output filter that breaks down color intents into discrete color components.", "Vixen Team", "1.0", "Module.OutputFilter.ColorBreakdown", "ab38a16f-0de1-4f6e-a8c0-ae5b20d347e0"),
    ("OutputFilter", "Color Wheel Filter", "An output filter that converts color intents into color wheel index commands.", "Vixen Team", "1.0", "Module.OutputFilter.ColorWheelFilter", "18470c0c-1e75-464f-bd9f-a3e241cd1eb8"),
    ("OutputFilter", "Dimming Curve", "An output filter that translates lighting intensity values according to a curve (to compensate for non-linear lighting response).", "Vixen Team", "1.0", "Module.OutputFilter.DimmingCurve", "2e40d6b1-43d2-4668-b63a-c600fadd7dd5"),
    ("OutputFilter", "Dimming Filter", "An output filter that converts color intents into dimmer intents.", "Vixen Team", "1.0", "Module.OutputFilter.DimmingFilter", "61913688-ef44-473c-91d4-d256cca6cb91"),
    ("OutputFilter", "Prism Filter", "An output filter that converts prism intents into open prism commands.", "Vixen Team", "1.0", "Module.OutputFilter.PrismFilter", "2522bd94-0434-4728-9400-0e18b212dc21"),
    ("OutputFilter", "Shutter Filter", "An output filter that converts color intents into shutter commands.", "Vixen Team", "1.0", "Module.OutputFilter.ShutterFilter", "72b86092-0a59-4e40-b0a8-9e5fad0b8eb3"),
    ("OutputFilter", "Tag Filter", "An output filter that filters intents based on a tag.", "Vixen Team", "1.0", "Module.OutputFilter.TaggedFilter", "fe552dcf-1bc0-4588-b23c-2600151d9fc5"),
    ("Preview", "Vixen Display Preview", "Vixen Display Preview", "Derek Backus", "1.0.0", "Module.Preview.VixenPreview", "f43c31bc-cfcd-4dbc-adb7-d349c9ec623d"),
    ("Property", "Color", "Defines the element to have color, and the particular color(s) it can produce", "Vixen Team", "0.1", "Module.Property.Color", "bff34727-6b88-4f87-82b7-68424498c725"),
    ("Property", "Face", "Provides mechanism for assigning Phoneme mapping of display elements", "Jeff Uchitjil", "1.0", "Module.Property.Face", "eda73851-8350-4d3f-9df2-0d3b34e70bb6"),
    ("Property", "Grid", "Marks the attributed node as a grid", "Vixen Team", "1.0", "Module.Property.Grid", "de3b4ec9-4e7f-4690-b0fe-3780b17a85ec"),
    ("Property", "Intelligent Fixture", "Provides ability to specify capabilities of an intelligent fixture", "Vixen Team", "1.0", "Module.Property.IntelligentFixture", "fdd74c85-1779-4521-b559-0dd5ea71054e"),
    ("Property", "Location", "Provides XYZ positioning of display elements", "Darren McDaniel", "1.0", "Module.Property.Location", "3fb53423-dd2a-4719-b3e3-19aa6f062f62"),
    ("Property", "Order", "Provides order mechanism for patching of display elements", "Jeff Uchitjil", "1.0", "Module.Property.Order", "47d4a43b-5407-44bd-8be1-4974b148504f"),
    ("Property", "Orientation", "Provides Orientation information about display elements", "Jeff Uchitjil", "1.0", "Module.Property.Orientation", "db0fc9fc-cdd5-4f88-bd66-5e4cf08feae4"),
    ("Property", "State", "Provides mechanism for assigning State mapping of display elements", "Jeff Uchitjil", "1.0", "Module.Property.State", "35b893c3-d10c-4359-82bc-929c0762e809"),
    ("SequenceType", "Timed Sequence", "A basic timed sequence, which is a collection of effects configured to occur at predefined times.", "Vixen Team", "9.0", "Module.SequenceType.Timed", "296bdba2-9bf3-4bff-a9f2-13efac5c8ecb"),
    ("SequenceType", "Vixen 2.x Sequence", "For importing sequences from Vixen 2.x.", "John McAdams", "1.0", "Module.SequenceType.Vixen2x", "92bbd2cb-b750-437f-8a88-49864d569ab4"),
    ("Timing", "Generic timing", "Stopwatch-based timing module", "Vixen Team", "1.0", "Module.Timing.Generic", "00000000-0000-0000-0000-000000000000"),
];

const EFFECT_GROUP: &str = "Effect";

/// `Module.Effect.{folder}`: the effect's project folder, the one holding `Images/EffectImage.png`.
fn effect_file(icon: &str) -> String {
    let folder = icon.rsplit('/').nth(2).unwrap_or("");
    format!("Module.Effect.{folder}")
}

/// Every module, in groups sorted by name. A group with no modules is left out.
pub fn installed_modules() -> Vec<ModuleGroup> {
    let mut groups: Vec<ModuleGroup> = Vec::new();
    let mut add = |group: &str, row: ModuleRow| match groups.iter_mut().find(|g| g.name == group) {
        Some(existing) => existing.rows.push(row),
        None => groups.push(ModuleGroup { name: group.to_string(), rows: vec![row] }),
    };
    for (group, name, description, author, version, file, type_id) in STATIC_MODULES {
        add(
            group,
            ModuleRow {
                type_name: name.to_string(),
                description: description.to_string(),
                author: author.to_string(),
                version: version.to_string(),
                file: file.to_string(),
                type_id_text: type_id.to_string(),
            },
        );
    }
    for d in vixen_effect_registry::all() {
        add(
            EFFECT_GROUP,
            ModuleRow {
                type_name: d.name.to_string(),
                description: d.description.to_string(),
                author: d.author.to_string(),
                version: d.version.to_string(),
                file: effect_file(d.icon),
                type_id_text: type_id_text(&d.type_id),
            },
        );
    }
    // The module file order is NTFS's, by name and ignoring case.
    for group in &mut groups {
        group.rows.sort_by_key(|row| row.file.to_lowercase());
    }
    groups.sort_by(|a, b| a.name.cmp(&b.name));
    groups
}

fn type_id_text(id: &Uuid) -> String {
    id.hyphenated().to_string()
}

/// A sequence type's descriptor, as far as New and Open use it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceTypeInfo {
    pub type_name: String,
    pub file_extension: String,
    pub can_create_new: bool,
}

/// In module file order.
pub fn sequence_types() -> Vec<SequenceTypeInfo> {
    vec![
        SequenceTypeInfo { type_name: "Timed Sequence".to_string(), file_extension: ".tim".to_string(), can_create_new: true },
        SequenceTypeInfo { type_name: "Vixen 2.x Sequence".to_string(), file_extension: ".vix".to_string(), can_create_new: false },
    ]
}

/// One entry of the Open dialog's file type menu. No extensions means every file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    pub label: String,
    pub extensions: Vec<String>,
}

/// `All Sequence Types`, each sequence type, then `All files`, as `VixenApplication` builds the filter.
pub fn open_panel_filters() -> Vec<FileFilter> {
    let types = sequence_types();
    let bare = |t: &SequenceTypeInfo| t.file_extension.trim_start_matches('.').to_string();
    let all_patterns: String = types.iter().map(|t| format!("*{};", t.file_extension)).collect();
    let mut filters = vec![FileFilter {
        label: format!("All Sequence Types ({all_patterns})"),
        extensions: types.iter().map(bare).collect(),
    }];
    filters.extend(types.iter().map(|t| FileFilter {
        label: format!("{} (*{})", t.type_name, t.file_extension),
        extensions: vec![bare(t)],
    }));
    filters.push(FileFilter { label: "All files (*.*)".to_string(), extensions: Vec::new() });
    filters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_are_sorted_and_all_there() {
        let names: Vec<String> = installed_modules().into_iter().map(|g| g.name).collect();
        assert_eq!(
            names,
            [
                "Analysis",
                "App",
                "Controller",
                "Editor",
                "Effect",
                "LayerMixingFilter",
                "Media",
                "OutputFilter",
                "Preview",
                "Property",
                "SequenceType",
                "Timing"
            ]
        );
    }

    #[test]
    fn there_are_113_rows_58_of_them_effects() {
        let groups = installed_modules();
        let total: usize = groups.iter().map(|g| g.rows.len()).sum();
        assert_eq!(total, 113);
        assert_eq!(STATIC_MODULES.len(), 55);
        assert_eq!(groups.iter().find(|g| g.name == "Effect").unwrap().rows.len(), 58);
    }

    fn files_of(group: &str) -> Vec<String> {
        installed_modules().into_iter().find(|g| g.name == group).unwrap().rows.into_iter().map(|r| r.file).collect()
    }

    #[test]
    fn controllers_sort_ignoring_case() {
        let names: Vec<String> =
            installed_modules().into_iter().find(|g| g.name == "Controller").unwrap().rows.into_iter().map(|r| r.type_name).collect();
        assert_eq!(names[..4], ["DDP", "Debugging Values", "DMX Pro", "Dummy Lighting (for testing)"]);
    }

    #[test]
    fn effects_sort_by_file_ignoring_case() {
        let files = files_of("Effect");
        let at = |name: &str| files.iter().position(|f| f == name).unwrap_or_else(|| panic!("{name} in {files:?}"));
        assert!(at("Module.Effect.Snowflakes") < at("Module.Effect.SnowStorm"));
        assert!(at("Module.Effect.VerticalMeter") < at("Module.Effect.VUMeter"));
        assert!(at("Module.Effect.Video") < at("Module.Effect.VUMeter"));
        assert!(files.contains(&"Module.Effect.FixtureEffect".to_string()));
    }

    #[test]
    fn the_tag_filter_is_the_last_output_filter() {
        let groups = installed_modules();
        let filters = groups.iter().find(|g| g.name == "OutputFilter").unwrap();
        let last = filters.rows.last().unwrap();
        assert_eq!(last.type_name, "Tag Filter");
        assert_eq!(last.type_id_text, "fe552dcf-1bc0-4588-b23c-2600151d9fc5");
    }

    #[test]
    fn apps_are_in_module_file_order() {
        let groups = installed_modules();
        let app = groups.iter().find(|g| g.name == "App").unwrap();
        let files: Vec<&str> = app.rows.iter().map(|r| r.file.as_str()).collect();
        assert_eq!(files.first(), Some(&"Module.App.ColorGradients"));
        assert_eq!(files.last(), Some(&"Module.App.WebServer"));
        let mut sorted = files.clone();
        sorted.sort();
        assert_eq!(files, sorted);
    }

    #[test]
    fn the_id_text_is_lower_case_with_hyphens() {
        let groups = installed_modules();
        let effects = groups.iter().find(|g| g.name == "Effect").unwrap();
        let alternating = effects.rows.iter().find(|r| r.type_name == "Alternating").unwrap();
        assert_eq!(alternating.type_id_text, "7b791008-56a2-4bff-8ce3-a7fb89ea4637");
        assert_eq!(alternating.file, "Module.Effect.Alternating");
    }

    #[test]
    fn nested_effect_projects_take_the_folder_above_images() {
        assert_eq!(effect_file("Vixen.Modules/Effect/Fixture/FixtureEffect/Images/EffectImage.png"), "Module.Effect.FixtureEffect");
        assert_eq!(effect_file("Vixen.Modules/Effect/Whirlpool/Whirlpool/Images/EffectImage.png"), "Module.Effect.Whirlpool");
    }

    #[test]
    fn generic_timings_id_really_is_empty() {
        let groups = installed_modules();
        let timing = groups.iter().find(|g| g.name == "Timing").unwrap();
        assert_eq!(timing.rows[0].type_id_text, "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn only_timed_sequences_can_be_created() {
        let creatable: Vec<String> = sequence_types().into_iter().filter(|t| t.can_create_new).map(|t| t.type_name).collect();
        assert_eq!(creatable, ["Timed Sequence"]);
    }

    #[test]
    fn the_open_dialog_has_its_four_filters_in_order() {
        let filters = open_panel_filters();
        let labels: Vec<&str> = filters.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(
            labels,
            ["All Sequence Types (*.tim;*.vix;)", "Timed Sequence (*.tim)", "Vixen 2.x Sequence (*.vix)", "All files (*.*)"]
        );
        assert_eq!(filters[0].extensions, ["tim", "vix"]);
        assert_eq!(filters[2].extensions, ["vix"]);
        assert!(filters[3].extensions.is_empty());
    }
}
