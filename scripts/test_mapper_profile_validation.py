#!/usr/bin/env python3
"""Private filesystem failure cases for the production-default shutdown allowance."""
import tempfile
from pathlib import Path
import unittest
from check_mapper_file_panels import hashes, validate_private_profile
DEFAULT='<VixenApplicationData><DataFormatVersion>3</DataFormatVersion><RecentSequences/><FilterShapePositions/><FilterSetupFormHighQualityRendering value="false"/></VixenApplicationData>'
class ProfileTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='mapper-profile-validation-')
        self.root=Path(self.temp.name);(self.root/'SystemData').mkdir();(self.root/'SystemData/original.xml').write_text('unchanged')
        self.initial=hashes(self.root)
    def tearDown(self):self.temp.cleanup()
    def check(self,text=DEFAULT):
        (self.root/'VixenApplicationData.xml').write_text(text)
        return validate_private_profile(self.root,self.initial)
    def test_empty_profile_and_exact_default_allowed(self):
        self.assertTrue(validate_private_profile(self.root,self.initial)['strictlyUnchanged'])
        result=self.check();self.assertTrue(result['accepted']);self.assertTrue(result['initialFilesUnchanged']);self.assertFalse(result['strictlyUnchanged'])
    def test_returned_default_bytes_allowed(self):
        result=self.check('\ufeff<?xml version="1.0" encoding="utf-8"?>\r\n'+DEFAULT)
        self.assertTrue(result['accepted'])
    def test_changed_or_missing_original_refused(self):
        (self.root/'SystemData/original.xml').write_text('changed');self.assertFalse(self.check()['accepted'])
        (self.root/'SystemData/original.xml').unlink();self.assertFalse(self.check()['accepted'])
    def test_unexpected_file_refused(self):
        (self.root/'other.xml').write_text('unexpected');self.assertFalse(self.check()['accepted'])
    def test_nondefault_content_schema_and_duplicate_elements_refused(self):
        bad=[DEFAULT.replace('>3<','>4<'),DEFAULT.replace('value="false"','value="true"'),DEFAULT.replace('<RecentSequences/>','<RecentSequences><Sequence>x</Sequence></RecentSequences>'),DEFAULT.replace('<FilterShapePositions/>','<FilterShapePositions><Position/></FilterShapePositions>'),DEFAULT.replace('<RecentSequences/>','<RecentSequences/><RecentSequences/>'),DEFAULT.replace('<VixenApplicationData>','<VixenApplicationData extra="1">'),DEFAULT.replace('</VixenApplicationData>','<Future/></VixenApplicationData>'),DEFAULT.replace('<RecentSequences/>','<RecentSequences>unexpected</RecentSequences>')]
        for text in bad:
            with self.subTest(text=text):self.assertFalse(self.check(text)['accepted'])
    def test_malformed_oversize_dtd_comment_pi_refused(self):
        for text in ['<bad','x'*5000,'<!DOCTYPE x [<!ENTITY x "3">]>'+DEFAULT,'<!-- data -->'+DEFAULT,'<?custom data?>'+DEFAULT,DEFAULT.replace('<RecentSequences/>','<!-- data --><RecentSequences/>')]:
            with self.subTest(text=text):self.assertFalse(self.check(text)['accepted'])
    def test_symlink_refused(self):
        (self.root/'link').symlink_to(self.root/'SystemData/original.xml');self.assertFalse(self.check()['accepted'])
    def test_preexisting_appdata_never_permits_modification(self):
        self.check();initial=hashes(self.root)
        (self.root/'VixenApplicationData.xml').write_text(DEFAULT+'\n')
        self.assertFalse(validate_private_profile(self.root,initial)['accepted'])
if __name__=='__main__':unittest.main(verbosity=2)
