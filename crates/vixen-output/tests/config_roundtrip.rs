//! Every controller and filter configuration reads and writes its module XML exactly: member order, namespaces, nil and types,
//! checked against samples laid out by the DataContractSerializer's rules.

use uuid::{uuid, Uuid};
use vixen_output::filters::tagged::{ColorWheelFilterConfig, FixtureColorWheelEntry};
use vixen_output::filters::{
    CoarseFineConfig, ColorBreakdownConfig, ColorBreakdownItem, CurveData, DimmingCurveConfig, DimmingFilterConfig, FilterConfig,
    PointPairData, PrismFilterConfig, ShutterFilterConfig, TaggedFilterConfig,
};
use vixen_output::ids::{self, CONTROLLER_DESCRIPTORS, FILTER_DESCRIPTORS};
use vixen_output::modules::{
    ControllerConfig, DdpConfig, DmxUsbProConfig, DummyLightingConfig, E131Config, ElexolConfig, GenericSerialConfig,
    LauncherConfig, OpenDmxConfig, OpenDmxDevice, RdsConfig, RdsHardware, RenardConfig, RenderStyle, UniverseEntry,
};
use vixen_output::serial::{Parity, SerialSettings, StopBits};
use vixen_output::{DotNetColor, ModuleIds};
use vixen_xml::XElement;

const I: &str = r#"xmlns:i="http://www.w3.org/2001/XMLSchema-instance""#;

const INSTANCE: Uuid = uuid!("ee79fee2-2d7d-4984-a178-288cee39a361");

fn head(type_id: &Uuid) -> String {
    format!(r#"<ModuleInstanceId xmlns="">{INSTANCE}</ModuleInstanceId><ModuleTypeId xmlns="">{type_id}</ModuleTypeId>"#)
}

fn parse(xml: &str) -> XElement {
    XElement::parse(xml).unwrap_or_else(|e| panic!("{e}: {xml}"))
}

fn assert_same(ours: &XElement, theirs: &XElement) {
    assert!(
        ours.semantic_eq(theirs),
        "{:?}\nours:\n{}\ntheirs:\n{}",
        ours.first_difference(theirs, String::new()),
        ours.to_fragment(),
        theirs.to_fragment()
    );
    // It also survives being written out as text.
    let back = parse(&ours.to_document());
    assert!(back.semantic_eq(theirs), "{:?}\n{}", back.first_difference(theirs, String::new()), ours.to_document());
}

fn controller_round_trip(type_id: Uuid, xml: &str) -> ControllerConfig {
    let e = parse(xml);
    let config = ControllerConfig::from_xml(&type_id, &e).expect("known module type");
    assert_eq!(config.ids(), &ModuleIds::new(type_id, INSTANCE));
    assert_same(&config.to_xml().expect("has data"), &e);
    config
}

fn filter_round_trip(type_id: Uuid, xml: &str) -> FilterConfig {
    let e = parse(xml);
    let config = FilterConfig::from_xml(&type_id, &e).expect("known module type");
    assert_eq!(config.ids(), &ModuleIds::new(type_id, INSTANCE));
    assert_same(&config.to_xml(), &e);
    config
}

// ---------------------------------------------------------------- Controllers

#[test]
fn generic_serial_with_text_and_with_nil() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.GenericSerial";
    let c = controller_round_trip(
        ids::GENERIC_SERIAL,
        &format!(
            r#"<Data xmlns="{ns}" {I}>{}<BaudRate>57600</BaudRate><DataBits>7</DataBits><Footer>~ end</Footer><Header> start&amp;</Header><Parity>Even</Parity><PortName>/dev/cu.usbserial-1</PortName><StopBits>Two</StopBits></Data>"#,
            head(&ids::GENERIC_SERIAL)
        ),
    );
    let ControllerConfig::GenericSerial(c) = c else { panic!() };
    assert_eq!(c.header.as_deref(), Some(" start&"));
    assert_eq!(c.footer.as_deref(), Some("~ end"));
    assert_eq!(c.serial.parity, Parity::Even);
    assert_eq!(c.serial.stop_bits, StopBits::Two);
    controller_round_trip(
        ids::GENERIC_SERIAL,
        &format!(
            r#"<Data xmlns="{ns}" {I}>{}<BaudRate>0</BaudRate><DataBits>0</DataBits><Footer i:nil="true"/><Header i:nil="true"/><Parity>None</Parity><PortName i:nil="true"/><StopBits>None</StopBits></Data>"#,
            head(&ids::GENERIC_SERIAL)
        ),
    );
}

