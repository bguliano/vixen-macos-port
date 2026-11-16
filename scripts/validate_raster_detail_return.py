#!/usr/bin/env python3
"""Read-only validation of schema3 raster detail evidence; never claims Mac parity."""
import argparse
import importlib.util
import json
import math
from pathlib import Path
import struct
import sys
from raster_picture_common import ROOT, BYTE_ORDER, Files, WIN_NAMES, digest, difference, drawing_defaults, hash_value, parse_json, require

SOURCE = ROOT/'tools/windows-raster-detail'
LABELS = ['original', 'flat025', 'flat050', 'flat100']
FLATNESS = [0, .25, .5, 1]


def grid():
    spec = importlib.util.spec_from_file_location('raster_detail_grid', SOURCE/'generate_cases.py')
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    expected = module.cases()
    raw = (SOURCE/'cases.json').read_bytes(); actual = parse_json(raw)
    require(actual == json.loads(json.dumps(expected)), 'detail grid drift')
    return actual, digest(raw)


def requested(case):
    if case['family'] != 'picture':
        return bytes(case['width']*case['height']*4)
    result = bytearray()
    for y in range(case['height']):
        for x in range(case['width']):
            if case['inputPattern'] == 'gray-swatches-alpha-rows-0-selected-255':
                result.extend(([0, case['constantAlpha'], 255][y], *case['swatches'][3*x:3*x+3]))
            else:
                alpha = [0,1,127,128,254,255][(y*case['width']+x)%6] if case['alpha'] else 255
                result.extend((alpha, (71*x+31*y+17)%256, (29*x+97*y+43)%256, (113*x+19*y+89)%256))
    return bytes(result)



def actual_defaults(value, width, height):
    drawing_defaults(value, 2)
    expected = dict(smoothing='None',pixelOffset='Default',interpolation='Bilinear',compositingMode='SourceOver',compositingQuality='Default',pageUnit='Display',pageScale=1,transform=[1,0,0,1,0,0],clipIsInfinite=True,visibleClipBounds=[0,0,width,height])
    require(all(value.get(key) == wanted for key,wanted in expected.items()), 'nondefault actual drawing state')


