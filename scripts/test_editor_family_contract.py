#!/usr/bin/env python3
"""Offline semantic corruption controls, never native gesture acceptance."""
import copy,json,shutil,tempfile,unittest
from pathlib import Path
import xml.etree.ElementTree as ET
import editor_family_contract as c
ROOT=Path(__file__).resolve().parents[1]

def element(value):
    node=ET.Element(value[0],dict(value[1]));node.text=value[2]
    for child in value[3]:node.append(element(child))
    return node

def writable(data):
    root=c.typed_xml(data)
    for node in root.iter():
        key='{'+c.XSI+'}type'
        if key in node.attrib and node.get(key).startswith('{'):node.set(key,ET.QName(node.get(key)))
    return root

class Controls(unittest.TestCase):
    def setUp(self):self.tmp=tempfile.TemporaryDirectory(prefix='editor-family-contract-');self.out=Path(self.tmp.name)
    def tearDown(self):self.tmp.cleanup()
    def setup_kind(self,kind):
        profile=self.out/'profile';shutil.copytree(ROOT/f'target/editor-family-fixture-v1/{kind}',profile);(self.out/'ui').mkdir();palette=self.out/'ui/ColorPalette.xml';shutil.copy2(profile/'ColorPalette.xml',palette)
        m=c.prepare_contract(profile,kind,profile/'Sequence'/('Editor '+kind.title()+'.tim'));m['preparedProfileHashes']={str(p.relative_to(profile)):c.sha(p) for p in profile.rglob('*') if p.is_file()}
        m=json.loads(json.dumps(m));return profile,palette,m
    def mutate_copy(self,profile,palette,m):
        if m['kind']=='color':
            root=writable(palette.read_bytes());node=copy.deepcopy(root[0]);c.one(node,'{'+c.DRAW+'}value').text=str(c.SOURCE_COLOR);root.insert(1,node);ET.ElementTree(root).write(palette,encoding='utf-8')
        else:
            store=profile/'SystemData/ModuleStore.xml';root=writable(store.read_bytes());lib=next(root.iter('{'+c.NS+'}_library'));entry=ET.Element('{'+c.ARRAYS+'}KeyValueOfstringColorGradientGbtyavQ7');ET.SubElement(entry,'{'+c.ARRAYS+'}Key').text=c.COPY_NAME;entry.append(element(m['expectedEntry']['value']));lib.append(entry)
            backup=profile/m['expectedModuleStoreBackup'];backup.parent.mkdir();shutil.copy2(store,backup);ET.ElementTree(root).write(store,encoding='utf-8')
    def test_color_exact_insert_and_wrong_value_order(self):
        profile,palette,m=self.setup_kind('color');ready=c.accept_checkpoint('ready',c.family_state(profile,palette),m,{})
        self.mutate_copy(profile,palette,m);s=c.family_state(profile,palette);accepted=c.accept_checkpoint('copied',s,m,{'ready':ready});self.assertEqual(accepted['palette'][1],c.SOURCE_COLOR)
        for key in ['palette','paletteXML','otherXML','entries']:
            bad=copy.deepcopy(s);bad[key]=[]
            with self.assertRaises(AssertionError):c.accept_checkpoint('copied',bad,m,{'ready':ready})
        bad=copy.deepcopy(s);bad['moduleStoreSha256']='a'*64
        with self.assertRaises(AssertionError):c.accept_checkpoint('copied',bad,m,{'ready':ready})
    def test_gradient_exact_copy_and_byte_exact_cancel(self):
        profile,palette,m=self.setup_kind('gradient');ready=c.accept_checkpoint('ready',c.family_state(profile,palette),m,{})
        self.mutate_copy(profile,palette,m);s=c.family_state(profile,palette);copied=c.accept_checkpoint('copied',s,m,{'ready':ready});c.accept_checkpoint('cancelled',s,m,{'ready':ready,'copied':copied})
        bad=copy.deepcopy(s);bad['moduleStoreSha256']='0'*64
        with self.assertRaises(AssertionError):c.accept_checkpoint('cancelled',bad,m,{'ready':ready,'copied':copied})
        for key in ['entries','otherXML','paletteXML']:
            bad=copy.deepcopy(s);bad[key]=[]
            with self.assertRaises(AssertionError):c.accept_checkpoint('copied',bad,m,{'ready':ready})
    def test_gradient_value_metadata_mutations(self):
        profile,palette,m=self.setup_kind('gradient');ready=c.accept_checkpoint('ready',c.family_state(profile,palette),m,{})
        self.mutate_copy(profile,palette,m);s=c.family_state(profile,palette)
        for index in range(6):
            bad=copy.deepcopy(s);bad['entries'][-1]['value'][3][index][2]='corrupt'
            with self.assertRaises(AssertionError):c.accept_checkpoint('copied',bad,m,{'ready':ready})
        bad=copy.deepcopy(s);bad['entries'][-1]['name']='Wrong'
        with self.assertRaises(AssertionError):c.accept_checkpoint('copied',bad,m,{'ready':ready})
    def test_checkpoint_order_and_serialized_initial(self):
        profile,palette,m=self.setup_kind('gradient');s=c.family_state(profile,palette);self.assertEqual(s,m['initialState'])
        with self.assertRaises(AssertionError):c.accept_checkpoint('copied',s,m,{})
        with self.assertRaises(AssertionError):c.accept_checkpoint('ready',s,m,{'copied':{}})
    def test_actual_qname_bindings_remain_required(self):
        profile,palette,m=self.setup_kind('gradient');store=profile/'SystemData/ModuleStore.xml';raw=store.read_bytes();self.assertIn(b'http://www.w3.org/2001/XMLSchema',raw)
        store.write_bytes(raw.replace(b'http://www.w3.org/2001/XMLSchema"',b'urn:wrong"'));s=c.family_state(profile,palette)
        with self.assertRaises(AssertionError):c.accept_checkpoint('ready',s,m,{})
    def test_profile_exact_initial_backup_and_unknown_files(self):
        profile,palette,m=self.setup_kind('gradient');self.mutate_copy(profile,palette,m);self.assertTrue(c.accept_profile(self.out,m)['passed'])
        backup=profile/m['expectedModuleStoreBackup'];good=backup.read_bytes();backup.write_bytes(b'wrong')
        with self.assertRaises(AssertionError):c.accept_profile(self.out,m)
        backup.write_bytes(good);extra=profile/'SystemData/auto_backup/unknown.xml';extra.write_bytes(good)
        with self.assertRaises(AssertionError):c.accept_profile(self.out,m)
    def test_color_profile_cannot_change_sequence_or_store(self):
        profile,palette,m=self.setup_kind('color');self.mutate_copy(profile,palette,m);self.assertTrue(c.accept_profile(self.out,m)['passed'])
        store=profile/'SystemData/ModuleStore.xml';store.write_bytes(store.read_bytes()+b'\n')
        with self.assertRaises(AssertionError):c.accept_profile(self.out,m)

    def test_unknown_gradient_wrapper_or_key_data_refused(self):
        profile,palette,m=self.setup_kind('gradient');store=profile/'SystemData/ModuleStore.xml';raw=store.read_bytes()
        for fault in ['library-attr','entry-attr','key-attr','key-child','library-text','key-tail','field-order']:
            root=writable(raw);library=next(root.iter('{'+c.NS+'}_library'));entry=library[0];key=c.one(entry,'{'+c.ARRAYS+'}Key')
            if fault=='library-attr':library.set('unexpected','data')
            elif fault=='entry-attr':entry.set('unexpected','data')
            elif fault=='key-attr':key.set('unexpected','data')
            elif fault=='key-child':ET.SubElement(key,'unexpected').text='data'
            elif fault=='library-text':library.text='unexpected'
            elif fault=='key-tail':key.tail='unexpected'
            else:entry[:]=list(reversed(list(entry)))
            ET.ElementTree(root).write(store,encoding='utf-8')
            with self.assertRaises(AssertionError):c.family_state(profile,palette)

    def test_native_observation_missing_cancel_wrong_window_and_path(self):
        for kind in ['color','gradient']:
            gradient=kind=='gradient';count=2 if gradient else 1;source=[970.,210.] if gradient else [912.,207.];dest=[37.75,81.];a=[x+20 for x in source];b=[x+20 for x in dest]
            fg={k:True for k in ['passed','restoredPrevious','restoredPointer','targetExited','noInputHeld','cleanupCompleted','inputOwnershipMaintained','inputCompleted','nativeNameSheetObserved','nameSheetDismissed','nativeCancelSheetObserved','cancelSheetDismissed']}
            fg.update(pendingKeyUpAcknowledgements=[],heldButtonAfterCleanup=False,heldKeyAfterCleanup=False,observedPressedKeyCodesAfterCleanup=[],observedPressedButtonsAfterCleanup=[],mouseDownCount=count,mouseUpCount=count,keyDownCount=3 if gradient else 0,keyUpCount=3 if gradient else 0,windowIDs={'editor':101},enteredName=c.COPY_NAME,events=[])
            for _ in range(count):
                points=[(5,a),(1,a)]+[(6,[a[i]+(b[i]-a[i])*step/28 for i in [0,1]]) for step in range(1,29)]+[(2,b)]
                fg['events'] += [dict(kind=event,point=point,window=101,label='editor',command=False) for event,point in points]
            if gradient:fg['events'] += [dict(key=key,down=down,command=command) for key,down,command in [(55,True,True),(0,True,True),(0,False,True),(55,False,False),(0,True,False),(0,False,False)]]
            self.assertEqual(c.verify_foreground(fg,{'kind':kind})['actualDragSessions'],count)
            for fault in ['window','point','held','pending','cancel' if gradient else 'extra-key']:
                bad=copy.deepcopy(fg)
                if fault=='window':bad['events'][2]['window']=999
                elif fault=='point':bad['events'][3]['point'][0]+=2
                elif fault=='held':bad['observedPressedKeyCodesAfterCleanup']=[55]
                elif fault=='pending':bad['pendingKeyUpAcknowledgements']=[55]
                elif fault=='cancel':bad['cancelSheetDismissed']=False
                else:bad['events'].append(dict(key=0,down=True,command=False))
                with self.assertRaises(AssertionError):c.verify_foreground(bad,{'kind':kind})

if __name__=='__main__':
    result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls))
    import sys
    sys.exit(0 if result.wasSuccessful() else 1)