#[test]
fn renard() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.Renard";
    let c = controller_round_trip(
        ids::RENARD,
        &format!(
            r#"<Data xmlns="{ns}" {I}>{}<BaudRate>57600</BaudRate><DataBits>8</DataBits><Parity>None</Parity><PortName>COM3</PortName><ProtocolVersion>1</ProtocolVersion><StopBits>One</StopBits><WriteTimeout>500</WriteTimeout></Data>"#,
            head(&ids::RENARD)
        ),
    );
    let ControllerConfig::Renard(c) = c else { panic!() };
    assert_eq!(c.write_timeout, 500);
    assert_eq!(c.protocol_version, 1);
}

#[test]
fn dmx_usb_pro_keeps_the_misspelled_member() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.DmxUsbPro";
    let c = controller_round_trip(
        ids::DMX_USB_PRO,
        &format!(
            r#"<Data xmlns="{ns}" {I}>{}<BaudRate>115200</BaudRate><DataBits>8</DataBits><Partity>Odd</Partity><PortName>COM5</PortName><StopBits>One</StopBits></Data>"#,
            head(&ids::DMX_USB_PRO)
        ),
    );
    let ControllerConfig::DmxUsbPro(c) = c else { panic!() };
    assert_eq!(c.serial.parity, Parity::Odd);
}

#[test]
fn open_dmx_with_and_without_a_device() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Controller.OpenDMX";
    controller_round_trip(
        ids::OPEN_DMX,
        &format!(r#"<OpenDMXData xmlns="{ns}" {I}>{}<Device i:nil="true"/></OpenDMXData>"#, head(&ids::OPEN_DMX)),
    );
    let c = controller_round_trip(
        ids::OPEN_DMX,
        &format!(
            r#"<OpenDMXData xmlns="{ns}" {I}>{}<Device><Description>USB</Description><Id>67330068</Id><Index>0</Index><Name i:nil="true"/><SerialNumber>A1</SerialNumber><Type>FT_DEVICE_232R</Type></Device></OpenDMXData>"#,
            head(&ids::OPEN_DMX)
        ),
    );
    let ControllerConfig::OpenDmx(c) = c else { panic!() };
    assert_eq!(c.device.unwrap().serial_number.as_deref(), Some("A1"));
}

