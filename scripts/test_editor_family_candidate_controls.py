#!/usr/bin/env python3
"""Read-only candidate-binding faults using private synthetic metadata; no app launch."""
import argparse,json,plistlib,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_editor_family_physical as r

class Controls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(dir=r.ROOT/'target',prefix='family-candidate-controls-');self.out=Path(self.temp.name)
        self.app=self.out/'private/Vixen.app';self.source=self.out/'canonical/Vixen.app';self.inputs=self.out/'inputs.json'
        for app,data in [(self.app,b'private signed bytes'),(self.source,b'canonical signed bytes')]:
            exe=app/'Contents/MacOS/Vixen';exe.parent.mkdir(parents=True);exe.write_bytes(data)
        (self.app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':'com.vixenlights.verification.mock'}))
        self.approved=r.sha(self.source/'Contents/MacOS/Vixen');self.clone=self.app.parent/'private-app.json'
        self.metadata=dict(passed=True,signatureVerified=True,sourceUnchanged=True,app=str(self.app),sourceApp=str(self.source),sourceExecutableSha256=self.approved,executableSha256=r.sha(self.app/'Contents/MacOS/Vixen'),executableCodeDigest='digest',bundleIdentifier='com.vixenlights.verification.mock')
        self.clone.write_text(json.dumps(self.metadata));self.inputs.write_text(json.dumps({'sources':{f'__candidate_mock__/{i}':'0'*64 for i in range(808)}}))
        self.original_sha=r.sha
    def tearDown(self):self.temp.cleanup()
    def invoke(self,approved=None,drift=False,digest='digest'):
        def sha(path):
            rel=str(path)
            if '/__candidate_mock__/' in rel:return '1'*64 if drift and rel.endswith('/807') else '0'*64
            return self.original_sha(path)
        with patch.object(r,'sha',sha),patch.object(r,'code_digest',return_value=digest):
            return r.candidate_binding(self.app,approved or self.approved,self.source,self.inputs)
    def test_exact_candidate_and_all_inputs_bind(self):
        got=self.invoke();self.assertEqual(got['approvedSourceSha256'],self.approved);self.assertEqual(got['app'],str(self.app));self.assertEqual(got['buildInputs'],str(self.inputs))
    def test_wrong_approval_or_code_digest_refused(self):
        with self.assertRaises(AssertionError):self.invoke(approved='f'*64)
        with self.assertRaises(AssertionError):self.invoke(digest='different')
    def test_source_drift_and_incomplete_manifest_refused(self):
        with self.assertRaises(AssertionError):self.invoke(drift=True)
        data=json.loads(self.inputs.read_text());data['sources'].pop('__candidate_mock__/807');self.inputs.write_text(json.dumps(data))
        with self.assertRaises(AssertionError):self.invoke()
    def test_clone_identity_and_audit_failure_refused(self):
        for key,value in [('passed',False),('signatureVerified',False),('sourceUnchanged',False),('bundleIdentifier','com.vixenlights.vixen3.mac'),('sourceApp',str(self.out/'other.app')),('executableSha256','f'*64)]:
            self.clone.write_text(json.dumps(dict(self.metadata,**{key:value})))
            with self.assertRaises(AssertionError,msg=key):self.invoke()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);a=p.parse_args();result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls));a.report.parent.mkdir(parents=True,exist_ok=True)
    r.save(a.report,dict(passed=result.wasSuccessful(),testsRun=result.testsRun,errors=len(result.errors),failures=len(result.failures),nativeProcessesLaunched=0,scope='Actual candidate binding with synthetic files and mocked Mach-O digest/production source reads; exact approval, clone identity, code digest and808-source refusal checks.',sourcePins={str(path.resolve()):r.sha(path) for path in [Path(__file__),Path(r.__file__)]}));raise SystemExit(0 if result.wasSuccessful() else 1)
