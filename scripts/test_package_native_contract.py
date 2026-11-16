#!/usr/bin/env python3
"""Offline exact-output consumer controls, never a native package acceptance."""
import argparse,copy,json,os,shutil,tempfile,unicodedata,unittest,uuid,zipfile
import test_package_native as runner
from pathlib import Path
import xml.etree.ElementTree as ET
import package_native_contract as c
PREPARED=None
class PackageControls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='package-native-oracle-');self.out=Path(self.temp.name)/'fixture';shutil.copytree(PREPARED.parent,self.out)
        self.m=json.loads((self.out/'prepared.json').read_text())
        for key in ['profile','sourceSequence','mapFile','packageOutput','importedSequence']:self.m[key]=str(self.out/Path(self.m[key]).relative_to(PREPARED.parent))
        self.tree=dict(id=str(uuid.uuid4()),name='Root',children=self.m['expectedTreeChildren'])
    def tearDown(self):self.temp.cleanup()
    def archive(self,tree=None,extra=False,source=None,nfd_name=False):
        with zipfile.ZipFile(self.m['packageOutput'],'w',compression=zipfile.ZIP_DEFLATED) as z:
            name=unicodedata.normalize('NFD',self.m['archiveSequenceEntry']) if nfd_name else self.m['archiveSequenceEntry']
            z.writestr(name,Path(self.m['sourceSequence']).read_bytes() if source is None else source)
            z.writestr('ProfileInfo\\ElementTree.v3e',json.dumps(tree or self.tree))
            if extra:z.writestr('Unexpected.txt','extra')
    def test_exact_export_and_readback(self):
        self.archive();proof=c.export_state(json.loads(json.dumps(self.m)));self.assertTrue(proof['passed']);self.assertEqual(proof['tree'],self.tree)
        self.archive(nfd_name=True);proof=c.export_state(self.m);self.assertTrue(proof['passed']);self.assertIn(unicodedata.normalize('NFD',self.m['archiveSequenceEntry']),proof['entries'])
    def test_wrong_source_tree_or_entry_refused(self):
        bad=copy.deepcopy(self.tree);bad['children'][0]['name']='wrong tree'
        for args in [dict(source=b'wrong bytes'),dict(tree=bad),dict(extra=True)]:
            self.archive(**args)
            with self.subTest(args=args),self.assertRaises(AssertionError):c.export_state(self.m)
    def test_cancel_then_exact_import_and_corruption(self):
        self.archive();proof=c.export_state(self.m);self.assertTrue(c.cancelled_import_state(self.m,proof)['passed'])
        target=Path(self.m['importedSequence']);target.write_bytes(Path(self.m['sourceSequence']).read_bytes())
        self.assertTrue(c.imported_state(self.m,proof)['completeImportedXMLMatches'])
        with self.assertRaises(AssertionError):c.cancelled_import_state(self.m,proof)
        root=c.parsed(target);next(x for x in root.iter() if x.tag.rsplit('}',1)[-1]=='NodeId').text=str(uuid.uuid4())
        key='{http://www.w3.org/2001/XMLSchema-instance}type'
        for node in root.iter():
            if key in node.attrib and node.get(key).startswith('{'):node.set(key,ET.QName(node.get(key)))
        ET.ElementTree(root).write(target,encoding='utf-8');c.parsed(target)  # Valid QName XML; rejection must be changed content.
        with self.assertRaises(AssertionError):c.imported_state(self.m,proof)
    def test_source_map_and_archive_unchanged_guards(self):
        self.archive();proof=c.export_state(self.m)
        for key in ['sourceSequence','mapFile','packageOutput']:
            path=Path(self.m[key]);before=path.read_bytes();path.write_bytes(before+b'changed')
            with self.subTest(key=key),self.assertRaises(AssertionError):c.cancelled_import_state(self.m,proof)
            path.write_bytes(before)
    def test_exact_backup_versions_and_same_second_replacement(self):
        profile=Path(self.m['profile']);store=profile/'SystemData/ModuleStore.xml';folder=profile/'SystemData/auto_backup';folder.mkdir()
        versions=[]
        # Two different accepted byte versions deliberately share one filename.
        for content,stamp in [(b'initial',1_700_000_000_100_000_000),(b'exported',1_700_000_000_900_000_000),(b'cancelled',1_700_000_002_100_000_000)]:
            store.write_bytes(content);os.utime(store,ns=(stamp,stamp));version=runner.store_snapshot(store);versions.append(version)
            (profile/version['backupPath']).write_bytes(content)
            self.assertEqual(runner.accepted_backups(profile,versions),{v['backupPath']:v['sha256'] for v in versions})
        self.assertEqual(versions[0]['backupPath'],versions[1]['backupPath'])
        self.assertNotEqual(versions[0]['sha256'],versions[1]['sha256'])
        overwritten=profile/versions[1]['backupPath'];overwritten.write_bytes(b'initial')
        with self.assertRaises(AssertionError):runner.accepted_backups(profile,versions)
        overwritten.write_bytes(b'exported')
        extra=folder/'ModuleStore_11111111_111111.xml';extra.write_bytes(b'exported')
        with self.assertRaises(AssertionError):runner.accepted_backups(profile,versions)
        extra.unlink();bad=copy.deepcopy(versions);bad[1]['backupPath']='SystemData/auto_backup/ModuleStore_11111111_111111.xml'
        with self.assertRaises(AssertionError):runner.accepted_backups(profile,bad)
    def test_distinct_three_backup_names_required(self):
        profile=Path(self.m['profile']);store=profile/'SystemData/ModuleStore.xml';folder=profile/'SystemData/auto_backup';folder.mkdir();versions=[]
        for i in range(3):
            content=('version'+str(i)).encode();stamp=1_700_000_000_100_000_000+i*2_000_000_000
            store.write_bytes(content);os.utime(store,ns=(stamp,stamp));version=runner.store_snapshot(store);versions.append(version);(profile/version['backupPath']).write_bytes(content)
        self.assertEqual(len(runner.accepted_backups(profile,versions)),3)
        (profile/versions[0]['backupPath']).unlink()
        with self.assertRaises(AssertionError):runner.accepted_backups(profile,versions)
    def test_exact_saved_settings_and_unrelated_data(self):
        self.archive()
        path=Path(self.m['profile'])/'SystemData/ModuleStore.xml';root=c.parsed(path);module=ET.SubElement(root.find('ModuleData'),'Module',dataModelType='VixenModules.App.TimedSequenceMapper.TimedSequencePackagerData, TimedSequenceMapper',moduleType=c.MODULE,moduleInstance=c.MODULE,assemblyName=c.PACKAGER);data=ET.SubElement(module,'{'+c.NS+'}TimedSequencePackagerData')
        for tag,value in [('ModuleInstanceId',c.MODULE),('ModuleTypeId',c.MODULE),('{'+c.NS+'}ExportIncludeAudio','true'),('{'+c.NS+'}ExportOutputFile',self.m['packageOutput'])]:ET.SubElement(data,tag).text=value
        files=ET.SubElement(data,'{'+c.NS+'}ExportSequenceFiles');ET.SubElement(files,'{'+c.ARRAYS+'}string').text=self.m['sourceSequence']
        ET.ElementTree(root).write(path,encoding='utf-8');self.assertTrue(c.saved_settings(self.m)['passed'])
        data.find('{'+c.NS+'}ExportOutputFile').text=unicodedata.normalize('NFD',self.m['packageOutput']);files[0].text=unicodedata.normalize('NFD',self.m['sourceSequence']);ET.ElementTree(root).write(path,encoding='utf-8')
        self.assertTrue(c.saved_settings(self.m)['passed']);files[0].text=self.m['sourceSequence']
        data.find('{'+c.NS+'}ExportOutputFile').text='unapproved.vpkg';ET.ElementTree(root).write(path,encoding='utf-8')
        with self.assertRaises(AssertionError):c.saved_settings(self.m)
        data.find('{'+c.NS+'}ExportOutputFile').text=self.m['packageOutput'];root.append(ET.Element('Unexpected'));ET.ElementTree(root).write(path,encoding='utf-8')
        with self.assertRaises(AssertionError):c.saved_settings(self.m)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--prepared',type=Path,required=True);p.add_argument('--report',type=Path,required=True);a=p.parse_args();PREPARED=a.prepared.resolve()
    result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(PackageControls))
    a.report.write_text(json.dumps(dict(passed=result.wasSuccessful(),testsRun=result.testsRun,nativeOperations=0,scope='Offline strict-output consumer controls on synthetic expected outputs. Does not establish actual native panel or package export/import success.',prepared=str(PREPARED),preparedSha256=c.sha(PREPARED),files={str(p):c.sha(p) for p in [Path(__file__).resolve(),Path(c.__file__),Path(runner.__file__)]}),indent=2)+'\n');raise SystemExit(0 if result.wasSuccessful() else 1)
