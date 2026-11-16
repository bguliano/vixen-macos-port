#!/usr/bin/env python3
"""Strict read-only package/TIM oracle for a private normal-app panel workflow."""
import argparse,hashlib,json,stat,unicodedata,uuid,zipfile
from pathlib import Path
import xml.etree.ElementTree as ET
from vixen2_physical_contract import typed_xml

PACKAGER='Module.App.TimedSequenceMapper'
NS='http://schemas.datacontract.org/2004/07/VixenModules.App.TimedSequenceMapper'
ARRAYS='http://schemas.microsoft.com/2003/10/Serialization/Arrays'
MODULE='b13997c2-d4b5-4b5a-9047-1dced97ad99b'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def canonical(node):return [node.tag,[[k,v] for k,v in sorted(node.attrib.items())],(node.text or '').strip(),[canonical(c) for c in node]]
def parsed(path):
    p=Path(path);assert p.is_file() and not p.is_symlink() and p.stat().st_size<4_000_000
    data=p.read_bytes();assert b'<!DOCTYPE' not in data.upper() and b'<!ENTITY' not in data.upper()
    return typed_xml(data)
def require_source(m):
    assert sha(m['sourceSequence'])==m['sourceSequenceSha256'],'Original private source changed'
    assert sha(m['mapFile'])==m['mapSha256'],'Private identity map changed'
def export_state(m):
    require_source(m);path=Path(m['packageOutput']);assert path.is_file() and not path.is_symlink() and path.stat().st_size<2_000_000
    with zipfile.ZipFile(path) as archive:
        items=archive.infolist();names=[x.filename for x in items]
        normalized={unicodedata.normalize('NFC',name):name for name in names}
        assert len(items)==2 and len(normalized)==2 and set(normalized)==set(m['archiveEntries']),'Unexpected package entry set or duplicate filename alias'
        assert archive.comment==b'' and all(not x.is_dir() and not x.flag_bits&1 and stat.S_IFMT(x.external_attr>>16)!=stat.S_IFLNK for x in items)
        assert sum(x.file_size for x in items)<len(Path(m['sourceSequence']).read_bytes())+16_384
        source=archive.read(normalized[m['archiveSequenceEntry']]);assert hashlib.sha256(source).hexdigest()==m['sourceSequenceSha256'],'Export rewrote source TIM bytes'
        tree=json.loads(archive.read('ProfileInfo\\ElementTree.v3e'))
        assert set(tree)=={'id','name','children'} and tree['name']=='Root'
        identifier=uuid.UUID(tree['id']);assert identifier.version==4 and str(identifier)==tree['id']
        assert tree['children']==m['expectedTreeChildren'],'Export tree differs from private profile'
    assert not list(path.parent.glob(path.name+'.part-*')),'Incomplete package left behind'
    return dict(passed=True,packageSha256=sha(path),entries=names,exportedSourceSha256=m['sourceSequenceSha256'],tree=tree)
def cancelled_import_state(m,exported):
    require_source(m);assert not Path(m['importedSequence']).exists(),'Cancelled native destination wrote a sequence'
    assert sha(m['packageOutput'])==exported['packageSha256'],'Import changed package bytes'
    assert not list(Path(m['profile']).rglob('.vixen-import-*')),'Temporary import output left behind'
    return dict(passed=True,sourceUnchanged=True,importedSequenceAbsent=True,packageSha256=exported['packageSha256'])
def imported_state(m,exported):
    require_source(m);assert sha(m['packageOutput'])==exported['packageSha256'],'Import changed package bytes'
    target=Path(m['importedSequence']);assert target.resolve().is_relative_to(Path(m['profile']).resolve()/'Sequence')
    root=parsed(target);assert canonical(root)==m['expectedImportedCanonical'],'Imported TIM differs from exact identity-mapped source'
    assert not list(Path(m['profile']).rglob('.vixen-import-*')),'Temporary import output left behind'
    return dict(passed=True,importedSequenceSha256=sha(target),sourceSequenceSha256=m['sourceSequenceSha256'],packageSha256=exported['packageSha256'],completeImportedXMLMatches=True)