#[test]
fn e131_with_universes_and_the_cid_in_the_instance_id() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.E131";
    let d = "http://schemas.datacontract.org/2004/07/VixenModules.Controller.E131";
    let c = controller_round_trip(
        ids::E131,
        &format!(
            r#"<E131ModuleDataModel xmlns="{ns}" {I}>{}<AutoPopulate>true</AutoPopulate><Blind>false</Blind><EventRepeatCount>2</EventRepeatCount><EventSuppressCount>3</EventSuppressCount><Multicast i:nil="true"/><OutputCount>0</OutputCount><Priority>100</Priority><Statistics>false</Statistics><Unicast>192.168.1.20</Unicast><Universes xmlns:a="{d}"><a:UniverseEntry><a:Active>true</a:Active><a:EventRepeatCount>0</a:EventRepeatCount><a:EventSuppressCount>0</a:EventSuppressCount><a:Multicast i:nil="true"/><a:Size>510</a:Size><a:Start>0</a:Start><a:Ttl>64</a:Ttl><a:Unicast i:nil="true"/><a:Universe>1</a:Universe><a:seqNum>42</a:seqNum></a:UniverseEntry><a:UniverseEntry><a:Active>false</a:Active><a:EventRepeatCount>1</a:EventRepeatCount><a:EventSuppressCount>1</a:EventSuppressCount><a:Multicast>{{GUID}}</a:Multicast><a:Size>2</a:Size><a:Start>510</a:Start><a:Ttl>1</a:Ttl><a:Unicast i:nil="true"/><a:Universe>2</a:Universe><a:seqNum>0</a:seqNum></a:UniverseEntry></Universes><Warnings>true</Warnings></E131ModuleDataModel>"#,
            head(&ids::E131)
        ),
    );
    let ControllerConfig::E131(c) = c else { panic!() };
    assert_eq!(c.event_repeat_count, 2);
    assert_eq!(c.unicast.as_deref(), Some("192.168.1.20"));
    let universes = c.universes.as_ref().unwrap();
    assert_eq!(universes[0].seq_num, 42);
    assert_eq!(universes[1].multicast.as_deref(), Some("{GUID}"));
    controller_round_trip(
        ids::E131,
        &format!(
            r#"<E131ModuleDataModel xmlns="{ns}" {I}>{}<AutoPopulate>false</AutoPopulate><Blind>true</Blind><EventRepeatCount>0</EventRepeatCount><EventSuppressCount>0</EventSuppressCount><Multicast>en0</Multicast><OutputCount>512</OutputCount><Priority>150</Priority><Statistics>true</Statistics><Unicast i:nil="true"/><Universes i:nil="true"/><Warnings>false</Warnings></E131ModuleDataModel>"#,
            head(&ids::E131)
        ),
    );
}

#[test]
fn ddp() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.DDP";
    let c = controller_round_trip(
        ids::DDP,
        &format!(
            r#"<DDPData xmlns="{ns}" {I}>{}<Address>192.168.1.50</Address><HostName>wled-porch</HostName></DDPData>"#,
            head(&ids::DDP)
        ),
    );
    let ControllerConfig::Ddp(c) = c else { panic!() };
    assert_eq!(c.host_name.as_deref(), Some("wled-porch"));
    controller_round_trip(
        ids::DDP,
        &format!(
            r#"<DDPData xmlns="{ns}" {I}>{}<Address>127.0.0.1</Address><HostName i:nil="true"/></DDPData>"#,
            head(&ids::DDP)
        ),
    );
}

#[test]
fn elexol() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.ElexolEtherIO";
    let c = controller_round_trip(
        ids::ELEXOL_ETHER_IO,
        &format!(
            r#"<ElexolEtherIOData xmlns="{ns}" {I}>{}<Address>10.0.0.3</Address><MinimumIntensity>20</MinimumIntensity><Port>2424</Port></ElexolEtherIOData>"#,
            head(&ids::ELEXOL_ETHER_IO)
        ),
    );
    let ControllerConfig::Elexol(c) = c else { panic!() };
    assert_eq!(c.minimum_intensity, 20);
}

#[test]
fn rds_in_every_hardware_mode() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.CommandController";
    for (name, hardware) in [
        ("HTTP", RdsHardware::Http),
        ("MRDS192", RdsHardware::Mrds192),
        ("MRDS1322", RdsHardware::Mrds1322),
        ("VFMT212R", RdsHardware::Vfmt212r),
    ] {
        let c = controller_round_trip(
            ids::RDS,
            &format!(
                r#"<Data xmlns="{ns}" {I}>{}<BiDirectional>true</BiDirectional><ConnectionMode>1</ConnectionMode><HardwareID>{name}</HardwareID><HideLaunchedWindows>true</HideLaunchedWindows><HttpPassword>p</HttpPassword><HttpUrl>http://127.0.0.1:8080/?action=update_rt&amp;update_rt={{text}}</HttpUrl><HttpUsername i:nil="true"/><PortName>COM2</PortName><PortNumber>2</PortNumber><RequireHTTPAuthentication>true</RequireHTTPAuthentication><Slow>true</Slow></Data>"#,
                head(&ids::RDS)
            ),
        );
        let ControllerConfig::Rds(c) = c else { panic!() };
        assert_eq!(c.hardware_id, hardware);
        assert_eq!(c.http_url.as_deref(), Some("http://127.0.0.1:8080/?action=update_rt&update_rt={text}"));
    }
}

