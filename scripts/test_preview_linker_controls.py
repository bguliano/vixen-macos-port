#!/usr/bin/env python3
import copy,json,shutil,tempfile,time,unittest
from pathlib import Path
from unittest.mock import patch
import test_preview_linker_physical as r
class Controls(unittest.TestCase):
 def fixture(self,o):
  shutil.copytree(r.SOURCE,o/'profile');shutil.rmtree(o/'profile/SystemData/auto_backup');(o/'profile/.lock').unlink()
  return r.contract.prepare_private(o/'profile',r.WEB)
 def accepted(self,o):
  m=self.fixture(o);p=o/'profile/SystemData/ModuleStore.xml';s=p.read_text();a=s.index('<a:Name>Test String 1</a:Name>');b=s.index('</a:Shape>',a)
  assert s[a:b].count(r.contract.IDS['Fixture Two'])==50
  s=s[:a]+s[a:b].replace(r.contract.IDS['Fixture Two'],r.contract.IDS['Fixture One'])+s[b:];p.write_text(s);return m
 def test_exact_fifty_link_change_and_namespaces(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);m=self.accepted(o);self.assertEqual(r.contract.saved(o/'profile',m)['nodeIDs'],[r.contract.IDS['Fixture One']]*50)
 def test_no_op_partial_links_unrelated_shape_and_graph_rejected(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);m=self.fixture(o)
   with self.assertRaises(AssertionError):r.contract.saved(o/'profile',m)
  for kind in ['partial','shape','graph','qname']:
   with tempfile.TemporaryDirectory() as d:
    o=Path(d);m=self.accepted(o);p=o/'profile/SystemData/ModuleStore.xml';s=p.read_text()
    if kind=='partial':s=s.replace(r.contract.IDS['Fixture One'],r.contract.IDS['Fixture Two'],1)
    if kind=='shape':s=s.replace('Test String 2','Changed unrelated string',1)
    if kind=='qname':s=s.replace('i:type="a:PreviewLine"','i:type="undefined:PreviewLine"',1)
    p.write_text(s)
    if kind=='graph':p=o/'profile/SystemData/SystemConfig.xml';p.write_text(p.read_text().replace('Lamp-3','Changed graph',1))
    with self.assertRaises(AssertionError):r.contract.saved(o/'profile',m)
 def fg(self):
  names=['initial','after-valid','after-blank','reopened-saved','unsaved-double-click'];values=['Fixture Two','Fixture One','Fixture One','Fixture One','Fixture Two']
  obs=[dict(stage=a,linkedRowLabels=['1',n],rowFrame=[310,100,300,20],tableFrame=[310,100,300,300]) for a,n in zip(names,values)]
  obs += [dict(stage=n+'-payload',source=s,payload=[r.contract.IDS[s]],changeCountBefore=1,changeCountDuring=2,start=[50,100],end=e) for n,s,e in [('valid-lower-half','Fixture One',[400,115]),('blank','Fixture Two',[400,380])]]
  kinds=([5,1]+[6]*30+[2])*2+[5,1,2]+[5,1,2,1,2]
  fg={k:True for k in ['passed','cleanupCompleted','noInputHeld','restoredPrevious','restoredPointer','beforeSaveUnchanged','cancelStoreUnchanged']}
  fg.update(ownershipLost=False,pendingMouseUp=False,mouseDowns=5,mouseUps=5,keyEvents=0,windowIDs={'drop':5,'cancel':6},observations=obs,events=[dict(kind=k,window=5 if i<66 else 6,label='drop' if i<66 else 'cancel',clickState=1 if i<72 else 2) for i,k in enumerate(kinds)])
  fg.update(doubleClickActualGap=.15,doubleClickInterval=.5,doubleClickDownTimesSeconds=[10,10.15]);fg['events'][70]['elapsedSeconds']=10;fg['events'][72]['elapsedSeconds']=10.15
  fg['observations'].append(dict(stage='double-click-target-selection',selectedRows=1,targetPoint=[400,115],sourcePoint=[80,160],source='Fixture Two',sourceID=r.contract.IDS['Fixture Two']))
  for index,payload in enumerate([x for x in obs if x['stage'].endswith('-payload')]):
   start,end=payload['start'],payload['end'];points=[start,start]+[[start[0]+i*3,start[1]] for i in range(1,7)]+[[start[0]+18+(end[0]-start[0]-18)*i/24,start[1]+(end[1]-start[1])*i/24] for i in range(1,25)]+[end]
   for event,point in zip(fg['events'][index*33:(index+1)*33],points):event['point']=point
  for event,point in zip(fg['events'][66:],[[400,115]]*3+[[80,160]]*5):event['point']=point
  return fg
 def test_foreground_exact_events_and_meaningful_mutations(self):
  m=dict(elementIDs=r.contract.IDS);fg=self.fg();r.verify_foreground(fg,m)
  for mutate in [lambda f:f.update(pendingMouseUp=True),lambda f:f['events'][73].update(clickState=1),lambda f:f['events'][0].update(window=7),lambda f:f['observations'][1].update(linkedRowLabels=['1','Fixture Two']),lambda f:f['observations'][-3].update(end=[400,110]),lambda f:f['observations'][-2].update(end=[400,115]),lambda f:f.update(doubleClickActualGap=.7),lambda f:f['events'][20].update(point=[0,0])]:
   x=copy.deepcopy(fg);mutate(x)
   with self.assertRaises(AssertionError):r.verify_foreground(x,m)
 def test_harness_readback_refuses_wrong_row_and_noop(self):
  r.contract.info(json.dumps([dict(number=1,name='Fixture One',node=r.contract.IDS['Fixture One'])]),'Fixture One')
  for v in [[],[dict(number=1,name='Fixture One',node=r.contract.IDS['Fixture Two'])],[dict(number=2,name='Fixture One',node=r.contract.IDS['Fixture One'])]]:
   with self.assertRaises(AssertionError):r.contract.info(json.dumps(v),'Fixture One')
 def test_preflight_failure_and_partial_report_retains_cleanup(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);(o/'foreground.json').write_text('{')
   with patch.object(r,'validate',side_effect=AssertionError('offline refusal')),patch.object(r,'owner_states',return_value={}),patch.object(r,'hashes',return_value={}):self.assertEqual(r.run(o,dict(scope='offline',pins={},sourceHashes={})),1)
   proof=json.loads((o/'result.json').read_text());self.assertFalse(proof['passed']);self.assertTrue(proof['appExited'] and proof['helperExited']);self.assertTrue(proof['cleanupErrors'])
  with self.assertRaises(AssertionError):r.remaining(time.monotonic()+2,66)
if __name__=='__main__':
 result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls));r.save(r.ROOT/'target/preview-linker-helper-01/consumer-controls.json',dict(passed=result.wasSuccessful(),methods=result.testsRun,nativeOperations=0));raise SystemExit(not result.wasSuccessful())
