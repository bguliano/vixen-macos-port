"""Explicit free public package fetch; normal oracle builds never use the network."""
from pathlib import Path
from hashlib import sha256
from urllib.request import urlopen
import json
here = Path(__file__).resolve().parent
info = json.loads((here/'references.json').read_text())['naudio.core.2.3.0.nupkg']
data = urlopen(info['url'], timeout=30).read()
assert sha256(data).hexdigest() == info['sha256'], 'NAudio package hash mismatch'
path = here.parents[2]/'target/helper-reference/naudio.core.2.3.0.nupkg'
path.parent.mkdir(parents=True, exist_ok=True)
path.write_bytes(data)
print(path)
