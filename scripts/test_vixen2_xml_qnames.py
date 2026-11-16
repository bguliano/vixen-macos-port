#!/usr/bin/env python3
"""Offline QName identity controls; never launch the native app."""
import argparse,json,unittest
from pathlib import Path
import vixen2_physical_contract as c
XSD='http://www.w3.org/2001/XMLSchema'
ROOT=Path(__file__).resolve().parents[1]
class QNameControls(unittest.TestCase):
    def xml(self,prefix='c',namespace=XSD,kind='int',value='11'):
        return f'<root xmlns:i="{c.XSI}"><value xmlns:{prefix}="{namespace}" i:type="{prefix}:{kind}">{value}</value></root>'.encode()
    def parse(self,data):return c.canonical(c.typed_xml(data))
    def test_different_bound_aliases_same_type(self):
        self.assertEqual(self.parse(self.xml()),self.parse(self.xml('d')))
    def test_type_namespace_and_numeric_text_changes_remain_distinct(self):
        baseline=self.parse(self.xml())
        for data in [self.xml(namespace='urn:wrong'),self.xml(kind='double'),self.xml(value='12')]:self.assertNotEqual(baseline,self.parse(data))
    def test_undefined_and_malformed_qnames_refused(self):
        for value in ['c:int','a:b:c',':int','c:','bad value']:
            with self.subTest(value=value),self.assertRaises(AssertionError):self.parse(f'<r xmlns:i="{c.XSI}" i:type="{value}"/>'.encode())
    def test_local_shadowing_and_scope_restore(self):
        data=f'<r xmlns:i="{c.XSI}" xmlns:c="{XSD}"><a i:type="c:int"/><b xmlns:c="urn:wrong" i:type="c:int"/><d i:type="c:int"/></r>'.encode()
        root=c.typed_xml(data);key='{'+c.XSI+'}type'
        self.assertEqual([x.get(key) for x in root],['{'+XSD+'}int','{urn:wrong}int','{'+XSD+'}int'])
    def test_default_namespace_and_unqualified_type_distinct(self):
        key='{'+c.XSI+'}type'
        namespaced=c.typed_xml(f'<r xmlns="{XSD}" xmlns:i="{c.XSI}" i:type="int"/>'.encode())
        plain=c.typed_xml(f'<r xmlns:i="{c.XSI}" i:type="int"/>'.encode())
        self.assertEqual(namespaced.get(key),'{'+XSD+'}int');self.assertEqual(plain.get(key),'int');self.assertNotEqual(namespaced.get(key),plain.get(key))
    def test_non_type_attribute_values_stay_lexical(self):
        a=f'<r xmlns:c="{XSD}" text="c:int"/>'.encode();b=f'<r xmlns:d="{XSD}" text="d:int"/>'.encode();self.assertNotEqual(self.parse(a),self.parse(b))
    def test_valid_source40_types_and_invalid_full06_fixture(self):
        source=ROOT/'target/library-drag-fixture/profile/SystemData/ModuleStore.xml'
        root=c.typed_xml(source.read_bytes());key='{'+c.XSI+'}type';types=[x.get(key) for x in root.iter() if key in x.attrib]
        self.assertEqual(types.count('{'+XSD+'}int'),16);self.assertEqual(types.count('{'+XSD+'}double'),24)
        out=ROOT/'target/vixen2-full-physical-06';m=json.loads((out/'prepared.json').read_text());backup=out/'profile'/m['expectedModuleStoreBackup']
        with self.assertRaisesRegex(AssertionError,'Undefined xsi:type namespace prefix: c'):c.typed_xml(backup.read_bytes())
        actual=c.typed_xml((out/'profile/SystemData/ModuleStore.xml').read_bytes());actualtypes=[x.get(key) for x in actual.iter() if key in x.attrib]
        self.assertEqual(actualtypes.count('int'),16);self.assertEqual(actualtypes.count('double'),24);self.assertNotEqual(types,actualtypes)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);a=p.parse_args();result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(QNameControls));a.report.parent.mkdir(parents=True,exist_ok=True);a.report.write_text(json.dumps(dict(passed=result.wasSuccessful(),testsRun=result.testsRun,errors=len(result.errors),failures=len(result.failures),nativeOperations=0,scope='Exact namespace-aware QName identity, original source40type bindings, and invalid Full06 fixture refusal.'),indent=2)+'\n');raise SystemExit(0 if result.wasSuccessful() else 1)