#[test]
fn launcher() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.LauncherController";
    let c = controller_round_trip(
        ids::LAUNCHER,
        &format!(r#"<Data xmlns="{ns}" {I}>{}<HideLaunchedWindows>true</HideLaunchedWindows></Data>"#, head(&ids::LAUNCHER)),
    );
    let ControllerConfig::Launcher(c) = c else { panic!() };
    assert!(c.hide_launched_windows);
}

#[test]
fn dummy_lighting_in_every_style() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.Output.DummyLighting";
    for (name, style) in [
        ("Monochrome", RenderStyle::Monochrome),
        ("RGBMultiChannel", RenderStyle::RGBMultiChannel),
        ("RGBSingleChannel", RenderStyle::RGBSingleChannel),
    ] {
        let c = controller_round_trip(
            ids::DUMMY_LIGHTING,
            &format!(
                r#"<DummyLightingData xmlns="{ns}" {I}>{}<FormTitle>Window</FormTitle><RenderStyle>{name}</RenderStyle></DummyLightingData>"#,
                head(&ids::DUMMY_LIGHTING)
            ),
        );
        let ControllerConfig::DummyLighting(c) = c else { panic!() };
        assert_eq!(c.render_style, style);
    }
}

#[test]
fn the_debug_controller_has_no_data() {
    let c = ControllerConfig::new_default(&ids::DEBUG_CONTROLLER, INSTANCE).unwrap();
    assert!(c.to_xml().is_none());
}

#[test]
fn every_controller_default_round_trips() {
    for d in &CONTROLLER_DESCRIPTORS {
        let config = ControllerConfig::new_default(&d.type_id, INSTANCE).unwrap();
        if let Some(xml) = config.to_xml() {
            let back = ControllerConfig::from_xml(&d.type_id, &parse(&xml.to_document())).unwrap();
            assert_eq!(back, config, "{}", d.type_name);
        }
    }
}

