#!/usr/bin/env python3
"""Exact saved-state contract for actual Color/Gradient editor-to-library gestures."""
import copy,hashlib,re,time
from pathlib import Path
from vixen2_physical_contract import typed_xml,canonical,one
NS='http://schemas.datacontract.org/2004/07/VixenModules.App.ColorGradients'
DRAW='http://schemas.datacontract.org/2004/07/System.Drawing'
ARRAYS='http://schemas.microsoft.com/2003/10/Serialization/Arrays'
XSI='http://www.w3.org/2001/XMLSchema-instance'
SOURCE_COLOR=0xff1177cb
COPY_NAME='Editor Gradient Copy'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
local=lambda s:s.rsplit('}',1)[-1]

def stages(kind):
    assert kind in ['color','gradient'];return ['ready','copied']+(['cancelled'] if kind=='gradient' else [])

def palette_state(path):
    root=typed_xml(Path(path).read_bytes());assert root.tag=='{'+DRAW+'}ArrayOfColor' and not root.attrib
    values=[]
    for node in root:
        assert node.tag=='{'+DRAW+'}Color' and not node.attrib
        assert [c.tag for c in node]==['{'+DRAW+'}'+n for n in ['knownColor','name','state','value']]
        assert one(node,'{'+DRAW+'}name').attrib=={'{'+XSI+'}nil':'true'} and not (one(node,'{'+DRAW+'}name').text or '').strip()
        assert (one(node,'{'+DRAW+'}knownColor').text or '').strip()=='0' and (one(node,'{'+DRAW+'}state').text or '').strip()=='2'
        value=int(one(node,'{'+DRAW+'}value').text);assert 0<=value<=0xffffffff;values.append(value)
    return dict(palette=values,paletteXML=canonical(root),paletteSha256=sha(path))

def family_state(profile,palette):
    profile=Path(profile);store=profile/'SystemData/ModuleStore.xml';root=typed_xml(store.read_bytes())
    libraries=list(root.iter('{'+NS+'}ColorGradientLibraryStaticData'));assert len(libraries)==1
    parent=libraries[0];entries=one(parent,'{'+NS+'}_library');values=[]
    assert not entries.attrib and not (entries.text or '').strip(),'Unexpected gradient library wrapper data'
    for entry in entries:
        assert entry.tag=='{'+ARRAYS+'}KeyValueOfstringColorGradientGbtyavQ7' and [c.tag for c in entry]==['{'+ARRAYS+'}Key','{'+ARRAYS+'}Value'] and not entry.attrib and not (entry.text or '').strip() and not (entry.tail or '').strip()
        key=one(entry,'{'+ARRAYS+'}Key');assert not key.attrib and len(key)==0 and not (key.tail or '').strip(),'Unexpected gradient key structure'
        name=key.text;value=one(entry,'{'+ARRAYS+'}Value');assert not (value.tail or '').strip()
        assert isinstance(name,str) and name and name not in [e['name'] for e in values]
        values.append(dict(name=name,value=canonical(value)))
    parent.remove(entries)
    return dict(entries=values,otherXML=canonical(root),moduleStoreSha256=sha(store),**palette_state(palette))

def expected_entry(sequence,initial):
    root=typed_xml(Path(sequence).read_bytes());values=[e for e in root.iter() if local(e.tag)=='ColorGradient'];assert len(values)==1
    source=copy.deepcopy(values[0]);source.tag=initial['entries'][0]['value'][0]
    assert [local(c.tag) for c in source]==['IsCurrentLibraryGradient','_alphas','_colors','_gammacorrected','_libraryReferenceName','_title']
    assert len(one(source,'{'+NS+'}_alphas'))==3 and len(one(source,'{'+NS+'}_colors'))==3
    assert one(source,'{'+NS+'}IsCurrentLibraryGradient').text=='false';one(source,'{'+NS+'}IsCurrentLibraryGradient').text='true'
    assert not (one(source,'{'+NS+'}_libraryReferenceName').text or '').strip()
    return dict(name=COPY_NAME,value=canonical(source))

def prepare_contract(profile,kind,sequence):
    profile=Path(profile);initial=family_state(profile,profile/'ColorPalette.xml')
    assert initial['palette']==[0xffff0000,0xff00ff00,0xff0000ff,0xffffff00]
    assert [e['name'] for e in initial['entries']]==['Gradient '+c for c in 'ABCD']
    result=dict(kind=kind,initialState=initial,sourceColor=SOURCE_COLOR)
    if kind=='gradient':result['expectedEntry']=expected_entry(sequence,initial)
    result['expectedModuleStoreBackup']='SystemData/auto_backup/ModuleStore_'+time.strftime('%m%d%Y_%I%M%S',time.localtime((profile/'SystemData/ModuleStore.xml').stat().st_mtime))+'.xml'
    return result

