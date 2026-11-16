#!/usr/bin/env python3
"""Offline controls for Vixen2 full-run disk/checkpoint acceptance; no native app."""
import argparse,copy,json,tempfile,time,unittest,uuid
from pathlib import Path
from unittest.mock import patch
import xml.etree.ElementTree as ET
import test_vixen2_full_physical as runner
import vixen2_physical_contract as c

ROOT=runner.ROOT
RETAINED=ROOT/'target/vixen2-release-ui-regression-v2/profile/SystemData/ModuleStore.xml'
N='{'+c.NS+'}';A='{'+c.ARRAYS+'}';D='{'+c.DRAW+'}'
DEFAULT='<VixenApplicationData><DataFormatVersion>3</DataFormatVersion><RecentSequences/><FilterShapePositions/><FilterSetupFormHighQualityRendering value="false"/></VixenApplicationData>'

def synthetic_root(data):
    # Preserve actual QName namespace identity when these explicitly synthetic
    # controls serialize mutations. Plain ElementTree parse/write loses namespace
    # declarations used only by xsi:type values.
    root=c.typed_xml(data);key='{'+c.XSI+'}type'
    for node in root.iter():
        if key in node.attrib and node.get(key).startswith('{'):
            node.set(key,ET.QName(node.get(key)))
    return root