#[test]
fn every_controller_with_all_fields_set_round_trips_through_its_struct() {
    let ids_for = |t| ModuleIds::new(t, INSTANCE);
    let serial = SerialSettings {
        port_name: Some("/dev/cu.x".to_string()),
        baud_rate: 115200,
        parity: Parity::Space,
        data_bits: 7,
        stop_bits: StopBits::OnePointFive,
    };
    let configs = vec![
        ControllerConfig::GenericSerial(GenericSerialConfig {
            ids: ids_for(ids::GENERIC_SERIAL),
            serial: serial.clone(),
            header: Some("<H>".to_string()),
            footer: None,
        }),
        ControllerConfig::Renard(RenardConfig {
            ids: ids_for(ids::RENARD),
            serial: serial.clone(),
            protocol_version: 1,
            write_timeout: 250,
        }),
        ControllerConfig::DmxUsbPro(DmxUsbProConfig { ids: ids_for(ids::DMX_USB_PRO), serial }),
        ControllerConfig::OpenDmx(OpenDmxConfig {
            ids: ids_for(ids::OPEN_DMX),
            device: Some(OpenDmxDevice {
                description: Some("d".to_string()),
                id: Some("i".to_string()),
                index: 3,
                name: Some("n".to_string()),
                serial_number: None,
                device_type: Some("t".to_string()),
            }),
        }),
        ControllerConfig::E131(E131Config {
            ids: ids_for(ids::E131),
            auto_populate: false,
            blind: true,
            event_repeat_count: 5,
            event_suppress_count: 6,
            multicast: Some("en0".to_string()),
            output_count: 7,
            priority: 8,
            statistics: true,
            unicast: Some("h".to_string()),
            universes: Some(vec![UniverseEntry {
                active: true,
                event_repeat_count: 1,
                event_suppress_count: 2,
                multicast: None,
                size: 3,
                start: 4,
                ttl: 5,
                unicast: None,
                universe: 6,
                seq_num: 7,
            }]),
            warnings: true,
        }),
        ControllerConfig::Ddp(DdpConfig { ids: ids_for(ids::DDP), address: None, host_name: Some("x".to_string()) }),
        ControllerConfig::Elexol(ElexolConfig {
            ids: ids_for(ids::ELEXOL_ETHER_IO),
            address: Some("a".to_string()),
            minimum_intensity: 9,
            port: 8,
        }),
        ControllerConfig::Rds(RdsConfig {
            ids: ids_for(ids::RDS),
            bi_directional: true,
            connection_mode: 2,
            hardware_id: RdsHardware::Vfmt212r,
            hide_launched_windows: true,
            http_password: Some("p".to_string()),
            http_url: Some("u".to_string()),
            http_username: Some("n".to_string()),
            port_name: Some("c".to_string()),
            port_number: 4,
            require_http_authentication: true,
            slow: true,
        }),
        ControllerConfig::Launcher(LauncherConfig { ids: ids_for(ids::LAUNCHER), hide_launched_windows: true }),
        ControllerConfig::DummyLighting(DummyLightingConfig {
            ids: ids_for(ids::DUMMY_LIGHTING),
            form_title: None,
            render_style: RenderStyle::RGBMultiChannel,
        }),
    ];
    for config in configs {
        let xml = config.to_xml().unwrap();
        let back = ControllerConfig::from_xml(&config.ids().type_id, &parse(&xml.to_document())).unwrap();
        assert_eq!(back, config);
    }
}

// ---------------------------------------------------------------- Filters

#[test]
fn color_breakdown_with_known_and_value_colours() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.ColorBreakdown";
    let d = "http://schemas.datacontract.org/2004/07/System.Drawing";
    let c = filter_round_trip(
        ids::COLOR_BREAKDOWN,
        &format!(
            r#"<ColorBreakdownData xmlns="{ns}" {I}>{}<BreakdownItems><ColorBreakdownItem><Color xmlns:a="{d}"><a:knownColor>141</a:knownColor><a:name i:nil="true"/><a:state>1</a:state><a:value>0</a:value></Color><Name>Red</Name></ColorBreakdownItem><ColorBreakdownItem><Color xmlns:a="{d}"><a:knownColor>0</a:knownColor><a:name i:nil="true"/><a:state>2</a:state><a:value>4294944000</a:value></Color><Name>Orange</Name></ColorBreakdownItem></BreakdownItems><MixColors>false</MixColors><_16Bit>true</_16Bit></ColorBreakdownData>"#,
            head(&ids::COLOR_BREAKDOWN)
        ),
    );
    let FilterConfig::ColorBreakdown(c) = c else { panic!() };
    assert_eq!(c.breakdown_items[0].color, DotNetColor::RED);
    assert_eq!(c.breakdown_items[1].color.to_argb_u32(), 4294944000);
    assert!(c.sixteen_bit);
    assert!(!c.mix_colors);
}