def validate_case(files, case, host, contract):
    prefix = 'cases/'+case['id']+'/'
    report = files.json(prefix+'manifest.json')
    require(report.get('schema') == 3 and report.get('status') == 'complete', 'incomplete case manifest')
    require(report.get('id') == case['id'] and report.get('input') == case and report.get('byteOrder') == BYTE_ORDER, 'case identity mismatch')
    width, height = (case['maxWidth'], case['maxHeight']) if case['operation'] == 'resize-exact' else (case['width'], case['height'])
    require(type(report.get('outputWidth')) is int and type(report.get('outputHeight')) is int and (report['outputWidth'], report['outputHeight']) == (width, height), 'exact output dimensions')
    require(report.get('inputStride') == case['width']*4 and report.get('outputStride') == width*4, 'stride')
    require(report.get('inputPixelFormat') == report.get('outputPixelFormat') == 'Format32bppArgb', 'pixel format')
    require(report.get('hostSha256') == host, 'host provenance')
    for field, source in [('originalMethodsSha256','OriginalPictureMethods.cs.in'), ('countDownMethodsSha256','OriginalCountDownMethods.cs.in'), ('fastPixelSha256','OriginalFastPixel.cs.in')]:
        require(report.get(field) == contract['generated'][source], 'source provenance: '+field)
    actual_defaults(report.get('freshGraphicsDefaults'), width, height)
    if case['operation'] == 'grayscale':
        require(report.get('actualDrawingDefaults') is None and report.get('drawingStateScope') == 'fresh-defaults-only-original-picture-body-uninstrumented', 'grayscale scope')
    else:
        actual_defaults(report.get('actualDrawingDefaults'), width, height)
        scope = 'actual-before-original-picture-drawimage-overload' if case['operation'] == 'resize-exact' else 'actual-before-identity-transform-fixture'
        require(report.get('drawingStateScope') == scope, 'actual drawing scope')
    names = set(WIN_NAMES)
    if case['operation'] == 'ellipse':
        names |= {'path-trace.json'} | {'path-'+label+'-'+api+'.argb' for label in LABELS for api in ['getpixel','fastpixel']}
    require({name[len(prefix):] for name in files.raw if name.startswith(prefix)} == names | {'manifest.json'}, 'exact case file set')
    records = report.get('files'); require(isinstance(records, list) and len(records) == len(names), 'raw record count')
    buffers = {}
    for record in records:
        name = record.get('path'); require(name in names, 'unexpected/duplicate case file'); names.remove(name)
        raw = files.read(prefix+name)
        expected_length = len(raw) if name == 'path-trace.json' else (case['width']*case['height'] if name.startswith('input-') else width*height)*4
        require(type(record.get('bytes')) is int and record['bytes'] == expected_length == len(raw) and record.get('sha256') == digest(raw), 'raw length/hash')
        buffers[name] = raw
    require(buffers['input-requested.argb'] == requested(case), 'requested exact synthetic bytes')
    require(report.get('inputUnchanged') is True, 'reported source mutation')
    for side in ['input','output']:
        require(report.get(side+'ReadbacksEqual') is True and buffers[side+'-getpixel.argb'] == buffers[side+'-fastpixel.argb'], 'readback APIs differ')
    paths = []
    if case['operation'] == 'ellipse':
        trace = parse_json(buffers['path-trace.json'])
        require(trace.get('schema') == 3 and trace.get('id') == case['id'] and isinstance(trace.get('paths'), list) and len(trace['paths']) == 4, 'trace identity/count')
        for item, label, flatness in zip(trace['paths'], LABELS, FLATNESS):
            require(item.get('label') == label and item.get('flatness') == flatness, 'trace label/flatness')
            points, types, bits = item.get('points'), item.get('types'), item.get('pointBits')
            require(isinstance(types,list) and 4 <= len(types) <= 2048 and isinstance(points,list) and len(points) == 2*len(types) and isinstance(bits,list) and len(bits) == len(points), 'trace lengths')
            require(all(type(x) is int and x & ~(128|7) == 0 and x & 7 in (0,1,3) for x in types), 'native path types')
            require(types[0] == 0 and all(x & 7 in (1,3) for x in types[1:]) and all(x & 128 == 0 for x in types[:-1]) and types[-1] & 128 != 0, 'ellipse single closed contour')
            if label != 'original':
                require(all(x & 7 == 1 for x in types[1:]), 'flattened path contains curves')
            else:
                curves = 0
                for kind in types[1:]:
                    if kind & 7 == 3: curves += 1
                    else:
                        require(curves % 3 == 0, 'incomplete Bezier triple'); curves = 0
                require(curves % 3 == 0, 'incomplete Bezier triple')
            for value, bit in zip(points,bits):
                require(type(value) in (float,int) and math.isfinite(value) and type(bit) is int and -2147483648 <= bit <= 2147483647 and struct.unpack('<i',struct.pack('<f',value))[0] == bit, 'path float32 bits')
            actual_defaults(item.get('drawingDefaults'),width,height)
            require(item.get('penColor') == 'FFFFFFFF' and item.get('penWidth') == 1 and item.get('penMiterLimit') == 10 and item.get('penStartCap') == item.get('penEndCap') == 'Flat' and item.get('penLineJoin') == 'Miter' and item.get('penAlignment') == 'Center' and item.get('fillMode') == 'Alternate', 'exact native path pen/fill state')
            pixel = buffers['path-'+label+'-getpixel.argb']
            require(pixel == buffers['path-'+label+'-fastpixel.argb'], 'path readback APIs differ')
            paths.append(dict(label=label,pointCount=len(types),drawEllipseVsDrawPath=difference(buffers['output-getpixel.argb'],pixel,width,height)))
    return dict(id=case['id'],status='validated',manifestSha256=digest(files.read(prefix+'manifest.json')),outputDimensions=[width,height],requestedVsObserved=difference(buffers['input-requested.argb'],buffers['input-getpixel.argb'],case['width'],case['height']),paths=paths)