class ContractControls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='vixen2-full-controls-');self.out=Path(self.temp.name)
        self.profile=self.out/'profile';(self.profile/'SystemData').mkdir(parents=True)
        self.store=self.profile/'SystemData/ModuleStore.xml'
        self.ids=[str(uuid.uuid5(uuid.NAMESPACE_URL,'private-vixen2-physical-'+str(i))) for i in range(1,5)]
        self.expected=c.expected_rows(runner.SAMPLE,self.ids)
        root=synthetic_root(RETAINED.read_bytes());self.module=next(x for x in root.iter('Module') if x.get('assemblyName')=='Module.SequenceType.Vixen2x')
        entry=c.one(c.one(self.module,N+'Vixen2xSequenceStaticData'),N+'vixen2xMappings')[0]
        c.one(entry,A+'Key').text=runner.MAP_NAME
        for node,row in zip(c.one(entry,A+'Value'),self.expected):
            for tag,key in [('ChannelName','name'),('ChannelNumber','number'),('ChannelOutput','output'),('ElementNodeId','target')]:c.one(node,N+tag).text=row[key]
            c.one(node,N+'RgbPixel').text='false'
            for tag,key in [('ChannelColor','sourceColor'),('DestinationColor','importColor')]:
                for part,value in row[key].items():c.one(c.one(node,N+tag),D+part).text=str(value)
        # Explicitly synthetic final-state fixture based on retained real output;
        # these controls validate the oracle, never claim native interaction.
        self.root=root;self.write();self.good=self.store.read_bytes()
        self.m={'expectedRows':self.expected,'initialOtherXML':c.other_xml(self.store)}
    def tearDown(self):self.temp.cleanup()
    def write(self):ET.ElementTree(self.root).write(self.store,encoding='utf-8',xml_declaration=True)
    def require(self):return c.require_saved(self.store,runner.MAP_NAME,self.expected,self.m['initialOtherXML'])
    def rows(self):return list(self.module.iter(N+'ChannelMapping'))
    def test_retained_actual_schema(self):
        state=c.saved_state(RETAINED,'V2 Test Map');self.assertEqual(len(state['rows']),6)
    def test_explicit_source_approval_and_discovery_default_restoration(self):
        canonical=self.out/'canonical/Vixen.app';(canonical/'Contents/MacOS').mkdir(parents=True);(canonical/'Contents/MacOS/Vixen').write_bytes(b'offline dummy executable; never launched')
        app=self.out/'clone/Vixen.app';app.mkdir(parents=True);approved=runner.sha(canonical/'Contents/MacOS/Vixen')
        runner.save(app.parent/'private-app.json',dict(sourceApp=str(canonical),sourceExecutableSha256=approved))
        helper=self.out/'not-launched';build=self.out/'mock-inputs.json'
        for code,source,inputs in [('not-a-sha',canonical,build),(approved,None,build),(approved,canonical,None),(approved,self.out/'wrong-source',build),('0'*64,canonical,build)]:
            with self.subTest(code=code,source=source,inputs=inputs),patch.object(runner.discovery,'prepare') as prepare:
                with self.assertRaises(AssertionError):runner.prepare(self.out/'unused',app,helper,code,source,inputs)
                prepare.assert_not_called()
        original=runner.discovery.RELEASE
        def preparation_failure(*args):
            self.assertEqual(runner.discovery.RELEASE,approved);raise RuntimeError('mock preparation failure')
        with patch.object(runner.discovery,'prepare',side_effect=preparation_failure):
            with self.assertRaisesRegex(RuntimeError,'mock preparation'):runner.prepare(self.out/'unused',app,helper,approved,canonical,build)
        self.assertEqual(runner.discovery.RELEASE,original)
    def test_synthetic_expected_state_and_serialized_manifest(self):
        self.assertEqual(self.require()['rows'],self.expected)
        self.m=json.loads(json.dumps(self.m));self.require()
    def test_row_corruptions_rejected(self):
        changes=[('ElementNodeId','not-a-destination'),('ChannelNumber','7'),('ChannelOutput','4'),('ChannelName','renamed'),('RgbPixel','true')]
        for tag,value in changes:
            with self.subTest(tag=tag):
                old=c.one(self.rows()[0],N+tag).text;c.one(self.rows()[0],N+tag).text=value;self.write()
                with self.assertRaises(AssertionError):self.require()
                c.one(self.rows()[0],N+tag).text=old
        for color in ['ChannelColor','DestinationColor']:
            with self.subTest(color=color):
                node=c.one(c.one(self.rows()[0],N+color),D+'value');old=node.text;node.text='4294901760';self.write()
                if color=='ChannelColor':node.text='1';self.write()
                with self.assertRaises(AssertionError):self.require()
                node.text=old
    def test_structure_and_unrelated_corruptions_rejected(self):
        mutations=[lambda: self.module.set('moduleType',c.NIL),lambda: self.rows()[0].append(ET.Element(N+'Unexpected')),lambda: self.root.append(ET.Element('Unknown'))]
        for mutate in mutations:
            with self.subTest(mutation=str(mutate)):
                self.root=synthetic_root(self.good);self.module=next(x for x in self.root.iter('Module') if x.get('assemblyName')=='Module.SequenceType.Vixen2x');mutate();self.write()
                with self.assertRaises(AssertionError):self.require()
    def test_duplicate_map_and_missing_row_rejected(self):
        entries=c.one(c.one(self.module,N+'Vixen2xSequenceStaticData'),N+'vixen2xMappings');entries.append(copy.deepcopy(entries[0]));self.write()
        with self.assertRaises(AssertionError):self.require()
        entries.remove(entries[-1]);records=c.one(entries[0],A+'Value');records.remove(records[-1]);self.write()
        with self.assertRaises(AssertionError):self.require()
    def request(self,name,**extra):runner.save(self.out/('checkpoint-'+name+'.request'),dict(checkpoint=name,**extra))
    def test_checkpoint_positive_readback_and_byte_exact_cancel(self):
        completed={};pending={}
        with patch.object(runner.discovery,'assert_services'):
            for name in runner.CHECKPOINTS:
                self.request(name);runner.service_checkpoints(self.out,self.m,completed,pending,time.monotonic()+20)
                self.assertEqual(json.loads((self.out/('checkpoint-'+name+'.json')).read_text()),completed[name])
            self.assertEqual(completed['accepted']['moduleStoreSha256'],completed['cancelled']['moduleStoreSha256'])
            self.store.write_bytes(self.good+b'\n')
            with self.assertRaisesRegex(AssertionError,'Cancel changed'):runner.checkpoint_state(self.out,self.m,'cancelled',completed['accepted'])
    def test_checkpoint_order_and_schema_rejected(self):
        self.request('cancelled')
        with self.assertRaisesRegex(AssertionError,'Out-of-order'):runner.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
        (self.out/'checkpoint-cancelled.request').unlink();self.request('accepted',extra=True)
        with self.assertRaisesRegex(AssertionError,'Unexpected checkpoint'):runner.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
    def test_checkpoint_bad_state_never_acknowledged_and_timeout_reported(self):
        self.request('accepted');c.one(self.rows()[0],N+'ElementNodeId').text=c.NIL;self.write();completed={};pending={}
        runner.service_checkpoints(self.out,self.m,completed,pending,time.monotonic()+20)
        self.assertFalse(completed);self.assertFalse((self.out/'checkpoint-accepted.json').exists())
        pending['accepted']=time.monotonic()-1
        with self.assertRaisesRegex(AssertionError,'checkpoint failed'):runner.service_checkpoints(self.out,self.m,completed,pending,time.monotonic()+20)
        self.assertIs(json.loads((self.out/'checkpoint-accepted.json').read_text())['passed'],False)
    def test_profile_default_and_unknown_output_controls(self):
        (self.profile/'ColorPalette.xml').write_text('<Palette/>');(self.profile/'SystemData/SystemConfig.xml').write_text('<SystemConfig/>')
        self.m['profileHashes']=runner.tree(self.profile)
        backup=runner.backup_path(self.store);self.m['expectedModuleStoreBackup']=backup
        (self.profile/backup).parent.mkdir(parents=True);(self.profile/backup).write_bytes(self.store.read_bytes())
        (self.profile/'VixenApplicationData.xml').write_text(DEFAULT)
        self.assertTrue(runner.accept_profile(self.out,self.m)['passed'])
        for name,text in [('unexpected.tim','sequence'),('.lock','123@host'),('ColorPalette.xml','changed'),(backup,'wrong backup bytes'),('SystemData/auto_backup/ModuleStore_00000000_000000.xml','wrong path'),('VixenApplicationData.xml',DEFAULT.replace('<RecentSequences/>','<RecentSequences><Sequence>private.vix</Sequence></RecentSequences>'))]:
            file=self.profile/name;old=file.read_bytes() if file.exists() else None;file.write_text(text)
            with self.subTest(name=name),self.assertRaises(AssertionError):runner.accept_profile(self.out,self.m)
            if old is None:file.unlink()
            else:file.write_bytes(old)
    def test_native_observation_contract_positive_and_corruptions(self):
        pointer=[]
        for drag in [True]*5+[False]*6:
            pointer.extend(dict(kind=k,point=[100,100],window=123) for k in [5,1]+([6]*22 if drag else [])+[2])
        keys=[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False),(51,True),(51,False)]]
        stages=['initial-six-rows','after-leaf-1','after-leaf-2','after-group-leaves','after-rejected-blank','after-physical-delete','after-overwrite-cancel','after-row1-color-ownership','before-map-accept','first-reopen-exact-rows','cancelled-reopen-exact-rows']
        assigned=[0,1,2,4,4,3,3,3,3,3,3];colors=['Red','Green','Blue','White','Yellow',''];observations=[]
        for step,stage in enumerate(stages):
            rows=[]
            for i,r in enumerate(self.expected):
                target='V2 Fixture '+str(i+1) if i<assigned[step] else '';color=colors[i] if target else ''
                if step>=7 and i==0:color='Lime'
                if step>=8 and i==1:color='Red'
                rows.append([r['number'],r['output'],r['name'],colors[i],target,color,''])
            observations.append(dict(stage=stage,rows=rows))
        observations.extend(dict(stage=s,dragCompleted=True) for s in ['map-leaf-1','map-leaf-2','map-group','blank-table-drop','overwrite-cancel-drag'])
        observations.append(dict(stage='blank-drop-comparison',beforeRows=copy.deepcopy(observations[3]['rows']),afterRows=copy.deepcopy(observations[4]['rows'])))
        observations.extend([dict(stage='blank-drop-geometry',point=[200,300],table=[100,100,300,300],lastRowBottom=200),dict(stage='unchanged-active-red-observed',value='rgb 1 0 0 1'),dict(stage='color-hex',value='00FF00'),dict(stage='color-hex',value='0000FF')])
        fg=dict(mouseDowns=11,mouseUps=11,keyDowns=5,keyUps=5,events=pointer+keys,observations=observations)
        self.assertEqual(runner.accept_observations(json.loads(json.dumps(fg)),self.m)['nativeRowObservations'],11)
        mutations=[lambda d:d['events'].pop(),lambda d:d['observations'][4]['rows'][5].__setitem__(4,'V2 Fixture 1'),lambda d:d['observations'][7]['rows'][1].__setitem__(5,'Lime'),lambda d:d['observations'][-1].__setitem__('value','00FF00'),lambda d:d['observations'][10]['rows'][0].__setitem__(5,'Blue')]
        for mutate in mutations:
            changed=copy.deepcopy(fg);mutate(changed)
            with self.assertRaises(AssertionError):runner.accept_observations(changed,self.m)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--report',type=Path,required=True);args=parser.parse_args()
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(ContractControls);result=unittest.TextTestRunner(verbosity=2).run(suite)
    runner.save(args.report,dict(passed=result.wasSuccessful(),testsRun=result.testsRun,errors=len(result.errors),failures=len(result.failures),nativeOperations=0,scope='Offline parser/schema/checkpoint/profile acceptance controls, retained native schema and explicitly synthetic intended final states; no physical verification.',retainedXML=str(RETAINED),retainedXMLSha256=runner.sha(RETAINED),files={str(p):runner.sha(p) for p in [Path(__file__).resolve(),Path(runner.__file__),Path(c.__file__)]}))
    raise SystemExit(0 if result.wasSuccessful() else 1)
