//! Which tool panels show and where: `SetDockDefaults`, the View menu's window items, and the Mac layout file that replaces WeifenLuo's XML.
//! F2, F68 to F77. A corrupt file falls back to the defaults, as upstream rebuilds its dock panel.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use vixen_io::file::write_atomic;
use vixen_io::IoError;

/// A tool panel. The timeline isn't one: it's always the centre.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PanelId {
    Effects,
    EffectEditor,
    ColorLibrary,
    CurveLibrary,
    GradientLibrary,
    Marks,
    Layers,
    FindEffects,
}

impl PanelId {
    pub const ALL: [PanelId; 8] = [
        PanelId::Effects,
        PanelId::EffectEditor,
        PanelId::ColorLibrary,
        PanelId::CurveLibrary,
        PanelId::GradientLibrary,
        PanelId::Marks,
        PanelId::Layers,
        PanelId::FindEffects,
    ];

    /// The title its tab shows.
    pub fn title(self) -> &'static str {
        match self {
            PanelId::Effects => "Effects",
            PanelId::EffectEditor => "Effect Editor",
            PanelId::ColorLibrary => "Color Library",
            PanelId::CurveLibrary => "Curve Library",
            PanelId::GradientLibrary => "Gradient Library",
            PanelId::Marks => "Mark Collections",
            PanelId::Layers => "Layers",
            PanelId::FindEffects => "Find Effects/Layers",
        }
    }

    fn key(self) -> &'static str {
        match self {
            PanelId::Effects => "effects",
            PanelId::EffectEditor => "effectEditor",
            PanelId::ColorLibrary => "colorLibrary",
            PanelId::CurveLibrary => "curveLibrary",
            PanelId::GradientLibrary => "gradientLibrary",
            PanelId::Marks => "marks",
            PanelId::Layers => "layers",
            PanelId::FindEffects => "findEffects",
        }
    }

    fn from_key(key: &str) -> Option<PanelId> {
        PanelId::ALL.into_iter().find(|p| p.key() == key)
    }

    /// Where the View menu item shows it when it's reopened. Layer Editor and Find go left, though the default layout puts them right
    /// (KEEP F71, F72).
    pub fn reopen_side(self) -> DockSide {
        match self {
            PanelId::Effects | PanelId::Layers | PanelId::FindEffects => DockSide::Left,
            _ => DockSide::Right,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DockSide {
    Left,
    Right,
}

/// One panel's place. Panels are listed in tab order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PanelPlace {
    pub id: PanelId,
    pub side: DockSide,
    pub visible: bool,
    pub floating: bool,
}

/// The dock layout.
#[derive(Clone, PartialEq, Debug)]
pub struct PanelLayout {
    /// In tab order.
    pub panels: Vec<PanelPlace>,
    pub selected_left: Option<PanelId>,
    pub selected_right: Option<PanelId>,
    pub left_width: f64,
    pub right_width: f64,
}

/// Upstream's `DockLeftPortion` is 150, but the Mac's list needs room for its indent or the names truncate.
const DEFAULT_LEFT_WIDTH: f64 = 200.0;
/// Upstream's `DockRightPortion` is 150 too. `Editor.png` shows about 340 once the effect editor's sliders are in it.
const DEFAULT_RIGHT_WIDTH: f64 = 340.0;

impl Default for PanelLayout {
    /// `SetDockDefaults`: Effects left; on the right Color, Curve and Gradient Library, Marks, Layer Editor and Find Effects, with the
    /// Effect Editor added at load, so it's the last tab, and the one showing.
    fn default() -> Self {
        let right = [
            PanelId::ColorLibrary,
            PanelId::CurveLibrary,
            PanelId::GradientLibrary,
            PanelId::Marks,
            PanelId::Layers,
            PanelId::FindEffects,
            PanelId::EffectEditor,
        ];
        let mut panels = vec![PanelPlace { id: PanelId::Effects, side: DockSide::Left, visible: true, floating: false }];
        panels.extend(right.into_iter().map(|id| PanelPlace { id, side: DockSide::Right, visible: true, floating: false }));
        PanelLayout {
            panels,
            selected_left: Some(PanelId::Effects),
            selected_right: Some(PanelId::EffectEditor),
            left_width: DEFAULT_LEFT_WIDTH,
            right_width: DEFAULT_RIGHT_WIDTH,
        }
    }
}

impl PanelLayout {
    pub fn place(&self, id: PanelId) -> Option<&PanelPlace> {
        self.panels.iter().find(|p| p.id == id)
    }

    /// The View menu's check: F77.
    pub fn is_visible(&self, id: PanelId) -> bool {
        self.place(id).is_some_and(|p| p.visible)
    }

    /// Tabs on one side, in order.
    pub fn tabs(&self, side: DockSide) -> Vec<PanelId> {
        self.panels.iter().filter(|p| p.side == side && p.visible && !p.floating).map(|p| p.id).collect()
    }

    /// `HandleDockContentToolStripMenuClick`: hidden or never shown, it shows at its menu's side and comes to the front; otherwise it closes.
    /// On the Mac a panel hides rather than disposes, so what's in it survives.
    pub fn toggle(&mut self, id: PanelId) {
        if self.is_visible(id) {
            self.hide(id);
        } else {
            self.show(id);
        }
    }

    pub fn hide(&mut self, id: PanelId) {
        let Some(index) = self.panels.iter().position(|p| p.id == id) else { return };
        self.panels[index].visible = false;
        let side = self.panels[index].side;
        if self.selected(side) == Some(id) {
            // The neighbouring tab shows next.
            let next = self.tabs(side).first().copied();
            self.set_selected(side, next);
        }
    }

    pub fn show(&mut self, id: PanelId) {
        let side = id.reopen_side();
        let Some(index) = self.panels.iter().position(|p| p.id == id) else { return };
        let mut place = self.panels.remove(index);
        place.visible = true;
        place.side = side;
        // Placed at the end of that side's tabs.
        self.panels.push(place);
        if !place.floating {
            self.set_selected(side, Some(id));
        }
    }

    /// The detach control, and closing a floating panel's window back into the dock.
    pub fn set_floating(&mut self, id: PanelId, floating: bool) {
        let Some(index) = self.panels.iter().position(|p| p.id == id) else { return };
        self.panels[index].floating = floating;
        let side = self.panels[index].side;
        if floating {
            if self.selected(side) == Some(id) {
                let next = self.tabs(side).first().copied();
                self.set_selected(side, next);
            }
        } else {
            self.set_selected(side, Some(id));
        }
    }

    pub fn selected(&self, side: DockSide) -> Option<PanelId> {
        match side {
            DockSide::Left => self.selected_left,
            DockSide::Right => self.selected_right,
        }
    }

    pub fn set_selected(&mut self, side: DockSide, id: Option<PanelId>) {
        match side {
            DockSide::Left => self.selected_left = id,
            DockSide::Right => self.selected_right = id,
        }
    }

    fn to_json(&self) -> Value {
        let side = |s: DockSide| if s == DockSide::Left { "left" } else { "right" };
        json!({
            "version": 1,
            "leftWidth": self.left_width,
            "rightWidth": self.right_width,
            "selectedLeft": self.selected_left.map(PanelId::key),
            "selectedRight": self.selected_right.map(PanelId::key),
            "panels": self.panels.iter().map(|p| json!({
                "id": p.id.key(),
                "side": side(p.side),
                "visible": p.visible,
                "floating": p.floating,
            })).collect::<Vec<_>>(),
        })
    }

    fn from_json(value: &Value) -> Option<PanelLayout> {
        let mut panels = Vec::new();
        for item in value.get("panels")?.as_array()? {
            let id = PanelId::from_key(item.get("id")?.as_str()?)?;
            let side = match item.get("side")?.as_str()? {
                "left" => DockSide::Left,
                "right" => DockSide::Right,
                _ => return None,
            };
            let visible = item.get("visible")?.as_bool()?;
            let floating = item.get("floating").and_then(Value::as_bool).unwrap_or(false);
            if panels.iter().any(|p: &PanelPlace| p.id == id) {
                return None;
            }
            panels.push(PanelPlace { id, side, visible, floating });
        }
        // `Form.cs:337-345`: a saved layout that doesn't know a panel gets it docked at the default place.
        let defaults = PanelLayout::default();
        for place in &defaults.panels {
            if !panels.iter().any(|p: &PanelPlace| p.id == place.id) {
                panels.push(*place);
            }
        }
        let panel = |key: &str| value.get(key).and_then(Value::as_str).and_then(PanelId::from_key);
        Some(PanelLayout {
            panels,
            selected_left: panel("selectedLeft"),
            selected_right: panel("selectedRight"),
            left_width: value.get("leftWidth").and_then(Value::as_f64).unwrap_or(defaults.left_width),
            right_width: value.get("rightWidth").and_then(Value::as_f64).unwrap_or(defaults.right_width),
        })
    }
}

pub fn layout_path(dir: &Path) -> PathBuf {
    dir.join("TimedSequenceEditorLayout.json")
}

/// A missing or corrupt file gives the defaults.
pub fn load_layout(dir: &Path) -> PanelLayout {
    std::fs::read(layout_path(dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| PanelLayout::from_json(&value))
        .unwrap_or_default()
}

pub fn save_layout(dir: &Path, layout: &PanelLayout) -> Result<(), IoError> {
    std::fs::create_dir_all(dir).map_err(|source| IoError::Io { path: dir.to_path_buf(), source })?;
    let text = serde_json::to_string_pretty(&layout.to_json()).unwrap_or_default();
    write_atomic(&layout_path(dir), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-panels-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_order_matches_set_dock_defaults() {
        let l = PanelLayout::default();
        assert_eq!(l.tabs(DockSide::Left), [PanelId::Effects]);
        assert_eq!(
            l.tabs(DockSide::Right),
            [
                PanelId::ColorLibrary,
                PanelId::CurveLibrary,
                PanelId::GradientLibrary,
                PanelId::Marks,
                PanelId::Layers,
                PanelId::FindEffects,
                PanelId::EffectEditor,
            ]
        );
        assert_eq!(l.selected(DockSide::Right), Some(PanelId::EffectEditor));
    }

    #[test]
    fn view_items_hide_then_reopen_on_their_menu_side() {
        let mut l = PanelLayout::default();
        l.toggle(PanelId::Layers);
        assert!(!l.is_visible(PanelId::Layers));
        assert!(!l.tabs(DockSide::Right).contains(&PanelId::Layers));
        // Reopened Layer Editor and Find go left (KEEP).
        l.toggle(PanelId::Layers);
        assert!(l.is_visible(PanelId::Layers));
        assert_eq!(l.tabs(DockSide::Left), [PanelId::Effects, PanelId::Layers]);
        assert_eq!(l.selected(DockSide::Left), Some(PanelId::Layers));
        l.toggle(PanelId::FindEffects);
        l.toggle(PanelId::FindEffects);
        assert_eq!(l.tabs(DockSide::Left), [PanelId::Effects, PanelId::Layers, PanelId::FindEffects]);
        // Others go right.
        l.toggle(PanelId::ColorLibrary);
        l.toggle(PanelId::ColorLibrary);
        assert_eq!(*l.tabs(DockSide::Right).last().unwrap(), PanelId::ColorLibrary);
    }

    #[test]
    fn hiding_the_showing_tab_shows_another() {
        let mut l = PanelLayout::default();
        l.toggle(PanelId::EffectEditor);
        assert_eq!(l.selected(DockSide::Right), Some(PanelId::ColorLibrary));
        l.set_selected(DockSide::Left, Some(PanelId::Effects));
        l.toggle(PanelId::Effects);
        assert_eq!(l.selected(DockSide::Left), None);
    }

    #[test]
    fn floating_panels_leave_the_tabs() {
        let mut l = PanelLayout::default();
        l.set_floating(PanelId::EffectEditor, true);
        assert!(!l.tabs(DockSide::Right).contains(&PanelId::EffectEditor));
        assert!(l.is_visible(PanelId::EffectEditor));
        l.set_floating(PanelId::EffectEditor, false);
        assert!(l.tabs(DockSide::Right).contains(&PanelId::EffectEditor));
    }

    #[test]
    fn round_trips_and_a_corrupt_file_gives_defaults() {
        let dir = temp("trip");
        let mut l = PanelLayout::default();
        l.toggle(PanelId::Layers);
        l.left_width = 222.0;
        save_layout(&dir, &l).unwrap();
        assert_eq!(load_layout(&dir), l);
        std::fs::write(layout_path(&dir), "not json").unwrap();
        assert_eq!(load_layout(&dir), PanelLayout::default());
    }

    #[test]
    fn a_layout_missing_a_panel_gets_it_docked() {
        let dir = temp("missing");
        std::fs::write(layout_path(&dir), r#"{"panels":[{"id":"effects","side":"left","visible":true}]}"#).unwrap();
        let l = load_layout(&dir);
        assert!(l.is_visible(PanelId::EffectEditor));
        assert!(l.tabs(DockSide::Right).contains(&PanelId::Layers));
    }
}
