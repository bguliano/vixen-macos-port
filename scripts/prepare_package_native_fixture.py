#!/usr/bin/env python3
"""Prepare only private files for normal package panels; never launch an app."""
import argparse,json,shutil,uuid
from pathlib import Path
import xml.etree.ElementTree as ET
import package_native_contract as c
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'target/native-panels-quit-menu-fixture/profile'
def prepare(out):
    assert out.is_relative_to(ROOT/'target') and not out.exists();out.mkdir(parents=True)
    profile=out/'profile'
    for folder in ['SystemData','Sequence','Sequence/Imported sequences é','Media','Module Data Files','Export']:(profile/folder).mkdir(parents=True,exist_ok=True)
    original_sequence=SOURCE/'Sequence/Panel Source Café.tim'
    copied={}
    for relative in ['SystemData/SystemConfig.xml','SystemData/ModuleStore.xml','Sequence/Panel Source Café.tim']:
        source=SOURCE/relative;target=profile/relative;shutil.copy2(source,target);copied[str(source)]=c.sha(source);assert c.sha(target)==c.sha(source)
    system=c.parsed(profile/'SystemData/SystemConfig.xml');assert all(len(system.find(t))==0 for t in ['Controllers','Patches','Previews'])
    nodes=system.find('Nodes');assert len(nodes)==1 and not len(nodes[0]);node=nodes[0];assert set(node.attrib)=={'name','id','channelId'}
    modules=c.parsed(profile/'SystemData/ModuleStore.xml');web=[e for e in modules.iter() if e.tag.endswith('}IsEnabled') and 'WebServer' in e.tag];assert len(web)==1 and web[0].text=='false'
    assert all(x.get('assemblyName')=='Module.App.WebServer' for x in modules.iter('Module'))
    source_sequence=profile/'Sequence/Panel Source Café.tim';sequence=c.parsed(source_sequence)
    assert not any((x.text or '').strip() for x in sequence.iter() if x.tag.rsplit('}',1)[-1] in ['FileName','Glediator_FileName'])
    effects=[x for x in sequence.iter() if x.tag.rsplit('}',1)[-1]=='EffectNodeSurrogate'];assert len(effects)==1
    targets=[x.text for x in effects[0].iter() if x.tag.rsplit('}',1)[-1]=='NodeId'];assert targets==[node.get('id')]
    child=dict(id=node.get('id'),name=node.get('name'),element=dict(id=node.get('channelId'),name=node.get('name')))
    input_folder=out/'Inputs';input_folder.mkdir();map_file=input_folder/'Private Identity Map é.vmap'
    mapping=dict(id=str(uuid.uuid5(uuid.NAMESPACE_URL,'private-package-map')),name='Private Package Identity',isDirty=False,elementMappings=[dict(sourceId=node.get('id'),sourceName=node.get('name'),targetId=node.get('id'),targetName=node.get('name'),isDirty=False)],sourceTree=dict(id=str(uuid.uuid5(uuid.NAMESPACE_URL,'private-package-tree')),name='Root',children=[child]))
    map_file.write_text(json.dumps(mapping,indent=2)+'\n')
    output_folder=out/'Exported packages é';output_folder.mkdir();package=output_folder/'Private Package Café.vpkg';imported=profile/'Sequence/Imported sequences é/Imported Package Café.tim'
    (out/'settings').mkdir();(out/'runtime').mkdir()
    m=dict(preparedOnly=True,launched=False,profile=str(profile),sourceSequence=str(source_sequence),sourceSequenceSha256=c.sha(source_sequence),mapFile=str(map_file),mapSha256=c.sha(map_file),packageOutput=str(package),importedSequence=str(imported),archiveSequenceEntry='Sequence\\Panel Source Café.tim',archiveEntries=['Sequence\\Panel Source Café.tim','ProfileInfo\\ElementTree.v3e'],expectedTreeChildren=[child],expectedImportedCanonical=c.canonical(sequence),initialOtherModuleXML=c.other_modules(profile/'SystemData/ModuleStore.xml'),systemConfigCanonical=c.canonical(system),sourceInputs=copied,profileFiles={str(p.relative_to(profile)):c.sha(p) for p in sorted(profile.rglob('*')) if p.is_file()},requiredEnvironment=dict(VIXEN_DATA=str(profile),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1',TMPDIR=str(out/'runtime')+'/'),requiredArguments=['no_controllers','no_execution'],forbiddenEnvironment=['VIXEN_UI_TEST','VIXEN_TEST_FILE','VIXEN_TEST_PACKAGE_OVERWRITE'],media=0,controllers=0,webEnabled=False,scope='Prepared files only. Future normal app must use approved private Cocoa bundle. Real package export/source/save chooser Cancel and acceptance, package/map Open Cancel and acceptance, import collision Choose Another Name, destination Save Cancel then a new private destination. No model hooks or controller outputs.',pins={str(p):c.sha(p) for p in [Path(__file__).resolve(),ROOT/'scripts/package_native_contract.py',ROOT/'scripts/test_package_native_contract.py']})
    m['pins'][str(ROOT/'scripts/vixen2_physical_contract.py')]=c.sha(ROOT/'scripts/vixen2_physical_contract.py')
    (out/'prepared.json').write_text(json.dumps(m,indent=2)+'\n');assert json.loads((out/'prepared.json').read_text())==m
    return m
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);a=p.parse_args();m=prepare(a.out.resolve());print(json.dumps(dict(preparedOnly=True,launched=False,sourceSequence=m['sourceSequence'],profile=m['profile'])))
