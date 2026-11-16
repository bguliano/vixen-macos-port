//! Sequences built in the test: save, load, and compare with what the file said.

mod common;

use common::*;
use uuid::Uuid;
use vixen_io::xml::same_tree;
use vixen_io::{load_sequence, new_sequence, save_sequence, EffectNode};
use vixen_xml::XSI_NS;

#[test]
fn a_new_sequence_saves_and_loads() {
    let dir = TempDir::new("new");
    let mut seq = new_sequence();
    seq.length = vixen_core::TimeSpan::from_secs(60.0);
    let d = vixen_effect_registry::by_data_type("PulseData").unwrap();
    seq.effects.push(EffectNode {
        start: vixen_core::TimeSpan::from_ms_i(500),
        time_span: vixen_core::TimeSpan::from_ms_i(1500),
        type_id: d.type_id,
        instance_id: Uuid::new_v4(),
        targets: vec![(Uuid::new_v4(), "Star".to_string())],
        params: d.new_params(),
    });
    let out = dir.path("New.tim");
    save_sequence(&seq, &out).unwrap();
    let back = load_sequence(&out).unwrap();
    assert_eq!(back.effects.len(), 1);
    assert_eq!(back.effects[0].targets, seq.effects[0].targets);
    assert_eq!(back.effects[0].params.get("LevelCurve"), seq.effects[0].params.get("LevelCurve"));
    assert_eq!(back.length, seq.length);
    assert_eq!(back.time_per_pixel, vixen_core::TimeSpan::MIN);
    assert_eq!(back.default_playback_start_time, Some(vixen_core::TimeSpan::ZERO));
    assert_eq!(back.layers.layers.len(), 1);
    assert_eq!(back.layers.layers[0].name, "Default");
    let xml = read(&out);
    assert!(same_tree(&xml, &read(&out)));
    // Every effect member was written, as a new object would be.
    let model = xml.child("_dataModels").unwrap().elements().next().unwrap();
    for p in d.params.iter().filter(|p| !p.xml.is_empty()) {
        assert!(model.child(p.xml).is_some(), "{}", p.xml);
    }
}

#[test]
fn an_unknown_effect_type_comes_back_unchanged() {
    let dir = TempDir::new("unknown");
    let text = format!(
        r#"<TimedSequenceData version="9" xmlns:a="http://www.w3.org/2001/XMLSchema" xmlns:i="{XSI_NS}" xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed">
  <ModuleInstanceId xmlns="">6879a66e-7199-45ae-ab30-874d6be50246</ModuleInstanceId>
  <ModuleTypeId xmlns="">296bdba2-9bf3-4bff-a9f2-13efac5c8ecb</ModuleTypeId>
  <Length xmlns="">PT1M</Length>
  <SequenceLayers xmlns:d1p1="http://schemas.datacontract.org/2004/07/Vixen.Sys.LayerMixing" xmlns="">
    <d1p1:Layers xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays">
      <d2p1:anyType i:type="d1p1:DefaultLayer">
        <d1p1:Id>57edb46f-84ab-400b-b3b7-5cbdf7773aa1</d1p1:Id>
        <d1p1:LayerLevel>0</d1p1:LayerLevel>
        <d1p1:LayerName>Default</d1p1:LayerName>
        <d1p1:Type>Default</d1p1:Type>
      </d2p1:anyType>
    </d1p1:Layers>
    <d1p1:_effectLayerMap xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" />
  </SequenceLayers>
  <Version xmlns="">0</Version>
  <_dataModels xmlns:d1p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" xmlns="">
    <d1p1:anyType xmlns:d2p1="http://schemas.datacontract.org/2004/07/Some.Future.Effect" i:type="d2p1:FutureData">
      <ModuleInstanceId>11111111-1111-1111-1111-111111111111</ModuleInstanceId>
      <ModuleTypeId>22222222-2222-2222-2222-222222222222</ModuleTypeId>
      <d2p1:Zed>
        <d2p1:Inner i:nil="true" />
      </d2p1:Zed>
      <d2p1:Alpha>1</d2p1:Alpha>
    </d1p1:anyType>
    <d1p1:anyType xmlns:d2p1="http://schemas.datacontract.org/2004/07/Some.Orphan" i:type="d2p1:OrphanData">
      <ModuleInstanceId>33333333-3333-3333-3333-333333333333</ModuleInstanceId>
      <ModuleTypeId>44444444-4444-4444-4444-444444444444</ModuleTypeId>
    </d1p1:anyType>
  </_dataModels>
  <_effectNodeSurrogates xmlns="">
    <EffectNodeSurrogate>
      <InstanceId>11111111-1111-1111-1111-111111111111</InstanceId>
      <StartTime>PT1S</StartTime>
      <TargetNodes>
        <ChannelNodeReferenceSurrogate>
          <Name i:nil="true" />
          <NodeId>55555555-5555-5555-5555-555555555555</NodeId>
        </ChannelNodeReferenceSurrogate>
      </TargetNodes>
      <TimeSpan>PT2S</TimeSpan>
      <TypeId>22222222-2222-2222-2222-222222222222</TypeId>
      <FutureMember>x</FutureMember>
    </EffectNodeSurrogate>
  </_effectNodeSurrogates>
  <_filterNodeSurrogates xmlns="" />
  <_layerMixingFilterDataModels xmlns="" />
  <_layerMixingFilterSurrogates xmlns="" />
  <_mediaSurrogates xmlns="" />
  <_selectedTimingProviderSurrogate xmlns="">
    <ProviderType i:nil="true" />
    <SourceName i:nil="true" />
  </_selectedTimingProviderSurrogate>
  <Future>kept</Future>
  <DefaultPlaybackEndTime i:nil="true" />
  <DefaultPlaybackStartTime>PT0S</DefaultPlaybackStartTime>
  <DefaultRowHeight>40</DefaultRowHeight>
  <DefaultSplitterDistance>0</DefaultSplitterDistance>
  <LabeledMarkCollections />
  <RowSettings />
  <TimePerPixel>PT0.01S</TimePerPixel>
  <VisibleTimeStart>PT0S</VisibleTimeStart>
</TimedSequenceData>"#
    );
    let path = dir.path("Unknown.tim");
    std::fs::write(&path, &text).unwrap();
    let original = read(&path);
    let seq = load_sequence(&path).unwrap();
    assert_eq!(seq.effects.len(), 1);
    assert!(seq.effects[0].params.desc.is_empty());
    assert!(seq.effects[0].params.unknown_members.iter().any(|(n, _)| n == "Zed"));
    assert_eq!(seq.effects[0].targets[0].1, "");
    assert_eq!(seq.fidelity_report().orphan_models, 1);
    assert_eq!(seq.fidelity_report().effect_surrogates, 1);

    let out = dir.path("Out.tim");
    save_sequence(&seq, &out).unwrap();
    assert_eq!(original.first_difference(&read(&out), String::new()), None);

    // Moving the effect keeps its data and the surrogate's unknown member.
    let mut moved = seq.clone();
    moved.effects[0].start = vixen_core::TimeSpan::from_secs(5.0);
    let out = dir.path("Moved.tim");
    save_sequence(&moved, &out).unwrap();
    let saved = read(&out);
    let s = saved.child("_effectNodeSurrogates").unwrap().elements().next().unwrap();
    assert_eq!(s.child_text("StartTime").as_deref(), Some("PT5S"));
    assert_eq!(s.child_text("FutureMember").as_deref(), Some("x"));
    assert!(same_tree(saved.child("_dataModels").unwrap(), original.child("_dataModels").unwrap()));
}
