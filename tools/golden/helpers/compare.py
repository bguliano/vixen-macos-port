"""Prove regeneration with original helpers preserves the checked-in golden data."""
from pathlib import Path
from hashlib import sha256
import gzip
import json
root = Path(__file__).resolve().parents[3]
rows = []
for name in ['Bars', 'VerticalMeter', 'VUMeter', 'Waveform']:
    old = gzip.decompress((root/f'tests/golden/{name}.json.gz').read_bytes())
    new = gzip.decompress((root/f'target/golden/helpers/regenerated/{name}.json.gz').read_bytes())
    assert old == new, f'{name}: regenerated JSON differs'
    rows.append({'effect': name, 'cases': len(json.loads(new)['cases']), 'byte_identical_json': True, 'sha256': sha256(new).hexdigest()})
(root/'target/golden/helpers/comparison.json').write_text(json.dumps(rows, indent=2)+'\n')
print(f'{sum(row["cases"] for row in rows):,} cases: all four regenerated JSON files are byte-identical')
