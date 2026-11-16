"""Exact private Preview linker saved-state contract; no UI or production writes."""
import copy,hashlib,json,re,time
from pathlib import Path
from vixen2_physical_contract import typed_xml,canonical
SHAPES='http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview.Shapes'
LOCATION='3fb53423-dd2a-4719-b3e3-19aa6f062f62'
SEEDED_LOCATION='3ab28349-1cce-44ad-9a41-460b78ae3756'
IDS={'Fixture One':'ebda06c0-04fc-46b0-a0a9-b8d09865d318','Fixture Two':'70debba1-02fe-4aa2-8d81-51c016f1c811'}
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def first_shape(root):
 shapes=[e for e in root.iter('{'+SHAPES+'}Shape') if e.findtext('{'+SHAPES+'}Name')=='Test String 1']
 assert len(shapes)==1;return shapes[0]
def nodes(root):
 s=first_shape(root);assert s.findtext('{'+SHAPES+'}StringType')=='Standard'
 p=s.find('{'+SHAPES+'}Pixels');assert len(p)==50
 result=[e.find('{'+SHAPES+'}NodeId') for e in p];assert all(e is not None for e in result)
 return result
def backup(path):return 'SystemData/auto_backup/'+path.stem+'_'+time.strftime('%m%d%Y_%I%M%S',time.localtime(path.stat().st_mtime))+path.suffix

def prepare_private(profile,web_reference):
 """Raw text substitutions preserve every existing scoped QName declaration."""
 store=profile/'SystemData/ModuleStore.xml';config=profile/'SystemData/SystemConfig.xml'
 s=store.read_text();r=typed_xml(store.read_bytes());shape=first_shape(r)
 assert shape.findtext('{'+SHAPES+'}StringType')=='Pixel'
 assert [e.text for e in shape.find('{'+SHAPES+'}Pixels').iter('{'+SHAPES+'}NodeId')]==[IDS['Fixture Two']]*50
 a=s.index('<a:Name>Test String 1</a:Name>');b=s.index('</a:Shape>',a);part=s[a:b];assert part.count('<a:StringType>Pixel</a:StringType>')==1
 s=s[:a]+part.replace('<a:StringType>Pixel</a:StringType>','<a:StringType>Standard</a:StringType>')+s[b:]
 assert s.count('<SaveLocations>true</SaveLocations>')==1
 s=s.replace('<SaveLocations>true</SaveLocations>','<SaveLocations>false</SaveLocations>')
 web=[x for x in re.findall(r'<Module\s[^>]*>.*?</Module>',Path(web_reference).read_text(),re.S) if 'Module.App.WebServer' in x];assert len(web)==1 and '<IsEnabled>false</IsEnabled>' in web[0]
 assert 'Module.App.WebServer' not in s and s.count('</ModuleData>')==1
 s=s.replace('</ModuleData>',web[0]+'\n</ModuleData>');store.write_text(s)
 root=typed_xml(store.read_bytes());assert [e.text for e in nodes(root)]==[IDS['Fixture Two']]*50
 cfg=typed_xml(config.read_bytes());assert len(cfg.find('Controllers'))==len(cfg.find('Patches'))==0
 assert [(e.findtext('{http://schemas.datacontract.org/2004/07/VixenModules.App.WebServer}IsEnabled')) for e in root.iter('{http://schemas.datacontract.org/2004/07/VixenModules.App.WebServer}Data')]==['false']
 expected=copy.deepcopy(root)
 for e in nodes(expected):e.text=IDS['Fixture One']
 return dict(initialXML=canonical(root),expectedXML=canonical(expected),systemXML=canonical(cfg),initialBackups={backup(p):sha(p) for p in [store,config]},elementIDs=IDS,fixtureMutations=['first StringType Pixel to Standard','SaveLocations false preserves unrelated element properties','explicit WebServer false'])

def saved(profile,m):
 store=profile/'SystemData/ModuleStore.xml';config=profile/'SystemData/SystemConfig.xml'
 r=typed_xml(store.read_bytes());assert canonical(r)==m['expectedXML'],'Saved XML differs beyond exact50 target NodeIds'
 assert [e.text for e in nodes(r)]==[IDS['Fixture One']]*50
 assert canonical(typed_xml(config.read_bytes()))==m['systemXML'],'System graph/properties changed'
 return dict(moduleStoreSha256=sha(store),systemConfigSha256=sha(config),nodeIDs=[e.text for e in nodes(r)],exactOtherXML=True)

def info(rows,expected):
 value=json.loads(rows);assert value==[dict(number=1,name=expected,node=IDS[expected])],value
 return value
