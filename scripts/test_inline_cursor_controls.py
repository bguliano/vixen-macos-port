#!/usr/bin/env python3
"""Offline strict capture metadata faults; never imports or invokes native APIs."""
import copy,json,struct,tempfile,unittest,zlib
from pathlib import Path
import check_inline_cursor_foreground as c
class Controls(unittest.TestCase):
 def fixture(self,o):
  r=json.loads((c.REFERENCE/'result.json').read_text());fg=copy.deepcopy(r['foreground']);host=json.loads((c.REFERENCE/'events.json').read_text())
  fg.update(restoredPointer=True,ownershipLost=False,pendingKeyUpAcknowledgements=[],mouseDowns=3,mouseUps=3,keyDowns=1,keyUps=1);fg['pointerEvents'].append(dict(type=5));fg['captures']=[]
  frame=host['windowFrame'];x=int(frame[0])+10;y=int(host['screenTop']-frame[1]-frame[3])+40;w,h=250,100
  def chunk(t,data):return struct.pack('>I',len(data))+t+data+struct.pack('>I',zlib.crc32(t+data)&0xffffffff)
  data=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+b'\xff'*w*3)*h))+chunk(b'IEND',b'')
  for name,index in zip(c.NAMES,[2,6,9,10,11,14,15,16]):
   p=o/(name+'.png');p.write_bytes(data);state=fg['observations'][index]['observation']['current']
   fg['captures'].append(dict(name=name,path=str(p),sha256=c.sha(p),cropCG=[x,y,w,h],pixelSize=[w,h],cursorRequested=True,screenCapturePreflight=True,captureArguments=['-x','-C',f'-R{x},{y},{w},{h}',str(p)],cursorCG=[x+50,y+50],windowNumber=host['windowNumber'],elapsedSeconds=10,passiveState=state))
  return fg,host
 def test_exact_capture_contract(self):
  with tempfile.TemporaryDirectory() as d:
   fg,h=self.fixture(Path(d));r=c.verify(Path(d),{},fg,h);self.assertTrue(r['manualVisualReviewRequired']);self.assertFalse(r['visualAppearanceAccepted'])
 def test_semantic_capture_faults(self):
  with tempfile.TemporaryDirectory() as d:
   fg,h=self.fixture(Path(d))
   changes=[lambda f:f.update(pendingKeyUpAcknowledgements=[56]),lambda f:f['captures'][0].update(cursorRequested=False),lambda f:f['captures'][0].update(path='/tmp/unowned.png'),lambda f:f['captures'][0].update(sha256='0'*64),lambda f:f['captures'][0].update(cropCG=[-10000,-10000,250,100]),lambda f:f['captures'][0]['passiveState'].update(tooltips=[]),lambda f:f.update(mouseUps=2),lambda f:f['captures'].reverse()]
   for mutate in changes:
    f=copy.deepcopy(fg);mutate(f)
    with self.assertRaises(AssertionError):c.verify(Path(d),{},f,h)
 def test_deadline_refuses_insufficient_budget(self):
  import time
  with self.assertRaises(AssertionError):c.remaining(time.monotonic()+1,26)
 def test_forced_cleanup_contains_capture_descendants(self):
  from unittest.mock import Mock,patch
  import time,signal
  child=Mock(pid=31415);child.poll.return_value=None;child.wait.return_value=0
  with patch.object(c.os,'killpg') as kill:c.terminate_owned(child,time.monotonic()+2,True)
  self.assertEqual(kill.call_args_list,[unittest.mock.call(31415,signal.SIGTERM),unittest.mock.call(31415,signal.SIGKILL)])
  child.terminate.assert_not_called();child.kill.assert_not_called()
 def test_exited_helper_still_cleans_owned_descendant_group(self):
  from unittest.mock import Mock,patch
  import time,signal
  child=Mock(pid=31416);child.poll.return_value=0;child.wait.return_value=0
  with patch.object(c.os,'killpg') as kill:c.terminate_owned(child,time.monotonic()+2,True)
  self.assertEqual(kill.call_args_list,[unittest.mock.call(31416,signal.SIGTERM),unittest.mock.call(31416,signal.SIGKILL)])
 def test_partial_foreground_retains_terminal_report(self):
  from unittest.mock import patch
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);(o/'foreground.json').write_text('{')
   with patch.object(c,'validate',side_effect=AssertionError('preflight blocked')),patch.object(c,'preferences',return_value={}):
    self.assertEqual(c.run(o,dict(scope='offline control',pins={})),1)
   report=json.loads((o/'result.json').read_text());self.assertIn('foregroundReadError',report);self.assertTrue(report['hostExited'] and report['driverExited']);self.assertFalse(report['passed']);self.assertIn('ownerPreferencesUnchanged',report)
if __name__=='__main__':
 result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls))
 c.save(c.ROOT/'target/inline-cursor-fix-01/consumer-controls.json',dict(passed=result.wasSuccessful(),methods=result.testsRun,semanticNegativeCases=8,nativeOperations=0));raise SystemExit(not result.wasSuccessful())