def validate(files, *, synthetic_fixture=False):
    case_grid, grid_hash = grid(); cases = case_grid['cases']
    require(files.return_integrity, 'missing verified return manifest')
    summary = files.json('capture-summary.json',2097152)
    require(summary.get('schema') == 3 and summary.get('version') == 'raster-detail-3' and summary.get('expectedCases') == 80, 'summary schema/grid')
    outcomes = summary.get('cases'); require(isinstance(outcomes,list) and [c.get('id') for c in outcomes] == [c['id'] for c in cases], 'outcome identity/order')
    count = sum(c.get('status') == 'captured' for c in outcomes)
    require(type(summary.get('completedCases')) is int and summary['completedCases'] == count and summary.get('validWindowsRasterCaptured') is (count > 0) and summary.get('comparisonPerformed') is False, 'capture count/claims')
    stages = summary.get('stages'); require(isinstance(stages,list) and len(stages) <= 84, 'stage records')
    allowed = {'integrity','compile-raster','preflight','integrity-final'} | {'case-'+c['id'] for c in cases}; stage_map = {}
    for stage in stages:
        name = stage.get('name'); require(name in allowed and name not in stage_map, 'unexpected/duplicate stage'); stage_map[name] = stage
        require(files.json(name+'.stage.json') == stage, 'stage file mismatch')
    def succeeded(name):
        stage = stage_map.get(name,{})
        require(stage.get('status') == 'completed' and type(stage.get('exitCode')) is int and stage['exitCode'] == 0 and all(stage.get(k) is False for k in ['timedOut','outputLimit','overallExpired']), 'unsuccessful stage: '+name)
    for name in ['integrity','compile-raster','preflight']: succeeded(name)
    runtime = files.json('preflight.json')
    require(not runtime.get('syntheticTestFixture') or synthetic_fixture, 'synthetic fixture is not actual Windows evidence')
    require(runtime.get('schema') == 1 and runtime.get('status') == 'available' and 'Windows' in runtime.get('os','') and runtime.get('clr','').startswith('4.') and runtime.get('drawingIdentity','').startswith('System.Drawing, Version=4.0.0.0,'), 'Windows Framework runtime')
    for key in ['hostSha256','drawingSha256','gdiplusSha256']: hash_value(runtime.get(key))
    for key in ['drawingPath','gdiplusPath','gdiplusVersion']:
        require(isinstance(runtime.get(key),str) and 0 < len(runtime[key]) <= 2048, 'runtime provenance')
    require(type(runtime.get('is64BitProcess')) is bool and type(runtime.get('is64BitOperatingSystem')) is bool, 'architecture')
    actual_defaults(runtime.get('freshGraphicsDefaults'),8,8)
    require(runtime['hostSha256'] == digest(files.read('RasterCapture.exe')), 'host hash')
    hash_value(summary.get('compilerSha256'))
    for key in ['compilerPath','compilerVersion']: require(isinstance(summary.get(key),str) and 0 < len(summary[key]) <= 2048, 'compiler provenance')
    contract = parse_json((SOURCE/'source-contract.json').read_bytes()); require(contract.get('schema') == 2, 'extracted source contract')
    ids = {c['id'] for c in cases}
    for name in files.raw:
        if name.startswith('cases/'):
            parts = name.split('/'); require(len(parts) == 3 and parts[1] in ids, 'unknown/nested case')
    rows = []; issues = []
    try: succeeded('integrity-final')
    except ValueError as error: issues.append(str(error))
    for case,outcome in zip(cases,outcomes):
        try:
            require(outcome.get('status') == 'captured', 'uncaptured: '+str(outcome.get('status')))
            succeeded('case-'+case['id']); row = validate_case(files,case,runtime['hostSha256'],contract)
            require(row['manifestSha256'] == outcome.get('manifestSha256'), 'outcome manifest hash')
        except (ValueError, KeyError, TypeError, OverflowError) as error:
            row = dict(id=case['id'],status='invalid-or-incomplete',error=str(error)[:500])
        rows.append(row)
    complete = not issues and summary.get('status') == 'captured-comparison-pending' and count == 80 and all(r['status'] == 'validated' for r in rows)
    return dict(schema=3,passed=complete,status='validated-capture-analysis-pending' if complete else 'partial-or-invalid',validCompleteWindowsCapture=complete and not synthetic_fixture,windowsParityVerified=False,syntheticConsumerTest=synthetic_fixture,caseGridSha256=grid_hash,returnIntegrityVerified=True,cases=rows,captureIssues=issues,runtime=runtime,scope='Exact detail evidence validation only. No Mac implementation or numerical-model comparison performed.')


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--windows',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();require(not a.out.exists(),'fresh report required')
    try: report=validate(Files(a.windows))
    except Exception as error: report=dict(passed=False,status='invalid-evidence',validCompleteWindowsCapture=False,windowsParityVerified=False,error=str(error)[:1000])
    a.out.parent.mkdir(parents=True,exist_ok=True)
    with a.out.open('x') as stream: json.dump(report,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ['cases','runtime']}));return 0 if report['passed'] else 2

if __name__ == '__main__': sys.exit(main())
