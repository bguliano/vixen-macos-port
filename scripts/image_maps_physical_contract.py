#!/usr/bin/env python3
"""Strict read-only contract for Image Maps physical rename and editor Cancel."""
import copy,hashlib,time
from pathlib import Path
from vixen2_physical_contract import canonical,typed_xml
from test_image_maps_live_ui import summary
NS='http://schemas.datacontract.org/2004/07/VixenModules.App.LipSyncApp'
ARR='http://schemas.microsoft.com/2003/10/Serialization/Arrays'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def expected_store(data):
    root=typed_xml(data);modules=[m for m in root.iter('Module') if m.get('assemblyName')=='Module.App.LipSyncApp'];assert len(modules)==1
    library=modules[0].find('.//{'+NS+'}_library');assert library is not None and len(library)==4
    assert [e.find('{'+ARR+'}Key').text for e in library]==['Active Map','Delete Me','Rename Me','Overwrite Me']
    entry=library[2];key=entry.find('{'+ARR+'}Key');ref=entry.find('.//{'+NS+'}LibraryReferenceName');notes=entry.find('.//{'+NS+'}Notes');default=entry.find('.//{'+NS+'}IsDefaultMapping')
    assert [key.text,ref.text,notes.text,default.text]==['Rename Me','Rename Me','Original Rename Me','false']
    # Upstream AddMapping(insertNew) stores Clone(), which clears IsCurrentLibraryMapping.
    current=entry.find('.//{'+NS+'}IsCurrentLibraryMapping');assert current.text=='true';current.text='false'
    key.text=ref.text='Renamed Map';library.remove(entry);library.append(entry)
    return canonical(root)
def require_store(before,after):
    assert canonical(typed_xml(after))==expected_store(before),'Module data differs from exact accepted rename (including Cancel/unrelated data)'
def backups(profile):
    return {'SystemData/auto_backup/'+name+'_'+time.strftime('%m%d%Y_%I%M%S',time.localtime((profile/'SystemData'/(name+'.xml')).stat().st_mtime))+'.xml':sha(profile/'SystemData'/(name+'.xml')) for name in ['ModuleStore','SystemConfig']}
def require_profile(profile,m,hashes):
    initial=m['profileHashes'];after=hashes(profile)
    require_store((profile.parent/'initial-ModuleStore.xml').read_bytes(),(profile/'SystemData/ModuleStore.xml').read_bytes())
    assert canonical(typed_xml((profile/'SystemData/SystemConfig.xml').read_bytes()))==m['systemConfig'],'SystemConfig semantics changed'
    expected={k.replace('Module Data Files/LipSync/Rename Me/','Module Data Files/LipSync/Renamed Map/'):v for k,v in initial.items() if k not in ['SystemData/ModuleStore.xml','SystemData/SystemConfig.xml']}
    operational={'.lock','logs/Application.Info.log'}|set(m['backups'])
    assert set(after)==set(expected)|{'SystemData/ModuleStore.xml','SystemData/SystemConfig.xml'}|operational,'Unexpected/missing private profile output'
    assert all(after[k]==v for k,v in expected.items()),'Unrelated input/sequence/image bytes changed'
    assert all(after[k]==v for k,v in m['backups'].items()),'Backup differs from exact pre-save configuration'
    assert not (profile/'Module Data Files/LipSync/Rename Me').exists(),'Old renamed folder remains'
    assert not list((profile.parent/'settings').rglob('*')),'Unexpected private settings file'
    return dict(renamedLibrary=True,editorCancelPreserved=True,unrelatedXMLUnchanged=True,sourceTIMUnchanged=True,renamedImageBytesUnchanged=True,backups=m['backups'])
BUTTONS={'initial-only-new':[True,False,False,False],'first-physical-selection':[True]*4,'inline-cancel-disabled':[False]*4,'cancel-only-new':[True,False,False,False],'same-row-after-cancel':[True]*4,'inline-accept-disabled':[False]*4,'commit-only-new':[True,False,False,False],'same-row-after-commit':[True]*4}
def require_foreground(f):
    assert all(f[k] for k in ['passed','inputCompleted','cleanupCompleted','noInputHeld','restoredPrevious','restoredPointer'])
    assert f['pendingMouseUp'] is False
    timing=f['doubleClickTiming'];assert 0<timing['elapsedSeconds']<timing['maximumSeconds']<=2
    assert f['ownershipLost'] is False and f['cleanupErrors']==[] and f['pendingKeyUpAcknowledgements']==[]
    assert f['physicalAfterRestoration']=={'keys':[],'buttons':[],'modifiers':0}
    assert [f[k] for k in ['mouseDowns','mouseUps','keyDowns','keyUps']]==[7,7,2,2]
    mouse=[e for e in f['events'] if 'kind' in e];keys=[e for e in f['events'] if 'key' in e]
    assert [e['kind'] for e in mouse]==[5,1,2]*5+[5,1,2,1,2]
    assert [e['clickCount'] for e in mouse]==[1]*18+[2,2]
    assert all(type(e['window']) is int and e['window']>0 for e in mouse)
    assert keys==[{'key':53,'down':True},{'key':53,'down':False},{'key':36,'down':True},{'key':36,'down':False}]
    observations=f['observations'];delays=[o['seconds'] for o in observations if o['stage']=='selected-name-delay'];assert len(delays)==3 and all(d>=0.75 for d in delays)
    buttons=[o for o in observations if o['stage'] in BUTTONS]
    assert [o['stage'] for o in buttons]==list(BUTTONS)
    assert all(o['buttons']==BUTTONS[o['stage']] for o in buttons)
    def only(stage):
        found=[o for o in observations if o['stage']==stage];assert len(found)==1,stage;return found[0]
    assert only('pending-rename-cancel')['value']=='Discarded Rename' and only('pending-rename-accept')['value']=='Renamed Map'
    assert only('pending-editor-notes')['value']=='Discarded editor notes' and only('cancelled-editor-notes')['value']=='Original Rename Me'
    assert all(only(stage)['moduleBytesUnchanged'] for stage in ['cancel-keeps-saved-bytes','editor-cancel-keeps-saved-bytes'])
    return dict(physicalMouseCycles=7,delayedSelectedNameDoubleClicks=1,keyPairs=2)
def require_render(rows):
    probes=[summary(__import__('json').loads(r['detail'])) for r in rows if '"effects":' in r.get('detail','')]
    assert len(probes)==2;before,after=probes
    assert before['1']['colors']=={'FFFF0000':32,'FF000000':96} and before['2']['colors']=={} and before['3']['colors']=={'FFFFFFFF':1}
    assert after['3']==before['3']
    for k in ['1','2']:
        want=copy.deepcopy(before[k]);want['generation']+=1;want['rendered']+=1
        assert after[k]==want,'LipSync exact single rerender/cache contents changed'
    return probes
