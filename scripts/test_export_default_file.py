#!/usr/bin/env python3
"""Offline checks for the untouched-default export checkpoint; no app launch."""
import argparse
import json
from pathlib import Path
import struct
import tempfile
import unittest

from export_file_panel_contract import sha, verify
from prepare_export_file_panel_fixture import ROOT, prepare


class DefaultExportControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / "target", prefix="export-default-controls-")
        self.root = Path(self.temp.name) / "fixture"
        self.m = prepare(self.root)
        self.prepared = self.root / "prepared.json"
        # Retained native FSEQ, with only the fixture frame count changed.
        self.data = bytearray((ROOT / "target/export-sheet-native-final/profile/complete.csv").read_bytes())
        struct.pack_into("<I", self.data, 14, 60)

    def tearDown(self):
        self.temp.cleanup()

    def write(self, key):
        Path(self.m[key]).write_bytes(self.data)

    def test_three_checkpoints_require_both_independent_exports(self):
        self.assertTrue(verify(self.prepared, "cancelled")["passed"])
        self.write("exportFile")
        first = verify(self.prepared, "exported")
        self.write("defaultExportFile")
        final = verify(self.prepared, "default-exported")
        self.assertEqual(final["fseq"]["sha256"], first["fseq"]["sha256"])
        self.assertEqual(final["defaultFseq"]["frames"], 60)
        self.assertEqual(final["defaultFileName"], "Panel Source Café.fseq")

    def test_cancel_and_first_export_reject_premature_default_output(self):
        self.write("defaultExportFile")
        with self.assertRaises(AssertionError):
            verify(self.prepared, "cancelled")
        self.write("exportFile")
        with self.assertRaises(AssertionError):
            verify(self.prepared, "exported")

    def test_doubled_extension_and_extra_outputs_are_rejected(self):
        self.write("exportFile")
        output = Path(self.m["defaultExportFile"])
        doubled = output.with_name(output.name + ".fseq")
        doubled.write_bytes(self.data)
        with self.assertRaises(AssertionError):
            verify(self.prepared, "default-exported")
        self.write("defaultExportFile")
        with self.assertRaises(AssertionError):
            verify(self.prepared, "default-exported")
        doubled.unlink()
        self.assertTrue(verify(self.prepared, "default-exported")["passed"])

    def test_default_checkpoint_checks_bytes_and_keeps_first_output_required(self):
        self.write("exportFile")
        self.write("defaultExportFile")
        output = Path(self.m["defaultExportFile"])
        output.write_bytes(self.data + b"unexpected trailing bytes")
        with self.assertRaises(AssertionError):
            verify(self.prepared, "default-exported")
        self.write("defaultExportFile")
        Path(self.m["exportFile"]).unlink()
        with self.assertRaises(AssertionError):
            verify(self.prepared, "default-exported")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(DefaultExportControls))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(dict(
        passed=result.wasSuccessful(), testsRun=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), nativeOperations=0,
        scope="Offline checkpoint controls using a retained native FSEQ with a synthetic 60-frame count; no new native save claimed.",
        pins={str(p): sha(p) for p in [Path(__file__).resolve(), ROOT / "scripts/export_file_panel_contract.py", ROOT / "scripts/prepare_export_file_panel_fixture.py"]},
    ), indent=2) + "\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
