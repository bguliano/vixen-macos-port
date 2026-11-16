"""Offline consumer corruption controls. Synthetic reports are not Windows proof."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('consumer', HERE / 'consume.py')
consumer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(consumer)


class Samples(unittest.TestCase):
    def setUp(self):
        self.expected = {'curves': [dict(name='a', reference='', current=True,
                                       samples=[dict(x=.5, value=.5, intValue=0)])],
                         'gradients': [dict(name='b', reference='', current=True,
                                          samples=[dict(x=.5, argb=0x7f7f7f7f)])]}
        self.actual = dict(schema=1, status='captured-validation-pending',
                           productCommit='41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a',
                           runtime='8.0.11', architecture='X64', inputBinariesUnchanged=True)
        for kind, rows in self.expected.items():
            self.actual[kind] = dict(samples=copy.deepcopy(rows), roundtripSamples=copy.deepcopy(rows))

    def reject(self, change):
        change(self.actual)
        with self.assertRaises(AssertionError):
            consumer.samples(self.expected, self.actual)

    def test_valid_serialized_readback(self):
        self.assertEqual(consumer.samples(self.expected, json.loads(json.dumps(self.actual))), {'curves': 1, 'gradients': 1})

    def test_curve_byte_numeric_changes(self):
        self.reject(lambda a: a['curves']['samples'][0]['samples'][0].update(value=.50000001))

    def test_rounding_changes(self):
        self.reject(lambda a: a['curves']['samples'][0]['samples'][0].update(intValue=1))

    def test_one_channel_changes(self):
        self.reject(lambda a: a['gradients']['samples'][0]['samples'][0].update(argb=0x7f7f7f80))

    def test_roundtrip_only_changes(self):
        self.reject(lambda a: a['gradients']['roundtripSamples'][0].update(current=False))

    def test_missing_entry(self):
        self.reject(lambda a: a['curves'].update(samples=[]))

    def test_library_link(self):
        self.reject(lambda a: a['curves']['samples'][0].update(reference='live user library'))

    def test_wrong_runtime(self):
        self.reject(lambda a: a.update(runtime='10.0.12'))

    def test_changed_install(self):
        self.reject(lambda a: a.update(inputBinariesUnchanged=False))


if __name__ == '__main__':
    unittest.main()