#[test]
fn dimming_curve_with_a_real_curve_layout() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.DimmingCurve";
    let curves = "http://schemas.datacontract.org/2004/07/VixenModules.App.Curves";
    let zed = "http://schemas.datacontract.org/2004/07/ZedGraph";
    let xsd = "http://www.w3.org/2001/XMLSchema";
    // The curve's layout is what real sequence files hold. Note the unqualified members of the ISerializable PointPair.
    let point = |x: &str, y: &str| {
        format!(
            r#"<z:PointPair><schema xmlns="" i:type="x:int">11</schema><X xmlns="" i:type="x:double">{x}</X><Y xmlns="" i:type="x:double">{y}</Y><schema2 xmlns="" i:type="x:int">11</schema2><Z xmlns="" i:type="x:double">0</Z><Tag xmlns="" i:nil="true"/></z:PointPair>"#
        )
    };
    let c = filter_round_trip(
        ids::DIMMING_CURVE,
        &format!(
            r#"<DimmingCurveData xmlns="{ns}" {I} xmlns:x="{xsd}">{}<Curve xmlns:c="{curves}"><c:IsCurrentLibraryCurve>false</c:IsCurrentLibraryCurve><c:Points xmlns:z="{zed}">{}{}{}</c:Points><c:_libraryReferenceName></c:_libraryReferenceName></Curve></DimmingCurveData>"#,
            head(&ids::DIMMING_CURVE),
            point("0", "0"),
            point("33.333333333333336", "12.5"),
            point("100", "100"),
        ),
    );
    let FilterConfig::DimmingCurve(c) = c else { panic!() };
    assert_eq!(c.curve.points.len(), 3);
    assert_eq!(c.curve.points[1].x, 33.333333333333336);
    assert_eq!(c.curve.to_curve().get_value(100.0), 100.0);
}

#[test]
fn a_new_dimming_curve_is_a_ramp() {
    let c = DimmingCurveConfig::default();
    assert_eq!(c.curve.points, vec![PointPairData::new(0.0, 0.0), PointPairData::new(100.0, 100.0)]);
    let back = FilterConfig::from_xml(&ids::DIMMING_CURVE, &parse(&FilterConfig::DimmingCurve(c.clone()).to_xml().to_document()))
        .unwrap();
    assert_eq!(back, FilterConfig::DimmingCurve(c));
}

#[test]
fn coarse_fine() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.CoarseFineBreakdown";
    let c = filter_round_trip(
        ids::COARSE_FINE_BREAKDOWN,
        &format!(
            r#"<CoarseFineBreakdownData xmlns="{ns}" {I}>{}<EnableDefaultValueMapping>true</EnableDefaultValueMapping><RestingCoarseValue>165</RestingCoarseValue><RestingFineValue>60</RestingFineValue></CoarseFineBreakdownData>"#,
            head(&ids::COARSE_FINE_BREAKDOWN)
        ),
    );
    let FilterConfig::CoarseFine(c) = c else { panic!() };
    assert_eq!((c.resting_coarse_value, c.resting_fine_value), (165, 60));
}

const TAG_NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.TaggedFilter";

#[test]
fn tagged_filter() {
    let c = filter_round_trip(
        ids::TAGGED_FILTER,
        &format!(r#"<TaggedFilterData xmlns="{TAG_NS}" {I}>{}<Tag>Pan</Tag></TaggedFilterData>"#, head(&ids::TAGGED_FILTER)),
    );
    let FilterConfig::Tagged(c) = c else { panic!() };
    assert_eq!(c.tag.as_deref(), Some("Pan"));
    filter_round_trip(
        ids::TAGGED_FILTER,
        &format!(r#"<TaggedFilterData xmlns="{TAG_NS}" {I}>{}<Tag i:nil="true"/></TaggedFilterData>"#, head(&ids::TAGGED_FILTER)),
    );
}

#[test]
fn the_tag_keeps_the_base_class_namespace_on_derived_filters() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.DimmingFilter";
    let c = filter_round_trip(
        ids::DIMMING_FILTER,
        &format!(
            r#"<DimmingFilterData xmlns="{ns}" {I}>{}<Tag xmlns="{TAG_NS}">Dimmer</Tag><ConvertRGBIntoDimmingIntents>true</ConvertRGBIntoDimmingIntents></DimmingFilterData>"#,
            head(&ids::DIMMING_FILTER)
        ),
    );
    let FilterConfig::Dimming(c) = c else { panic!() };
    assert!(c.convert_rgb_into_dimming_intents);
    assert_eq!(c.tag.as_deref(), Some("Dimmer"));
}

#[test]
fn prism_filter() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.PrismFilter";
    let c = filter_round_trip(
        ids::PRISM_FILTER,
        &format!(
            r#"<PrismFilterData xmlns="{ns}" {I}>{}<Tag xmlns="{TAG_NS}">Open</Tag><AssociatedFunctionName>Prism</AssociatedFunctionName><ClosePrismIndexValue>10</ClosePrismIndexValue><ConvertPrismIntentsIntoOpenPrismIntents>true</ConvertPrismIntentsIntoOpenPrismIntents><OpenPrismIndexValue>200</OpenPrismIndexValue></PrismFilterData>"#,
            head(&ids::PRISM_FILTER)
        ),
    );
    let FilterConfig::Prism(c) = c else { panic!() };
    assert_eq!((c.open_prism_index_value, c.close_prism_index_value), (200, 10));
}

