"""Bounded, read-only contracts for the Picture raster diagnostic."""
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import re
import stat
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BYTE_ORDER = 'ARGB8-row-major-top-down'
MAC_NAMES = {'input-requested.argb', 'input-getpixel.argb', 'input-to-rgba.argb', 'output-getpixel.argb', 'output-to-rgba.argb'}
WIN_NAMES = {'input-requested.argb', 'input-getpixel.argb', 'input-fastpixel.argb', 'output-getpixel.argb', 'output-fastpixel.argb'}

def require(condition, message):
    if not condition:
        raise ValueError(message)

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def object_unique(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, 'duplicate JSON key: ' + key)
        value[key] = item
    return value

def parse_json(raw, limit=65536):
    require(len(raw) <= limit, 'JSON exceeds size limit')
    return json.loads(raw.decode('utf-8-sig'), object_pairs_hook=object_unique,
                      parse_constant=lambda x: (_ for _ in ()).throw(ValueError('non-finite JSON number')))

def safe_name(name):
    require(isinstance(name, str) and re.fullmatch(r'[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*', name), 'unsafe path')
    require(all(p not in ('.', '..') for p in PurePosixPath(name).parts), 'path traversal')
    return name

def hash_value(value):
    require(isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value), 'invalid SHA256')
    return value

def cases(path):
    require(Path(path).stat().st_size <= 65536, 'case JSON exceeds size limit')
    raw = Path(path).read_bytes()
    doc = parse_json(raw)
    require(type(doc.get('schema')) is int and isinstance(doc.get('cases'), list), 'case schema types')
    for row in doc['cases']:
        require(all(type(row.get(k)) is int for k in ['width', 'height', 'maxWidth', 'maxHeight']), 'case dimension types')
        require(type(row.get('alpha')) is bool and type(row.get('scale')) in (int, float), 'case alpha/scale types')
    if doc['schema'] == 2:
        require(digest(raw) == '9ee68be40a334898970f38a08f37e3a1f6184ee90999774623b5cd8ae117cb41', 'schema2 case grid differs from pinned74 contract')
        require(doc['byteOrder'] == BYTE_ORDER and len(doc['cases']) == 74, 'schema2 case grid')
        return doc, digest(raw)
    expected = []
    for w, h in [(3, 5), (8, 4)]:
        for alpha in [False, True]:
            prefix = f'picture-{w}x{h}-' + ('alpha-' if alpha else 'opaque-')
            def add(suffix, operation, mw=0, mh=0, scale=0):
                expected.append(dict(id=prefix+suffix, operation=operation, width=w, height=h, alpha=alpha,
                                     maxWidth=mw, maxHeight=mh, scale=scale))
            add('identity', 'identity')
            for mw, mh in [(1, 1), (5, 3), (13, 11)]:
                add(f'fit-{mw}x{mh}', 'fit', mw, mh)
            for percent in [25, 50, 100, 150]:
                add('scale-'+str(percent), 'scale', scale=percent/100)
            add('grayscale', 'grayscale')
    require(doc == dict(schema=1, byteOrder=BYTE_ORDER, cases=expected), 'case grid differs from fixed36 contract')
    return doc, digest(raw)

def requested(case):
    if case.get('family', 'picture') != 'picture':
        return bytes(case['width']*case['height']*4)
    result = bytearray()
    for y in range(case['height']):
        for x in range(case['width']):
            a = [0, 1, 127, 128, 254, 255][(y*case['width']+x) % 6] if case['alpha'] else 255
            result.extend([a, (71*x+31*y+17)%256, (29*x+97*y+43)%256, (113*x+19*y+89)%256])
    return bytes(result)

class Files:
    """Snapshot bounded files without extraction or following links."""
    def __init__(self, path):
        path = Path(path)
        require(not path.is_symlink(), 'linked input root')
        self.raw = {}
        self.return_integrity = False
        seen = set()
        total = 0
        def add(name, size, read):
            nonlocal total
            safe_name(name)
            require(name.casefold() not in seen and len(seen) < 1000, 'duplicate path or file count limit')
            require(0 <= size <= 2097152, 'file exceeds2MiB bound')
            total += size
            require(total <= 67108864, 'input exceeds64MiB bound')
            seen.add(name.casefold())
            raw = read()
            require(len(raw) == size, 'file changed while reading')
            self.raw[name] = raw
        if path.is_dir():
            for entry, p in enumerate(path.rglob('*')):
                require(entry < 2000, 'directory entry count limit')
                require(not p.is_symlink(), 'linked entry')
                if p.is_dir():
                    continue
                require(stat.S_ISREG(p.stat().st_mode), 'nonregular file')
                add(p.relative_to(path).as_posix(), p.stat().st_size, p.read_bytes)
        else:
            require(path.is_file(), 'input does not exist')
            with zipfile.ZipFile(path) as archive:
                require(len(archive.infolist()) <= 1000, 'archive entry count')
                for info in archive.infolist():
                    require(not info.is_dir() and not info.flag_bits & 1, 'directory/encrypted archive entry')
                    require(stat.S_IFMT(info.external_attr >> 16) in (0, stat.S_IFREG), 'linked/nonregular archive entry')
                    add(info.filename, info.file_size, lambda i=info: archive.read(i))
            require('return-manifest.json' in self.raw, 'archive missing return manifest')
        if 'return-manifest.json' in self.raw:
            manifest = self.json('return-manifest.json', 2097152)
            require(manifest.get('schema') == 1, 'return manifest schema')
            records = manifest.get('files')
            require(isinstance(records, list) and len(records) <= 999, 'return manifest records')
            expected = set(self.raw)-{'return-manifest.json'}
            for record in records:
                name = safe_name(record['path'])
                require(name in expected, 'unexpected or duplicate return record')
                expected.remove(name)
                raw = self.raw[name]
                require(record['bytes'] == len(raw) and record['sha256'] == digest(raw), 'return file hash/length mismatch')
            require(not expected, 'unlisted return files')
            self.return_integrity = True
    def read(self, name):
        require(name in self.raw, 'missing file: '+name)
        return self.raw[name]
    def json(self, name, limit=65536):
        return parse_json(self.read(name), limit)