def accept_checkpoint(name,state,m,previous):
    kind=m['kind'];order=stages(kind);assert name in order
    assert set(previous)==set(order[:order.index(name)]),'Checkpoint order changed'
    initial=m['initialState'];expected=copy.deepcopy(initial)
    if name!='ready':
        if kind=='color':
            expected['palette'].insert(1,SOURCE_COLOR)
            node=copy.deepcopy(expected['paletteXML'][3][0]);next(c for c in node[3] if local(c[0])=='value')[2]=str(SOURCE_COLOR)
            expected['paletteXML'][3].insert(1,node)
            expected['paletteSha256']=state['paletteSha256']
            assert state['paletteSha256']!=initial['paletteSha256'],'Color copy was not saved'
        else:
            expected['entries'].append(m['expectedEntry']);expected['moduleStoreSha256']=state['moduleStoreSha256']
            assert state['moduleStoreSha256']!=initial['moduleStoreSha256'],'Gradient copy was not saved'
    assert state==expected,'Exact saved family state differs at '+name
    if name=='cancelled':assert state=={k:v for k,v in previous['copied'].items() if k not in ['passed','whileAlive']},'Cancel changed persisted bytes'
    return dict(state,passed=True,whileAlive=True)

def accept_profile(out,m):
    profile=Path(out)/'profile';assert not any(p.is_symlink() for p in profile.rglob('*'))
    actual={str(p.relative_to(profile)):sha(p) for p in profile.rglob('*') if p.is_file()};initial=m['preparedProfileHashes']
    allowed={'.lock','logs/Application.Info.log'}
    if m['kind']=='gradient':allowed.add(m['expectedModuleStoreBackup'])
    assert set(initial)<=set(actual) and set(actual)-set(initial)<=allowed,'Unexpected private profile output'
    for name,digest in initial.items():
        if name=='SystemData/ModuleStore.xml' and m['kind']=='gradient':continue
        assert actual[name]==digest,'Original profile changed: '+name
    if m['kind']=='gradient':assert actual.get(m['expectedModuleStoreBackup'])==initial['SystemData/ModuleStore.xml'],'Incorrect or missing initial-byte ModuleStore backup'
    if 'logs/Application.Info.log' in actual:assert (profile/'logs/Application.Info.log').stat().st_size<4_000_000
    return dict(passed=True,sourceSequenceByteExact=True,originalPaletteByteExact=True,initialByteBackup=m['kind']=='gradient')

def verify_foreground(fg,m):
    assert fg.get("pendingKeyUpAcknowledgements")==[],"Posted keyUPs remain unacknowledged"
    assert fg.get('passed') is True,'Helper failed: '+str(fg.get('error','missing successful report'))
    for key in ['restoredPrevious','restoredPointer','targetExited','noInputHeld','cleanupCompleted','inputOwnershipMaintained','inputCompleted']:assert fg.get(key) is True,key
    assert not fg['heldButtonAfterCleanup'] and not fg['heldKeyAfterCleanup'] and fg['observedPressedKeyCodesAfterCleanup']==fg['observedPressedButtonsAfterCleanup']==[]
    gradient=m['kind']=='gradient';count=2 if gradient else 1
    assert fg['mouseDownCount']==fg['mouseUpCount']==count and fg['keyDownCount']==fg['keyUpCount']==(3 if gradient else 0)
    ids=fg['windowIDs'];assert set(ids)=={'editor'}
    pointer=[e for e in fg['events'] if 'point' in e];assert len(pointer)==31*count
    assert [e['kind'] for e in pointer]==([5,1]+[6]*28+[2])*count
    assert all(e['command'] is False and e['label']=='editor' and e['window']==ids['editor'] for e in pointer)
    source=m.get('sourcePoint',[970.,210.] if gradient else [912.,207.]);dest=m.get('destinationPoint',[37.75,81.])
    for group in range(count):
        chunk=pointer[group*31:(group+1)*31];a=chunk[0]['point'];b=chunk[-1]['point']
        assert a==chunk[1]['point']
        assert all(abs((b[i]-a[i])-(dest[i]-source[i]))<1e-7 for i in [0,1])
        for step,event in enumerate(chunk[2:30],1):assert all(abs(event['point'][i]-(a[i]+(b[i]-a[i])*step/28))<1e-7 for i in [0,1])
    keys=[e for e in fg['events'] if 'key' in e]
    expected=[(55,True,True),(0,True,True),(0,False,True),(55,False,False),(0,True,False),(0,False,False)] if gradient else []
    assert [(e['key'],e['down'],e['command']) for e in keys]==expected
    if gradient:
        for key in ['nativeNameSheetObserved','nameSheetDismissed','nativeCancelSheetObserved','cancelSheetDismissed']:assert fg.get(key) is True,key
        assert fg['enteredName']==COPY_NAME
    return dict(actualDragSessions=count,pointerEvents=len(pointer),keyEvents=len(keys),mousePairs=count,keyPairs=len(keys)//2)
