"""Exact normal sequence close settings, tied to accepted native and source defaults."""
import json
from pathlib import Path
from test_vixen2_physical import canonical,sha,tree
ROOT=Path(__file__).resolve().parents[1]
PROVENANCE=ROOT/'target/collection-picker-settings-reference.json'

def require_settings(settings,m):
    from test_collection_picker_physical import xml_root,require_collections,EXPECTED
    provenance=json.loads(PROVENANCE.read_text())
    assert provenance['passed'] and all(sha(p)==h for p,h in provenance['files'].items())
    assert sha(provenance['nativeAcceptance'])==provenance['nativeAcceptanceSha256']
    assert all(sha(p)==h for p,h in provenance['sourcePins'].items())
    refs={Path(p).name:Path(p) for p in provenance['files']}
    current=tree(settings)
    assert set(current)==set(refs)|{'ColorCollections.xml'},'Unknown/missing settings output'
    assert not any(p.is_symlink() for p in settings.rglob('*'))
    require_collections(settings/'ColorCollections.xml',EXPECTED)
    for name,reference in refs.items():
        actual=settings/name
        if name.endswith('.xml'):
            assert canonical(xml_root(actual))==canonical(xml_root(reference)),'Unexpected default preference '+name
        else:
            assert actual.stat().st_size<65536
            assert json.loads(actual.read_text())==json.loads(reference.read_text()),'Unexpected editor layout'
    return dict(passed=True,files=current,referenceProvenance=str(PROVENANCE),referenceProvenanceSha256=sha(PROVENANCE))
