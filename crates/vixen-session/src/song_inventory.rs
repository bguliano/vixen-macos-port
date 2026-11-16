//! Singing Faces inventory loading, parsing and browser state. Network is injected;
//! no parser or test opens a connection.
use std::collections::HashMap;
use vixen_xml::XElement;
pub const VENDOR_URL: &str = "https://app.vixenlights.com/singing-faces-vendor.json";
pub const USER_AGENT: &str = "Mozilla/5.0 (compatible; Vixen/1.0)";
pub const TIMEOUT_SECONDS: u32 = 20;
pub trait InventoryTransport: Send + Sync {
    fn get(&self, url: &str) -> Result<String, String>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendorLink {
    pub name: String,
    pub url: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub creator: String,
    pub category_id: String,
    pub link: Option<String>,
    pub download: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    pub name: String,
    pub id: String,
    pub categories: Vec<Category>,
    pub songs: Vec<Song>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub name: String,
    pub website: Option<String>,
    pub logo: Option<String>,
    pub categories: Vec<Category>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryError {
    pub title: String,
    pub message: String,
    pub detail: String,
}
pub fn parse_vendor_list(text: &str) -> Result<Vec<VendorLink>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    v.as_array()
        .ok_or("Vendor list must be a JSON array.")?
        .iter()
        .map(|v| {
            Ok(VendorLink {
                name: v
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
                url: v
                    .get("Url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
            })
        })
        .collect()
}
/// Absolute URI syntax, including non-HTTP links, as the upstream Uri constructor
/// allows. HTTP(S) also requires an authority; network errors remain transport errors.
fn absolute_uri(value: &str) -> Option<String> {
    let value = value.trim();
    let (scheme, tail) = value.split_once(':')?;
    if scheme.is_empty()
        || !scheme.as_bytes()[0].is_ascii_alphabetic()
        || !scheme
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
        || tail.is_empty()
        || value.chars().any(|c| c.is_control())
    {
        return None;
    }
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        let authority = tail.strip_prefix("//")?.split(['/', '?', '#']).next()?;
        if authority.is_empty() || authority.chars().any(char::is_whitespace) {
            return None;
        }
    }
    Some(value.to_owned())
}
pub fn parse_inventory(text: &str) -> Result<Inventory, String> {
    let root = XElement::parse(text).map_err(|e| e.to_string())?;
    if root.local != "musicinventory" {
        return Err("Expected musicinventory root.".into());
    }
    let vendor = root.child("vendor").ok_or("Inventory has no vendor.")?;
    let music = root.child("music").ok_or("Inventory has no music.")?;
    let categories = root
        .child("categories")
        .ok_or("Inventory has no categories.")?;
    let required_uri = |name: &str| -> Result<Option<String>, String> {
        let value = vendor.child_text(name).unwrap_or_default();
        if value.is_empty() {
            Ok(None)
        } else {
            absolute_uri(&value)
                .map(Some)
                .ok_or_else(|| format!("Invalid vendor {name}."))
        }
    };
    let mut songs: HashMap<String, Vec<Song>> = HashMap::new();
    for song in music.elements().filter(|s| s.local == "song") {
        if ["artist", "title", "creator", "categoryid"]
            .iter()
            .any(|key| song.child(key).is_some_and(|v| v.nil))
        {
            continue;
        }
        let text = |key: &str| song.child_text(key).unwrap_or_default().trim().to_owned();
        let optional_uri = |key: &str| {
            let value = text(key);
            let uri = absolute_uri(&value);
            if !value.is_empty() && uri.is_none() {
                crate::logging::info(
                    "VixenModules.App.TimingTrackBrowser",
                    &format!("Invalid {key} for song '{}': {value}", text("title")),
                );
            }
            uri
        };
        let row = Song {
            title: text("title"),
            artist: text("artist"),
            creator: text("creator"),
            category_id: text("categoryid"),
            link: optional_uri("weblink"),
            download: optional_uri("download"),
        };
        songs.entry(row.category_id.clone()).or_default().push(row);
    }
    for rows in songs.values_mut() {
        crate::collation::sort_by_name(rows, |s| &s.title);
    }
    fn read_categories(
        parent: &XElement,
        songs: &HashMap<String, Vec<Song>>,
        depth: usize,
    ) -> Result<Vec<Category>, String> {
        if depth > 128 {
            return Err("Inventory category nesting is too deep.".into());
        }
        parent
            .elements()
            .filter(|e| e.local == "category")
            .map(|c| {
                let id = c.child_text("id").ok_or("Category has no id.")?;
                Ok(Category {
                    name: c.child_text("name").unwrap_or_default(),
                    songs: songs.get(&id).cloned().unwrap_or_default(),
                    id,
                    categories: match c.child("categories") {
                        Some(children) => read_categories(children, songs, depth + 1)?,
                        None => vec![],
                    },
                })
            })
            .collect()
    }
    Ok(Inventory {
        name: vendor.child_text("name").unwrap_or_default(),
        website: required_uri("website")?,
        logo: required_uri("logolink")?,
        categories: read_categories(categories, &songs, 0)?,
    })
}
pub fn load_inventories(
    transport: &dyn InventoryTransport,
    url: &str,
    mut on_error: impl FnMut(InventoryError),
) -> Vec<Inventory> {
    let vendors = match transport.get(url).and_then(|s| parse_vendor_list(&s)) {
        Ok(v) => v,
        Err(detail) => {
            on_error(InventoryError{title:"Error Retrieving Vendors".into(),message:"Unable to retrieve singing faces vendor list. Ensure you have an active internet connection.".into(),detail});
            return vec![];
        }
    };
    let mut inventories = Vec::new();
    for vendor in vendors {
        match absolute_uri(&vendor.url).ok_or_else(||"Invalid inventory URL.".into()).and_then(|url|transport.get(&url)).and_then(|text|parse_inventory(&text)) {
            Ok(inventory)=>inventories.push(inventory),Err(detail)=>on_error(InventoryError{title:"Error Retrieving Inventory".into(),message:format!("Unable to retrieve inventory from {}\nEnsure you have an active internet connection.",vendor.name),detail}),
        }
    }
    inventories
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub path: String,
    pub name: String,
    pub song: bool,
    pub child_count: usize,
}
#[derive(Default)]
pub struct Browser {
    pub inventories: Vec<Inventory>,
    pub provider: Option<usize>,
    pub selected: Option<Song>,
}
impl Browser {
    pub fn set_inventories(&mut self, inventory: Vec<Inventory>) {
        self.inventories = inventory;
        self.provider = if self.inventories.is_empty() {
            None
        } else {
            Some(0)
        };
        self.selected = None;
    }
    pub fn select_provider(&mut self, index: usize) {
        if index < self.inventories.len() {
            self.provider = Some(index);
        }
    }
    fn category(&self, path: &str) -> Option<&Category> {
        let mut rows = &self.inventories.get(self.provider?)?.categories;
        let mut found = None;
        for part in path.split('/').filter(|s| !s.is_empty()) {
            let index = part.strip_prefix('c')?.parse::<usize>().ok()?;
            let c = rows.get(index)?;
            found = Some(c);
            rows = &c.categories;
        }
        found
    }
    pub fn rows(&self, parent: Option<&str>) -> Vec<TreeRow> {
        let parent = parent.unwrap_or("");
        let (categories, songs) = if parent.is_empty() {
            match self.provider.and_then(|p| self.inventories.get(p)) {
                Some(i) => (&i.categories[..], &[][..]),
                None => return vec![],
            }
        } else {
            match self.category(parent) {
                Some(c) => (&c.categories[..], &c.songs[..]),
                None => return vec![],
            }
        };
        let prefix = if parent.is_empty() {
            String::new()
        } else {
            format!("{parent}/")
        };
        categories
            .iter()
            .enumerate()
            .map(|(i, c)| TreeRow {
                path: format!("{prefix}c{i}"),
                name: c.name.clone(),
                song: false,
                child_count: c.categories.len() + c.songs.len(),
            })
            .chain(songs.iter().enumerate().map(|(i, s)| TreeRow {
                path: format!("{prefix}s{i}"),
                name: s.title.clone(),
                song: true,
                child_count: 0,
            }))
            .collect()
    }
    pub fn select_song(&mut self, path: &str) -> bool {
        let Some((parent, index)) = path.rsplit_once('/') else {
            return false;
        };
        let Some(index) = index
            .strip_prefix('s')
            .and_then(|s| s.parse::<usize>().ok())
        else {
            return false;
        };
        let Some(song) = self
            .category(parent)
            .and_then(|c| c.songs.get(index))
            .cloned()
        else {
            return false;
        };
        self.selected = Some(song);
        true
    }
    pub fn download(&self, transport: &dyn InventoryTransport) -> Result<String, String> {
        let selected = self.selected.as_ref().ok_or("No song is selected.")?;
        // KEEP: Select is enabled without a URL; the marks importer receives empty XML and reports F33.
        selected
            .download
            .as_ref()
            .map_or(Ok(String::new()), |url| transport.get(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = r#"<musicinventory><vendor><name>Vendor</name><website>https://example.test/</website></vendor><categories><category><id>root</id><name>Root</name><categories><category><id>child</id><name>Child</name></category></categories></category></categories><music><song><title> Zebra </title><artist> A </artist><creator> C </creator><categoryid>root</categoryid><download>https://example.test/z.xml</download></song><song><title>Alpha</title><categoryid>root</categoryid><weblink>bad link</weblink></song><song><title>Child Song</title><categoryid>child</categoryid></song><song><title>Hidden</title><categoryid>unknown</categoryid></song></music></musicinventory>"#;
    struct Fake(HashMap<String, Result<String, String>>);
    impl InventoryTransport for Fake {
        fn get(&self, url: &str) -> Result<String, String> {
            self.0
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err("unexpected request".into()))
        }
    }
    #[test]
    fn parse_hierarchy_and_links() {
        let i = parse_inventory(XML).unwrap();
        assert_eq!(
            i.categories[0]
                .songs
                .iter()
                .map(|s| s.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Zebra"]
        );
        assert_eq!(i.categories[0].songs[1].artist, "A");
        assert_eq!(i.categories[0].songs[0].link, None);
        assert_eq!(i.categories[0].categories[0].songs.len(), 1);
    }
    #[test]
    fn required_vendor_fields_and_nested_category_guard() {
        for xml in [
            XML.replace("https://example.test/", "bad link"),
            XML.replace("<music>", "<other>")
                .replace("</music>", "</other>"),
            XML.replace("<vendor>", "<other>")
                .replace("</vendor>", "</other>"),
        ] {
            assert!(parse_inventory(&xml).is_err());
        }
        assert!(parse_inventory(XML).is_ok());
    }
    #[test]
    fn failed_vendor_does_not_hide_later_inventory() {
        let list = r#"[{"Name":"Bad","Url":"https://bad.test"},{"Name":"Good","Url":"https://good.test"}]"#;
        let fake = Fake(HashMap::from([
            ("list".into(), Ok(list.into())),
            ("https://bad.test".into(), Err("timeout".into())),
            ("https://good.test".into(), Ok(XML.into())),
        ]));
        let mut errors = vec![];
        let i = load_inventories(&fake, "list", |e| errors.push(e));
        assert_eq!(i.len(), 1);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Bad\n"));
    }
    #[test]
    fn list_failure_and_no_download_are_reported_to_the_right_owner() {
        let fake = Fake(HashMap::new());
        let mut errors = vec![];
        assert!(load_inventories(&fake, "missing", |e| errors.push(e)).is_empty());
        assert_eq!(errors[0].title, "Error Retrieving Vendors");
        let mut b = Browser::default();
        b.set_inventories(vec![parse_inventory(XML).unwrap()]);
        assert!(b.select_song("c0/s0"));
        assert_eq!(b.download(&fake).unwrap(), "");
    }
    #[test]
    fn category_children_precede_songs_and_provider_retains_song() {
        let i = parse_inventory(XML).unwrap();
        let mut b = Browser::default();
        b.set_inventories(vec![i.clone(), i]);
        let rows = b.rows(Some("c0"));
        assert_eq!(
            rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            vec!["c0/c0", "c0/s0", "c0/s1"]
        );
        assert!(b.select_song("c0/c0/s0"));
        b.select_provider(1);
        assert_eq!(b.selected.as_ref().unwrap().title, "Child Song");
        assert!(!b.select_song("invalid"));
    }
}