#[test]
fn shutter_filter() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.ShutterFilter";
    let c = filter_round_trip(
        ids::SHUTTER_FILTER,
        &format!(
            r#"<ShutterFilterData xmlns="{ns}" {I}>{}<Tag xmlns="{TAG_NS}">Shutter</Tag><CloseShutterIndexValue>0</CloseShutterIndexValue><ConvertColorIntoShutterIntents>true</ConvertColorIntoShutterIntents><OpenShutterIndexValue>255</OpenShutterIndexValue></ShutterFilterData>"#,
            head(&ids::SHUTTER_FILTER)
        ),
    );
    let FilterConfig::Shutter(c) = c else { panic!() };
    assert_eq!(c.open_shutter_index_value, 255);
    assert!(c.convert_color_into_shutter_intents);
}

#[test]
fn color_wheel_filter() {
    let ns = "http://schemas.datacontract.org/2004/07/VixenModules.OutputFilter.ColorWheelFilter";
    let fx = "http://schemas.datacontract.org/2004/07/VixenModules.App.Fixture";
    let c = filter_round_trip(
        ids::COLOR_WHEEL_FILTER,
        &format!(
            r#"<ColorWheelFilterData xmlns="{ns}" {I}>{}<Tag xmlns="{TAG_NS}">Wheel</Tag><ColorWheelData xmlns:f="{fx}"><f:FixtureColorWheel><f:Name>Red</f:Name><f:EndValue>20</f:EndValue><f:IndexType>ColorWheel</f:IndexType><f:StartValue>10</f:StartValue><f:UseCurve>false</f:UseCurve><f:Color1Html>#FF0000</f:Color1Html><f:Color2Html>White</f:Color2Html><f:ExcludeColorProperty>false</f:ExcludeColorProperty><f:HalfStep>true</f:HalfStep></f:FixtureColorWheel></ColorWheelData><ConvertColorIntentsIntoIndexCommands>true</ConvertColorIntentsIntoIndexCommands></ColorWheelFilterData>"#,
            head(&ids::COLOR_WHEEL_FILTER)
        ),
    );
    let FilterConfig::ColorWheel(c) = c else { panic!() };
    let entry = &c.color_wheel_data.as_ref().unwrap()[0];
    assert_eq!(entry.color1(), Some(DotNetColor::from_rgb(255, 0, 0)));
    assert_eq!(entry.color2(), Some(DotNetColor::WHITE));
    assert!(entry.half_step);
    filter_round_trip(
        ids::COLOR_WHEEL_FILTER,
        &format!(
            r#"<ColorWheelFilterData xmlns="{ns}" {I}>{}<Tag xmlns="{TAG_NS}" i:nil="true"/><ColorWheelData i:nil="true"/><ConvertColorIntentsIntoIndexCommands>false</ConvertColorIntentsIntoIndexCommands></ColorWheelFilterData>"#,
            head(&ids::COLOR_WHEEL_FILTER)
        ),
    );
}

