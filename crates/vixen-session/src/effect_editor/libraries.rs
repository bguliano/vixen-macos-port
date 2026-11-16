//! The curve and gradient libraries: `CurveLibrary` and `ColorGradientLibrary`. F84 to F86, F93, F97, and what links do. The libraries stay in the
//! module store's XML, so a save keeps unknown content; each change writes them back, updates the sequences open now, and saves off the main thread.

use crate::app::AppCore;
use crate::library_links::Libraries;
use std::sync::Arc;
use vixen_core::{ColorGradient, Curve};
use vixen_io::{curve_library, gradient_library, set_curve_library, set_gradient_library, CurveLibrary, GradientLibrary};

/// `name`, then `name 2`, `name 3`... whichever isn't taken (F84).
pub fn unique_import_name(existing: &[String], name: &str) -> String {
    if !existing.iter().any(|e| e == name) {
        return name.to_string();
    }
    let mut i = 2;
    loop {
        let candidate = format!("{name} {i}");
        if !existing.iter().any(|e| *e == candidate) {
            return candidate;
        }
        i += 1;
    }
}

/// F82's reorder, the Move branch of `listViewCurves_DragDrop`: every selected name, in list order, is taken out and put back at the target's index, in reverse when the first selected is
/// right of the target; with no target, at the last index. The index is the target's before anything moved, as upstream's is, so a move to the right lands after the target.
pub fn moved_names(order: &[String], selected: &[String], target: Option<&str>) -> Vec<String> {
    let mut list = order.to_vec();
    let mut items: Vec<String> = list.iter().filter(|n| selected.contains(n)).cloned().collect();
    let Some(first) = items.first().and_then(|f| list.iter().position(|n| n == f)) else { return list };
    let target_index = target.and_then(|t| list.iter().position(|n| n == t));
    if target_index.is_some_and(|t| first > t) {
        items.reverse();
    }
    let index = target_index.unwrap_or(list.len() - 1);
    for item in items {
        if let Some(i) = list.iter().position(|n| *n == item) {
            list.remove(i);
        }
        let at = index.min(list.len());
        list.insert(at, item);
    }
    list
}

/// F82's drop of a value: the written entry goes to the target item's index, or stays last. FIX: upstream moves whichever item is last, which is wrong when the name overwrote an entry.
fn placed<T>(entries: &mut Vec<(String, T)>, name: &str, index: Option<usize>) {
    let Some(index) = index else { return };
    let Some(from) = entries.iter().position(|(n, _)| n == name) else { return };
    let entry = entries.remove(from);
    let at = index.min(entries.len());
    entries.insert(at, entry);
}

/// `AddCurveAsync`'s treatment of what goes in: marked current, its link cleared. An empty name is refused.
fn add_curve_entry(library: &mut CurveLibrary, name: &str, mut curve: Curve) -> bool {
    curve.is_current_library_curve = true;
    curve.library_reference_name.clear();
    match library.curves.iter_mut().find(|(n, _)| n == name) {
        Some(entry) => {
            entry.1 = curve;
            true
        }
        None => {
            library.curves.push((name.to_string(), curve));
            false
        }
    }
}

fn add_gradient_entry(library: &mut GradientLibrary, name: &str, mut gradient: ColorGradient) -> bool {
    gradient.is_current_library_gradient = true;
    gradient.library_reference_name.clear();
    match library.gradients.iter_mut().find(|(n, _)| n == name) {
        Some(entry) => {
            entry.1 = gradient;
            true
        }
        None => {
            library.gradients.push((name.to_string(), gradient));
            false
        }
    }
}

impl AppCore {
    pub fn library_curves(&self) -> CurveLibrary {
        curve_library(&self.system.read().module_store)
    }

    pub fn library_gradients(&self) -> GradientLibrary {
        gradient_library(&self.system.read().module_store)
    }

    /// Writes a changed curve library, and lets the open sequences know.
    fn store_curves(&self, library: &CurveLibrary) {
        let gradients = {
            let mut system = self.system.write();
            set_curve_library(&mut system.module_store, library);
            gradient_library(&system.module_store)
        };
        self.update_open_sequences(library, &gradients);
    }

    fn store_gradients(&self, library: &GradientLibrary) {
        let curves = {
            let mut system = self.system.write();
            set_gradient_library(&mut system.module_store, library);
            curve_library(&system.module_store)
        };
        self.update_open_sequences(&curves, library);
    }

    fn update_open_sequences(&self, curves: &CurveLibrary, gradients: &GradientLibrary) {
        let libraries = Libraries { curves: curves.clone(), gradients: gradients.clone() };
        for session in self.open_sequence_sessions() {
            session.lock().libraries_changed(libraries.clone());
        }
    }

