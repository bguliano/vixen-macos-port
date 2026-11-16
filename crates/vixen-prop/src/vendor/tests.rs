use super::*;
use std::collections::HashMap;
const INVENTORY: &str = r#"<modelinventory><vendor><name>Vendor A</name><contact>Team</contact><email>support@example.test</email><phone>555</phone><website>https://example.test/</website><facebook>https://example.test/social</facebook><notes>Vendor Notes</notes><logolink>https://example.test/logo.png</logolink></vendor><categories><category><id>1</id><name>Props</name><categories><category><id>2</id><name>Stars</name></category></categories></category></categories><models><model><id>abc</id><categoryid>1</categoryid><categoryid>2</categoryid><name>Star</name><type>Spinner</type><weblink>example.test/star</weblink><material>Coro</material><width>24</width><height>25</height><thickness>10mm</thickness><pixelcount>150</pixelcount><pixeldescription>Bullet</pixeldescription><pixelspacing>2</pixelspacing><notes>Assembly notes</notes><imagefile>https://example.test/first.png</imagefile><imagefile>invalid ignored second</imagefile><wiring><xmodellink>https://example.test/a.xmodel</xmodellink></wiring><wiring><name>Named</name><description>Native</description><xmodellink>https://example.test/b.prp</xmodellink></wiring><wiring><xmodellink>https://example.test/c.PRP</xmodellink></wiring><wiring><name>Invalid empty</name></wiring></model></models></modelinventory>"#;
#[test]
fn inventory_maps_children_metadata_categories_and_model_links() {
    let i = parse_inventory(INVENTORY).unwrap();
    assert_eq!(i.vendor.name, "Vendor A");
    assert_eq!(i.vendor.links.len(), 2);
    assert_eq!(i.vendor.description, "Vendor Notes");
    let p = &i.products[0];
    assert_eq!(p.id, 0);
    assert_eq!(p.pixel_count, 150);
    assert_eq!(p.category_ids, vec![1, 2]);
    assert_eq!(p.image.as_deref(), Some("https://example.test/first.png"));
    assert_eq!(p.url.as_deref(), Some("http://example.test/star"));
    assert_eq!(
        p.models
            .iter()
            .map(|m| (&*m.name, m.kind, &*m.description))
            .collect::<Vec<_>>(),
        vec![
            ("Model 1", ModelType::XModel, "No description provided."),
            ("Named", ModelType::Prop, "Native"),
            ("Model 2", ModelType::XModel, "No description provided.")
        ]
    );
    assert_eq!(i.categories[0].products, vec![0]);
    assert_eq!(i.categories[0].categories[0].products, vec![0]);
    assert_eq!(i.warnings.len(), 1);
}
#[test]
fn browser_tree_keeps_category_then_product_order_and_clears_cross_vendor_selection() {
    let a = parse_inventory(INVENTORY).unwrap();
    let mut b = a.clone();
    b.vendor.name = "Vendor B".into();
    let mut browser = Browser::default();
    browser.set_inventories(vec![a, b]);
    assert_eq!(browser.vendor, Some(0));
    assert_eq!(
        browser.rows(None)[0],
        TreeRow {
            path: "c0".into(),
            name: "Props".into(),
            product: false,
            child_count: 2
        }
    );
    let rows = browser.rows(Some("c0"));
    assert_eq!(
        rows.iter()
            .map(|r| (&*r.path, r.product))
            .collect::<Vec<_>>(),
        vec![("c0/c0", false), ("c0/p0", true)]
    );
    assert!(browser.select_product("c0/c0/p0"));
    assert_eq!(browser.selection(1).unwrap().vendor.name, "Vendor A");
    assert!(!browser.select_product("c0/p999"));
    browser.select_vendor(1);
    assert!(browser.selected_product().is_none());
    assert!(browser.selection(0).is_err());
    assert!(browser.select_product("c0/p0"));
    assert_eq!(browser.selection(0).unwrap().vendor.name, "Vendor B");
}
#[test]
fn xmodel_metadata_overrides_product_fields_but_only_fills_empty_vendor_fields() {
    let mut browser = Browser::default();
    browser.set_inventories(vec![parse_inventory(INVENTORY).unwrap()]);
    browser.select_product("c0/p0");
    let mut prop = Prop::new("P", "test", "");
    prop.vendor.name = "Existing".into();
    prop.vendor.email = "existing@example.test".into();
    prop.physical.node_count = "3".into();
    browser.selection(0).unwrap().apply_metadata(&mut prop);
    assert_eq!(prop.vendor.name, "Existing");
    assert_eq!(prop.vendor.email, "existing@example.test");
    assert_eq!(prop.vendor.contact, "Team");
    assert_eq!(prop.vendor.website, "https://example.test/");
    assert_eq!(prop.physical.node_count, "150");
    assert_eq!(prop.physical.depth, "10mm");
    assert_eq!(prop.information.notes, "Assembly notes");
    assert_eq!(prop.prop_type, "Spinner");
    let mut prop = Prop::new("P", "test", "");
    prop.physical.node_count = "3".into();
    prop.prop_type = "Native".into();
    browser.selection(1).unwrap().apply_metadata(&mut prop);
    assert_eq!(prop.physical.node_count, "3");
    assert_eq!(prop.prop_type, "Native");
    assert_eq!(prop.vendor.name, "Vendor A");
}
#[test]
fn invalid_required_urls_or_numeric_categories_fail_whole_vendor() {
    for replacement in [
        INVENTORY.replace("<id>1</id>", "<id>bad</id>"),
        INVENTORY.replace("<categoryid>2</categoryid>", "<categoryid>-1</categoryid>"),
        INVENTORY.replace("https://example.test/first.png", "relative.png"),
        INVENTORY.replace("https://example.test/logo.png", "relative.png"),
        INVENTORY.replace("https://example.test/a.xmodel", "relative.xmodel"),
    ] {
        assert!(parse_inventory(&replacement).is_err());
    }
    let p = parse_inventory(
        &INVENTORY
            .replace(
                "<pixelcount>150</pixelcount>",
                "<pixelcount>bad</pixelcount>",
            )
            .replace("example.test/star", "http://"),
    )
    .unwrap();
    assert_eq!(p.products[0].pixel_count, 0);
    assert!(p.products[0].url.is_none());
}
#[test]
fn lists_and_xml_are_case_sensitive_and_fields_are_not_attributes() {
    assert_eq!(
        parse_vendor_list(r#"[{"Name":"A","Url":"https://example.test/a.xml"}]"#).unwrap()[0].name,
        "A"
    );
    assert!(parse_vendor_list("{}").is_err());
    let i=parse_inventory(r#"<modelinventory><vendor name="ignored"/><categories><category id="55" name="ignored"/></categories><models/></modelinventory>"#).unwrap();
    assert_eq!(i.vendor.name, "");
    assert_eq!(i.categories[0].id, 0);
    assert_eq!(i.categories[0].name, "");
    assert!(parse_inventory("<ModelInventory/>").is_err());
}
struct Mock(HashMap<String, Result<Vec<u8>, String>>);
impl Transport for Mock {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        self.0
            .get(url)
            .cloned()
            .unwrap_or_else(|| Err(format!("Unexpected request {url}")))
    }
}
#[test]
fn loading_reports_progress_partial_failures_and_empty_list() {
    let m = Mock(HashMap::from([
        (
            "https://list".into(),
            Ok(
                br#"[{"Name":"Bad","Url":"https://bad"},{"Name":"Good","Url":"https://good"}]"#
                    .to_vec(),
            ),
        ),
        ("https://bad".into(), Err("offline".into())),
        ("https://good".into(), Ok(INVENTORY.as_bytes().to_vec())),
    ]));
    let (mut progress, mut errors) = (Vec::new(), Vec::new());
    let all = load(&m, "https://list", |s| progress.push(s), |e| errors.push(e));
    assert_eq!(all.len(), 1);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Bad"));
    assert_eq!(
        progress,
        vec![
            "Loading vendor list...",
            "Retrieving inventory from Bad...",
            "Retrieving inventory from Good...",
            ""
        ]
    );
    let mut errors = Vec::new();
    assert!(load(&m, "https://missing", |_| {}, |e| errors.push(e)).is_empty());
    assert_eq!(errors[0].title, "Error Retrieving Vendors");
    let m = Mock(HashMap::from([(
        "https://empty".into(),
        Ok(b"[]".to_vec()),
    )]));
    assert!(load(
        &m,
        "https://empty",
        |_| {},
        |_| panic!("empty list must be silent")
    )
    .is_empty());
}
#[test]
fn utf16_inventory_text_decodes_and_missing_email_stays_empty() {
    let xml = INVENTORY.replace("<email>support@example.test</email>", "");
    let mut bytes = vec![0xff, 0xfe];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let decoded = decode_text(&bytes).unwrap();
    assert_eq!(parse_inventory(&decoded).unwrap().vendor.email, "");
    assert!(decode_text(&[0xff, 0xfe, 0]).is_err());
}