def other_modules(path):
    root=parsed(path)
    for parent in root.iter():
        for module in list(parent):
            if module.tag=='Module' and module.attrib.get('assemblyName')==PACKAGER:parent.remove(module)
    return canonical(root)
def saved_settings(m):
    path=Path(m['profile'])/'SystemData/ModuleStore.xml';root=parsed(path)
    modules=[x for x in root.iter('Module') if x.get('assemblyName')==PACKAGER];assert len(modules)==1
    module=modules[0];assert module.attrib==dict(dataModelType='VixenModules.App.TimedSequenceMapper.TimedSequencePackagerData, TimedSequenceMapper',moduleType=MODULE,moduleInstance=MODULE,assemblyName=PACKAGER)
    assert len(module)==1;data=module[0];assert data.tag=='{'+NS+'}TimedSequencePackagerData'
    expected=ET.Element(data.tag)
    for tag,text in [('ModuleInstanceId',MODULE),('ModuleTypeId',MODULE),('{'+NS+'}ExportIncludeAudio','true'),('{'+NS+'}ExportOutputFile',m['packageOutput'])]:ET.SubElement(expected,tag).text=text
    entries=ET.SubElement(expected,'{'+NS+'}ExportSequenceFiles');ET.SubElement(entries,'{'+ARRAYS+'}string').text=m['sourceSequence']
    raw_output=data.find('{'+NS+'}ExportOutputFile').text
    raw_sources=[x.text for x in data.find('{'+NS+'}ExportSequenceFiles')]
    assert len(raw_sources)==1
    for actual,wanted in [(raw_output,m['packageOutput']),(raw_sources[0],m['sourceSequence'])]:
        assert isinstance(actual,str) and unicodedata.normalize('NFC',actual)==unicodedata.normalize('NFC',wanted),'Saved filesystem path differs'
        assert Path(actual).samefile(wanted),'Saved filesystem path resolves to different file'
    # Normalize only these two filesystem path fields, retaining raw values in
    # evidence. No XML names, typed values, colors, or arbitrary text are relaxed.
    data.find('{'+NS+'}ExportOutputFile').text=m['packageOutput'];data.find('{'+NS+'}ExportSequenceFiles')[0].text=m['sourceSequence']
    assert canonical(data)==canonical(expected),'Saved package settings differ from accepted workflow'
    assert other_modules(path)==m['initialOtherModuleXML'],'Unrelated module XML changed'
    return dict(passed=True,moduleStoreSha256=sha(path),output=m['packageOutput'],includeAudio=True,sequences=[m['sourceSequence']],rawOutput=raw_output,rawSequences=raw_sources)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--prepared',type=Path,required=True);p.add_argument('--phase',choices=['exported','cancelled-import','imported','settings'],required=True);p.add_argument('--export-proof',type=Path);p.add_argument('--report',type=Path,required=True);a=p.parse_args()
    assert not a.report.exists(),'Preserve prior acceptance evidence';m=json.loads(a.prepared.read_text())
    assert all(sha(path)==digest for path,digest in m['sourceInputs'].items()),'Source fixture input drift'
    if a.phase=='exported':result=export_state(m)
    elif a.phase=='settings':result=saved_settings(m)
    else:
        assert a.export_proof is not None;exported=json.loads(a.export_proof.read_text());assert exported['passed'] is True and exported['phase']=='exported'
        result=(cancelled_import_state if a.phase=='cancelled-import' else imported_state)(m,exported)
    result.update(phase=a.phase,scope='Read-only persisted-file acceptance only; native input/window evidence must be independently accepted.',preparedSha256=sha(a.prepared),nativeOperations=0)
    a.report.write_text(json.dumps(result,indent=2)+'\n')
