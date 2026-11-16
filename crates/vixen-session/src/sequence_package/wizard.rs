use super::*;

pub struct PackageWizard {
    pub importing: bool,
    pub stage: u32,
    pub settings: ExportSettings,
    pub input: String,
    pub map: String,
    pub index: Option<PackageIndex>,
    pub running: bool,
    pub final_text: String,
    pub result: Option<RunResult>,
    paths: DataPaths,
    profile: Option<String>,
}
impl PackageWizard {
    pub fn new(importing: bool, core: &AppCore) -> Self {
        let mut settings = ExportSettings::load(&core.system.read().module_store, &core.paths);
        settings.sequences.retain(|p| Path::new(p).is_file());
        Self {
            importing,
            stage: 0,
            settings,
            input: String::new(),
            map: String::new(),
            index: None,
            running: false,
            final_text: String::new(),
            result: None,
            paths: core.paths.clone(),
            profile: core.profile_name.clone(),
        }
    }
    pub fn can_next(&self) -> bool {
        if self.running {
            return false;
        }
        match (self.importing, self.stage) {
            (false, 0) => !self.settings.sequences.is_empty(),
            (false, 1) => self.settings.output_valid(),
            (true, 0) => {
                self.index.is_some()
                    && Path::new(&self.input).is_file()
                    && Path::new(&self.map).is_file()
                    && self.map.ends_with("vmap")
            }
            (true, 1) => self
                .index
                .as_ref()
                .is_some_and(|i| i.sequences.iter().any(|s| s.selected)),
            (_, 2) => true,
            _ => false,
        }
    }
    pub fn next(&mut self) -> bool {
        if !self.can_next() || self.stage >= 2 {
            return false;
        }
        self.stage += 1;
        if !self.importing && self.stage == 1 {
            self.settings
                .enter_output(&self.paths, self.profile.as_deref());
        }
        true
    }
    pub fn previous(&mut self) {
        if !self.running && (1..=2).contains(&self.stage) {
            self.stage -= 1;
            if !self.importing && self.stage == 0 {
                self.settings.sequences.retain(|p| Path::new(p).is_file());
            }
        }
    }
    pub fn add(&mut self, paths: Vec<String>) -> bool {
        if self.running {
            return false;
        }
        let mut foreign = false;
        for p in paths {
            let path = Path::new(&p);
            if !path.is_file() {
                continue;
            }
            foreign |= !path.starts_with(self.paths.sequence_dir());
            if !self.settings.sequences.contains(&p) {
                self.settings.sequences.push(p);
            }
        }
        foreign
    }
    pub fn add_all(&mut self) {
        let paths = crate::export_wizard::all_sequence_files(&self.paths, true)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        self.add(paths);
    }
    pub fn remove(&mut self, rows: &[usize]) {
        if self.running {
            return;
        }
        let remove: HashSet<_> = rows.iter().copied().collect();
        let mut row = 0;
        self.settings.sequences.retain(|_| {
            let keep = !remove.contains(&row);
            row += 1;
            keep
        });
    }
    pub fn set_input(&mut self, path: String) -> Result<(), SessionError> {
        if self.running {
            return Err(error("Package operation is running."));
        }
        let index = inspect(Path::new(&path))?;
        if path != self.input {
            self.index = Some(index);
        }
        self.input = path;
        Ok(())
    }
    pub fn set_selected(&mut self, rows: &[usize], selected: bool) {
        if self.running {
            return;
        }
        if let Some(index) = &mut self.index {
            for row in rows {
                if let Some(s) = index.sequences.get_mut(*row) {
                    s.selected = selected;
                }
            }
        }
    }
    pub fn selected_names(&self) -> Vec<String> {
        self.index
            .as_ref()
            .map(|i| {
                i.sequences
                    .iter()
                    .filter(|s| s.selected)
                    .map(|s| s.name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn finish(&mut self, result: RunResult) {
        self.final_text = if result.cancelled {
            if self.importing {
                "Import Canceled"
            } else {
                "Export Canceled"
            }
            .into()
        } else if !result.errors.is_empty() {
            if self.importing {
                "Import Finished with Errors"
            } else {
                "Export Failed"
            }
            .into()
        } else if self.importing {
            "Package Import Complete".into()
        } else {
            "Package Export Complete".into()
        };
        self.result = Some(result);
        self.running = false;
        self.stage = 3;
    }
    pub fn mapper(&self, core: &AppCore, edit: bool) -> Result<ElementMapState, SessionError> {
        let index = inspect(Path::new(&self.input))?;
        let mut map = ElementMapState::new(
            "New Map".into(),
            vec![],
            Arc::new(core.system.read().tree.clone()),
        );
        if edit {
            map.load(Path::new(&self.map))?;
        }
        if !map.has_source_tree() {
            if let Some(tree) = index.tree {
                map.load_tree_json(&tree)?;
            }
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_survives_back_and_changes_reset_for_new_package() {
        let f = super::super::tests::Fixture::new();
        let a = f.root.join("a.vpkg");
        let b = f.root.join("b.vpkg");
        super::super::tests::archive(&a, &[("Sequence/A.tim", b"x"), ("Sequence/B.tim", b"x")]);
        super::super::tests::archive(&b, &[("Sequence/C.tim", b"x")]);
        let mut w = PackageWizard::new(true, &f.core);
        w.set_input(a.to_string_lossy().into()).unwrap();
        w.set_selected(&[0], false);
        w.stage = 2;
        w.previous();
        assert_eq!(w.selected_names(), vec!["B"]);
        w.set_input(b.to_string_lossy().into()).unwrap();
        assert_eq!(w.selected_names(), vec!["C"]);
    }
}