#[test]
fn every_filter_default_round_trips() {
    for d in &FILTER_DESCRIPTORS {
        let config = FilterConfig::new_default(&d.type_id, INSTANCE).unwrap();
        let back = FilterConfig::from_xml(&d.type_id, &parse(&config.to_xml().to_document())).unwrap();
        assert_eq!(back, config, "{}", d.type_name);
    }
}

#[test]
fn every_filter_with_all_fields_set_round_trips_through_its_struct() {
    let ids_for = |t| ModuleIds::new(t, INSTANCE);
    let mut wheel = FixtureColorWheelEntry {
        name: Some("Blue".to_string()),
        end_value: 30,
        start_value: 20,
        use_curve: true,
        exclude_color_property: true,
        ..Default::default()
    };
    wheel.set_color1(&DotNetColor::from_rgb(0, 0, 255));
    wheel.set_color2(&DotNetColor::RED);
    let configs = vec![
        FilterConfig::ColorBreakdown(ColorBreakdownConfig {
            ids: ids_for(ids::COLOR_BREAKDOWN),
            breakdown_items: vec![
                ColorBreakdownItem::new(DotNetColor::from_argb(1, 2, 3, 4), "x"),
                ColorBreakdownItem::default(),
            ],
            mix_colors: true,
            sixteen_bit: true,
        }),
        FilterConfig::DimmingCurve(DimmingCurveConfig {
            ids: ids_for(ids::DIMMING_CURVE),
            curve: CurveData {
                is_current_library_curve: true,
                points: vec![PointPairData { schema: 11, x: 1.5, y: 2.25, schema2: 11, z: 3.0, tag: None }],
                library_reference_name: Some("Lib".to_string()),
            },
        }),
        FilterConfig::Dimming(DimmingFilterConfig {
            ids: ids_for(ids::DIMMING_FILTER),
            tag: Some("t".to_string()),
            convert_rgb_into_dimming_intents: true,
        }),
        FilterConfig::CoarseFine(CoarseFineConfig {
            ids: ids_for(ids::COARSE_FINE_BREAKDOWN),
            enable_default_value_mapping: true,
            resting_coarse_value: 1,
            resting_fine_value: 2,
        }),
        FilterConfig::Tagged(TaggedFilterConfig { ids: ids_for(ids::TAGGED_FILTER), tag: Some("t".to_string()) }),
        FilterConfig::Prism(PrismFilterConfig {
            ids: ids_for(ids::PRISM_FILTER),
            tag: None,
            associated_function_name: None,
            close_prism_index_value: 1,
            convert_prism_intents_into_open_prism_intents: true,
            open_prism_index_value: 2,
        }),
        FilterConfig::Shutter(ShutterFilterConfig {
            ids: ids_for(ids::SHUTTER_FILTER),
            tag: None,
            close_shutter_index_value: 3,
            convert_color_into_shutter_intents: true,
            open_shutter_index_value: 4,
        }),
        FilterConfig::ColorWheel(ColorWheelFilterConfig {
            ids: ids_for(ids::COLOR_WHEEL_FILTER),
            tag: Some("w".to_string()),
            color_wheel_data: Some(vec![wheel]),
            convert_color_intents_into_index_commands: true,
        }),
    ];
    for config in configs {
        let back = FilterConfig::from_xml(&config.ids().type_id, &parse(&config.to_xml().to_document())).unwrap();
        assert_eq!(back, config);
    }
}

#[test]
fn an_unknown_module_type_isnt_a_config() {
    let e = parse("<x/>");
    assert!(FilterConfig::from_xml(&Uuid::nil(), &e).is_none());
    assert!(ControllerConfig::from_xml(&Uuid::nil(), &e).is_none());
    assert!(FilterConfig::new_default(&Uuid::nil(), Uuid::nil()).is_none());
    assert!(ControllerConfig::new_default(&Uuid::nil(), Uuid::nil()).is_none());
}