def drawing_defaults(value, schema):
    require(isinstance(value, dict), 'missing drawing settings')
    for field in ['smoothing', 'pixelOffset', 'interpolation', 'compositingMode', 'compositingQuality', 'pageUnit']:
        require(isinstance(value.get(field), str) and 0 < len(value[field]) < 100, 'missing drawing setting: '+field)
    for field in ['pageScale', 'dpiX', 'dpiY']:
        n = value.get(field)
        require(type(n) in (int, float) and math.isfinite(n) and n > 0, 'invalid drawing setting: '+field)
    for field, count in [('transform', 6)] + ([('clipBounds', 4), ('visibleClipBounds', 4)] if schema == 2 else []):
        array = value.get(field)
        require(isinstance(array, list) and len(array) == count and all(type(n) in (int, float) and math.isfinite(n) for n in array), 'invalid drawing setting: '+field)
    if schema == 2:
        require(type(value.get('clipIsInfinite')) is bool, 'missing clip state')

def validate_case(files, case, windows, case_hash=None, schema=1):
    prefix = 'cases/'+case['id']+'/'
    raw_manifest = files.read(prefix+'manifest.json')
    report = parse_json(raw_manifest)
    require(type(report.get('schema')) is int and report['schema'] == schema and report.get('status') == 'complete', 'incomplete case manifest')
    require(report.get('id') == case['id'] and report.get('input') == case and report.get('byteOrder') == BYTE_ORDER, 'case identity/schema mismatch')
    w, h = report.get('outputWidth'), report.get('outputHeight')
    require(type(w) is int and type(h) is int and 1 <= w <= 64 and 1 <= h <= 64, 'invalid output dimensions')
    names = WIN_NAMES if windows else MAC_NAMES
    records = report.get('files')
    require(isinstance(records, list) and len(records) == 5, 'expected five raw records')
    actual = {name[len(prefix):] for name in files.raw if name.startswith(prefix)}
    require(actual == names | {'manifest.json'}, 'unexpected/missing case files')
    pending = set(names)
    buffers = {}
    for record in records:
        name = record.get('path')
        require(name in pending, 'unexpected/duplicate raw file')
        pending.remove(name)
        raw = files.read(prefix+name)
        length = (case['width']*case['height'] if name.startswith('input-') else w*h)*4
        require(record.get('bytes') == length == len(raw) and record.get('sha256') == digest(raw), 'raw length/hash mismatch')
        buffers[name] = raw
    require(buffers['input-requested.argb'] == requested(case), 'requested bytes differ from exact synthetic input')
    require(report.get('inputUnchanged') is True, 'input mutation reported')
    other = 'fastpixel' if windows else 'to-rgba'
    for side in ['input', 'output']:
        equal = buffers[side+'-getpixel.argb'] == buffers[side+'-'+other+'.argb']
        require(report.get(side+'ReadbacksEqual') is equal, 'readback equality claim disagrees with bytes')
        require(equal, 'readback APIs disagree; case retained as invalid diagnostic evidence')
    if windows:
        require(report.get('inputStride') == case['width']*4 and report.get('outputStride') == w*4, 'unexpected Windows stride')
        require(report.get('inputPixelFormat') == report.get('outputPixelFormat') == 'Format32bppArgb', 'unexpected Windows pixel format')
        drawing_defaults(report.get('freshGraphicsDefaults'), schema)
        if schema == 2 and case['family'] != 'picture':
            drawing_defaults(report.get('actualDrawingDefaults'), schema)
            require(report.get('drawingStateScope') == 'actual-before-identity-transform-fixture', 'wrong actual drawing scope')
        elif schema == 2:
            require(report.get('actualDrawingDefaults') is None and report.get('drawingStateScope') == 'fresh-defaults-only-original-picture-body-uninstrumented', 'Picture internal drawing state is not instrumented')
    else:
        require(report.get('caseGridSha256') == case_hash, 'Mac case grid provenance mismatch')
        expected = 'CountDown.Canvas-post-mask' if case.get('family') == 'countdown' else 'Bitmap-straight-readback'
        if schema == 2:
            require(report.get('outputRepresentation') == expected, 'Mac output stage/representation mismatch')
    return report, buffers, digest(raw_manifest)

def difference(left, right, width, height):
    require(len(left) == len(right) == width*height*4, 'dimension mismatch in byte comparison')
    changed = []; channels = 0; maximum = 0; counts = dict.fromkeys('ARGB', 0)
    first = []
    for index in range(width*height):
        a, b = left[index*4:index*4+4], right[index*4:index*4+4]
        if a != b:
            x,y = index%width,index//width
            changed.append((x,y))
            if len(first) < 16:
                first.append(dict(x=x,y=y,left=list(a),right=list(b)))
            for name, av,bv in zip('ARGB',a,b):
                if av != bv:
                    channels += 1; counts[name] += 1; maximum=max(maximum,abs(av-bv))
    bounds = None if not changed else dict(minX=min(x for x,y in changed), minY=min(y for x,y in changed), maxX=max(x for x,y in changed), maxY=max(y for x,y in changed))
    return dict(exact=not changed,differingPixels=len(changed),differingChannels=channels,channels=counts,maxChannelDelta=maximum,mismatchBounds=bounds,firstDifferences=first)