    /// `AddCurveAsync`: true if the name was already in the library. The old entry's links read the new points.
    pub fn add_library_curve(&self, name: &str, curve: Curve) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut library = self.library_curves();
        let existed = add_curve_entry(&mut library, name, curve);
        self.store_curves(&library);
        existed
    }

    /// `AddColorGradientAsync`.
    pub fn add_library_gradient(&self, name: &str, gradient: ColorGradient) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut library = self.library_gradients();
        let existed = add_gradient_entry(&mut library, name, gradient);
        self.store_gradients(&library);
        existed
    }

    /// F82: a curve dropped on the Curve Library window, named by the user, goes at the target item's index (`None` for the end). True if the name was already there.
    pub fn insert_library_curve(&self, name: &str, curve: Curve, index: Option<usize>) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut library = self.library_curves();
        let existed = add_curve_entry(&mut library, name, curve);
        placed(&mut library.curves, name, index);
        self.store_curves(&library);
        existed
    }

    pub fn insert_library_gradient(&self, name: &str, gradient: ColorGradient, index: Option<usize>) -> bool {
        if name.is_empty() {
            return false;
        }
        let mut library = self.library_gradients();
        let existed = add_gradient_entry(&mut library, name, gradient);
        placed(&mut library.gradients, name, index);
        self.store_gradients(&library);
        existed
    }

    /// F82's reorder by dragging within the window: see `moved_names`. Only the window's own drags come here (GUARD).
    pub fn move_library_curves(&self, selected: &[String], target: Option<&str>) {
        let order: Vec<String> = self.library_curves().curves.iter().map(|(n, _)| n.clone()).collect();
        self.reorder_library_curves(&moved_names(&order, selected, target));
    }

    pub fn move_library_gradients(&self, selected: &[String], target: Option<&str>) {
        let order: Vec<String> = self.library_gradients().gradients.iter().map(|(n, _)| n.clone()).collect();
        self.reorder_library_gradients(&moved_names(&order, selected, target));
    }

    /// `RemoveCurveAsync` in one bulk update. Links to a removed entry become independent curves (F81).
    pub fn remove_library_curves(&self, names: &[String]) {
        let mut library = self.library_curves();
        library.curves.retain(|(n, _)| !names.contains(n));
        self.store_curves(&library);
    }

    pub fn remove_library_gradients(&self, names: &[String]) {
        let mut library = self.library_gradients();
        library.gradients.retain(|(n, _)| !names.contains(n));
        self.store_gradients(&library);
    }

    /// F82's rewrite in list order. A name not given keeps its relative place after those that are; an unknown name is ignored.
    pub fn reorder_library_curves(&self, names: &[String]) {
        let library = self.library_curves();
        let mut reordered = CurveLibrary::default();
        for name in names {
            if let Some(entry) = library.curves.iter().find(|(n, _)| n == name) {
                reordered.curves.push(entry.clone());
            }
        }
        for entry in &library.curves {
            if !names.contains(&entry.0) {
                reordered.curves.push(entry.clone());
            }
        }
        self.store_curves(&reordered);
    }

    pub fn reorder_library_gradients(&self, names: &[String]) {
        let library = self.library_gradients();
        let mut reordered = GradientLibrary::default();
        for name in names {
            if let Some(entry) = library.gradients.iter().find(|(n, _)| n == name) {
                reordered.gradients.push(entry.clone());
            }
        }
        for entry in &library.gradients {
            if !names.contains(&entry.0) {
                reordered.gradients.push(entry.clone());
            }
        }
        self.store_gradients(&reordered);
    }
}

/// `SaveModuleConfigAsync` off the calling thread. A save already running makes the first try give up, so this waits its turn: the last change's save writes the final state.
pub fn save_libraries(core: &Arc<AppCore>) {
    let core = Arc::clone(core);
    let _ = std::thread::Builder::new().name("vixen-library-save".to_string()).spawn(move || {
        for _ in 0..400 {
            match core.save_module_config() {
                Ok(true) => return,
                Ok(false) => std::thread::sleep(std::time::Duration::from_millis(25)),
                Err(error) => {
                    crate::logging::error("Vixen.Libraries", &format!("Couldn't save the libraries: {error}"), None);
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names() {
        let existing = vec!["A".to_string(), "A 2".to_string()];
        assert_eq!(unique_import_name(&existing, "A"), "A 3");
        assert_eq!(unique_import_name(&existing, "B"), "B");
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn moving_right_lands_after_the_target_and_left_before_it() {
        let order = names(&["A", "B", "C", "D"]);
        assert_eq!(moved_names(&order, &names(&["A"]), Some("C")), names(&["B", "C", "A", "D"]));
        assert_eq!(moved_names(&order, &names(&["D"]), Some("B")), names(&["A", "D", "B", "C"]));
        // Several, moving left: reversed first, so they keep their order.
        assert_eq!(moved_names(&order, &names(&["C", "D"]), Some("A")), names(&["C", "D", "A", "B"]));
        // No target: the last index.
        assert_eq!(moved_names(&order, &names(&["A", "B"]), None), names(&["C", "D", "A", "B"]));
        // Nothing selected: nothing moves.
        assert_eq!(moved_names(&order, &[], Some("A")), order);
    }

    #[test]
    fn a_dropped_entry_goes_at_the_target_and_an_overwrite_moves_the_written_one() {
        let mut entries: Vec<(String, u8)> = vec![("A".into(), 0), ("B".into(), 0), ("C".into(), 0)];
        entries.push(("New".into(), 1));
        placed(&mut entries, "New", Some(1));
        assert_eq!(entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["A", "New", "B", "C"]);
        // Overwriting C, dropped on A: C moves, not whichever is last.
        placed(&mut entries, "C", Some(0));
        assert_eq!(entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["C", "A", "New", "B"]);
        placed(&mut entries, "A", None);
        assert_eq!(entries.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["C", "A", "New", "B"]);
    }

    #[test]
    fn adding_marks_the_entry_current_and_clears_any_link() {
        let mut library = CurveLibrary::default();
        let mut curve = Curve::flat(10.0);
        curve.library_reference_name = "Other".into();
        assert!(!add_curve_entry(&mut library, "One", curve));
        let entry = library.get("One").unwrap();
        assert!(entry.is_current_library_curve);
        assert!(!entry.is_library_reference());
        // Overwriting keeps its place.
        library.curves.push(("Two".into(), Curve::flat(20.0)));
        assert!(add_curve_entry(&mut library, "One", Curve::flat(30.0)));
        assert_eq!(library.curves[0].0, "One");
        assert_eq!(library.curves[0].1.points[0].1, 30.0);
    }
}
