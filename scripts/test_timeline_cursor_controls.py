#!/usr/bin/env python3
"""Pure consumers and bounded failure controls; no native app or input."""
import copy,json,platform,shutil,struct,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_timeline_cursor as r
class Controls(unittest.TestCase):
 def test_timeline_exact_unmodified(self):
  initial=json.loads((r.LAYOUT/'ui/timeline-state.json').read_text());self.assertEqual(r.timeline_contract(initial),initial)
  for key,value in [('modified',True),('transport','running'),('undo',['Resize 1 effect']),('drag','HResizing')]:
   bad=copy.deepcopy(initial);bad[key]=value
   with self.assertRaises(AssertionError):r.timeline_contract(bad)
  bad=copy.deepcopy(initial);bad['elements'][0]['duration']+=1
  with self.assertRaises(AssertionError):r.timeline_contract(bad)
 def test_three_hover_crops(self):
  with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
   out=Path(tmp);f=[20,50,1100,780];fg=dict(passed=True,cleanupCompleted=True,inputCompleted=True,noInputHeld=True,restoredPrevious=True,restoredPointer=True,ownershipLost=False,mouseDowns=0,mouseUps=0,keyEvents=0,windowIDs={'editor':7},frames={'editor':f},inputCutoffSeconds=18,events=[],captures=[])
   for name,p in zip(r.NAMES,r.POINTS):
    point=[f[0]+p[0],f[1]+p[1]];fg['events'].append(dict(kind=5,point=point,window=7,elapsedSeconds=7));path=out/(name+'.png');path.write_bytes(b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR'+struct.pack('>II',80,56)+b'SYNTHETIC-CONSUMER-ONLY')
    x,y=point[0]-36,point[1]-24;fg['captures'].append(dict(name=name,path=str(path),sha256=r.sha(path),pixelSize=[80,56],cropCG=[x,y,80,56],cursorCG=point,cursorRequested=True,window=7,arguments=['-x','-C',f'-R{x},{y},80,56',str(path)],elapsedSeconds=7.5))
   self.assertEqual(r.verify_hover(out,{},fg)['captures'],3)
   for mutation in ['down','key','window','path','point','cursor','held']:
    bad=copy.deepcopy(fg)
    if mutation=='down':bad['mouseDowns']=1
    elif mutation=='key':bad['keyEvents']=2
    elif mutation=='window':bad['events'][0]['window']=9
    elif mutation=='path':bad['captures'][0]['path']='/tmp/unowned.png'
    elif mutation=='point':bad['events'][0]['point'][0]+=1
    elif mutation=='cursor':bad['captures'][0]['cursorRequested']=False
    else:bad['noInputHeld']=False
    with self.assertRaises(AssertionError,msg=mutation):r.verify_hover(out,{},bad)
 def test_profile_only_owned_operational_files(self):
  with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
   out=Path(tmp);shutil.copytree(r.SOURCE,out/'profile');(out/'profile/.lock').unlink(missing_ok=True);(out/'settings').mkdir();(out/'ui').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml');m={'profileHashes':r.hashes(out/'profile')}
   (out/'profile/.lock').write_text('123@'+platform.node());log=out/'profile/logs/Application.Info.log';log.parent.mkdir(exist_ok=True);log.write_bytes(log.read_bytes()+b'fixture log' if log.exists() else b'fixture log')
   self.assertIn('.lock',r.verify_profile(out,m,123))
   extra=out/'profile/unexpected.xml';extra.write_text('unexpected')
   with self.assertRaises(AssertionError):r.verify_profile(out,m,123)
   extra.unlink();seq=out/'profile/Sequence/Editor Color.tim';seq.write_bytes(seq.read_bytes()+b' ')
   with self.assertRaises(AssertionError):r.verify_profile(out,m,123)
 def test_rejected_preflight_never_launches(self):
  with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
   out=Path(tmp)
   with patch.object(r,'validate',side_effect=AssertionError('frozen drift')),patch.object(r,'owner_states',return_value={}),patch.object(r.subprocess,'Popen') as launch:
    self.assertEqual(r.run(out,dict(scope='synthetic failure',pins={},sourceHashes=r.hashes(r.SOURCE))),1);launch.assert_not_called()
   result=json.loads((out/'result.json').read_text());self.assertFalse(result['passed']);self.assertIn('frozen drift',result['error'])
if __name__=='__main__':unittest.main()
