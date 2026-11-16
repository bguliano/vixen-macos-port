#!/usr/bin/env python3
"""Read-only acceptance of retained native v5 evidence after one assertion fix.

Preserves the original false runner result. No native app or input is launched.
"""
import argparse,copy,json,shutil,tempfile
from pathlib import Path
from types import SimpleNamespace
import test_library_cross_window_foreground as runner
ROOT=Path(__file__).resolve().parents[1]
OLD="all(e['command'] is command for e in events)"
NEW="all(e['command'] is (command if i else False) for i,e in enumerate(events))"

def verify_evidence(out,m,raw):
    result=copy.deepcopy(raw)
    runner.verify(out,m,result,SimpleNamespace(returncode=raw['exitCode']),SimpleNamespace(returncode=raw['driverExitCode']))
    for name in ['ready','copied','rejected']:
        disk=json.loads((out/('checkpoint-'+name+'.json')).read_text())
        assert disk==result['checkpoints'][name]
        runner.accept_checkpoint(name,disk,m,result['checkpoints'])
    fg=result['foreground']
    for evidence in fg['rasterEvidence']:
        assert evidence['toolbarSamples']==1296 and evidence['maximumRGBDifference']<=3
        assert evidence['sourceImageSha256']==runner.sha(out/'ui'/(evidence['label']+'-ready.png'))
    keyboard=[e for e in fg['events'] if 'key' in e]
    assert [(e['key'],e['down'],e['command']) for e in keyboard]==[(55,True,True),(0,True,True),(0,False,True),(55,False,False),(0,True,False),(0,False,False),(55,True,True),(55,False,False)]
    assert fg['nameFocusIdentity']['fieldReadback']=='Cross Window Curve' and fg['nameFocusIdentity']['mainTitle']=='Destination Library – profile'
    assert fg['nameOKObservation']['actionsStatus']==0 and fg['nameOKObservation']['actions']==['AXPress']
    return result

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--capture',type=Path,required=True);parser.add_argument('--out',type=Path,required=True);args=parser.parse_args();out=args.capture.resolve();destination=args.out.resolve()
    assert out.is_relative_to(ROOT/'target') and destination.is_relative_to(ROOT/'target') and not destination.exists()
    raw_path=out/'result.json';before=runner.sha(raw_path);raw=json.loads(raw_path.read_text());m=json.loads((out/'prepared.json').read_text())
    captured_runner=out/'fixture-source/test_library_cross_window_foreground.py';current=Path(runner.__file__).resolve()
    old=captured_runner.read_text();assert old.count(OLD)==1 and old.replace(OLD,NEW)==current.read_text(),'Unexpected change beyond the single Command flag assertion'
    assert runner.sha(captured_runner)==m['frozenHashes'][str(current)]
    source_pins={}
    for source,digest in m['frozenHashes'].items():
        retained=captured_runner if Path(source)==current else Path(source)
        assert runner.sha(retained)==digest,'Captured input changed: '+source
        source_pins[source]=dict(sha256=digest,retainedAt=str(retained))
    assert raw['passed'] is False and raw['error']=='AssertionError()'
    assert raw['exitCode']==raw['driverExitCode']==0
    for key in ['processExited','driverExited','cleanupCompleted','noInputHeld','restoredPrevious','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline']:assert raw[key] is True,key
    assert runner.tree(Path(m['source']))==m['sourceHashes']
    owned=json.loads((out/'owned-processes.json').read_text());assert owned['appPID']==raw['foreground']['targetPID'] and owned['appExecutable']==m['app']
    accepted=verify_evidence(out,m,raw)
    checks=['single verifier expression is the entire source correction','all14 capture inputs retained with original hashes','normal app and helper exit0','all original corrected verifier assertions','all3 independently rechecked exact XML checkpoints','actual keyboard sequence pairs','both current normalized toolbar rasters','actual destination field and AX action metadata','owner preferences unchanged and no held input']
    with tempfile.TemporaryDirectory(prefix='cross-actual-negative-',dir=ROOT/'target') as name:
        clone=Path(name);shutil.copytree(out/'profile',clone/'profile');shutil.copytree(out/'ui',clone/'ui');shutil.copy2(out/'process.log',clone/'process.log')
        for stage in ['ready','copied','rejected']:shutil.copy2(out/('checkpoint-'+stage+'.json'),clone/('checkpoint-'+stage+'.json'))
        base_fg=json.loads((out/'foreground.json').read_text())
        for mode in ['missing-command-down','command-on-premove','wrong-window','held-input','wrong-copy-name','changed-rejection-hash']:
            fg=copy.deepcopy(base_fg);changed=copy.deepcopy(raw)
            if mode=='missing-command-down':[e for e in fg['events'] if 'point' in e][32]['command']=False
            elif mode=='command-on-premove':[e for e in fg['events'] if 'point' in e][31]['command']=True
            elif mode=='wrong-window':[e for e in fg['events'] if 'point' in e][32]['window']=fg['windowIDs']['destination']
            elif mode=='held-input':fg['noInputHeld']=False
            elif mode=='wrong-copy-name':changed['checkpoints']['copied']['entries'][-1]['name']='Wrong Curve'
            else:changed['checkpoints']['rejected']['moduleStoreSha256']='0'*64
            (clone/'foreground.json').write_text(json.dumps(fg))
            try:verify_evidence(clone,m,changed)
            except AssertionError:checks.append(mode+' rejected')
            else:raise AssertionError('Negative evidence incorrectly accepted: '+mode)
    assert runner.sha(raw_path)==before
    fg=accepted['foreground']
    report=dict(passed=True,acceptance=True,scope='Actual private d129/5bbb same-PID two-model Curve toolbar copy with native name sheet, then physical Command-linked rejection. Does not cover Color/Gradient cross-window or editor/library directions.',capture=str(out),rawResultRemainsPassed=False,rawResultSha256=before,rawFailure='Original checker incorrectly demanded Command on the positioning mouseMoved before Command-down. All actual second mouse-down/drag/up events carry Command.',correction=dict(oldExpression=OLD,newExpression=NEW,capturedRunnerSha256=runner.sha(captured_runner),correctedRunnerSha256=runner.sha(current)),checks=checks,checkCount=len(checks),steps=accepted['steps'],actualDragSessions=2,nativeSessionEndedOperations=[1,0],mousePairs=2,keyPairs=4,windowIDs=fg['windowIDs'],nameOKPressStatus=fg['nameOKPressStatus'],nameOKSameIdentityAsBeforeTyping=fg['nameOKObservation']['sameAsBeforeTyping'],nameAXInterpretation='AXCannotComplete was returned while native dismissal and exact copy completed; stale button identity is not supported by this run.',savedOrder=[v['name'] for v in accepted['checkpoints']['copied']['entries']],savedLevels=[v['points'][0][1] for v in accepted['checkpoints']['copied']['entries']],copiedAndRejectedModuleSha256=accepted['checkpoints']['copied']['moduleStoreSha256'],copiedAndRejectedBytesEqual=True,normalProcessExitCodes=[0,0],noInputHeld=True,restoredPointer=True,restoredPrevious=True,ownerPreferencesUnchanged=True,captureInputs=source_pins,consumerSha256=runner.sha(Path(__file__)),elapsedSeconds=raw['elapsedSeconds'],newInputPosted=False,newApplicationLaunched=False)
    destination.parent.mkdir(parents=True,exist_ok=True);destination.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(passed=True,checks=len(checks),report=str(destination),newInputPosted=False)))
if __name__=='__main__':main()
