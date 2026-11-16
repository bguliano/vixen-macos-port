#!/usr/bin/env python3
"""Validate a returned raster folder/ZIP, then compare exact ARGB bytes with Mac output."""
import argparse
import json
from pathlib import Path
import sys
from raster_picture_common import ROOT, BYTE_ORDER, Files, cases, digest, difference, drawing_defaults, hash_value, parse_json, require, validate_case


def compare(mac_path, windows_path, case_path, source_contract, *, synthetic_fixture=False):
    grid, grid_hash=cases(case_path)
    schema=grid['schema'];case_count=len(grid['cases'])
    mac=Files(mac_path);win=Files(windows_path)
    mac_summary=mac.json('capture-summary.json',2097152)
    require(mac_summary.get('schema')==schema and mac_summary.get('passed') is True and mac_summary.get('status')=='mac-render-complete-windows-comparison-pending','Mac capture incomplete')
    require(mac_summary.get('caseGridSha256')==grid_hash and mac_summary.get('caseCount')==case_count,'Mac case grid mismatch')
    hash_value(mac_summary.get('testExecutableSha256'))
    for name,value in mac_summary.get('sourceHashes',{}).items():hash_value(value)
    require(len(mac_summary.get('sourceHashes',{}))>=4,'missing Mac source provenance')
    mac_rows=mac_summary.get('cases',[])
    require(len(mac_rows)==case_count and [r['id'] for r in mac_rows]==[c['id'] for c in grid['cases']],'Mac summary case identity/order')
    summary=win.json('capture-summary.json',2097152)
    require(summary.get('schema')==schema and summary.get('version')=='raster-'+str(schema) and summary.get('expectedCases')==case_count,'Windows summary schema/count')
    outcomes=summary.get('cases')
    require(isinstance(outcomes,list) and len(outcomes)==case_count,'Windows summary missing case outcomes')
    require([r.get('id') for r in outcomes]==[c['id'] for c in grid['cases']],'Windows summary duplicate/unexpected/reordered cases')
    count=sum(row.get('status')=='captured' for row in outcomes)
    require(type(summary.get('completedCases')) is int and summary['completedCases']==count,'Windows completed count inconsistent')
    require(summary.get('validWindowsRasterCaptured') is (count>0),'Windows captured-state flag inconsistent')
    require(summary.get('comparisonPerformed') is False,'Windows capture cannot claim local comparison')
    stages=summary.get('stages')
    require(isinstance(stages,list) and len(stages)<=case_count+4,'missing/oversized stage records')
    allowed_stages={'integrity','compile-raster','preflight','integrity-final'}|{'case-'+c['id'] for c in grid['cases']}
    stage_map={}
    for stage in stages:
        require(isinstance(stage,dict) and stage.get('name') in allowed_stages and stage['name'] not in stage_map,'unexpected/duplicate stage')
        stage_map[stage['name']]=stage
    def successful_stage(name):
        require(name in stage_map,'missing stage: '+name)
        stage=stage_map[name]
        require(win.json(name+'.stage.json')==stage,'stage summary/file mismatch: '+name)
        require(stage.get('status')=='completed' and type(stage.get('exitCode')) is int and stage['exitCode']==0 and all(stage.get(k) is False for k in ['timedOut','outputLimit','overallExpired']),'stage did not succeed: '+name)
    for name in ['integrity','compile-raster','preflight']:successful_stage(name)
    capture_issues=[]
    try:successful_stage('integrity-final')
    except (ValueError,KeyError,TypeError) as error:capture_issues.append(str(error))
    preflight=win.json('preflight.json')
    require(not preflight.get('syntheticTestFixture') or synthetic_fixture, 'synthetic fixture is not Windows capture evidence')
    require(preflight.get('schema')==1 and preflight.get('status')=='available','Windows preflight unavailable')
    require('Windows' in preflight.get('os','') and str(preflight.get('clr','')).startswith('4.'),'expected Windows Framework runtime')
    require(preflight.get('drawingIdentity','').startswith('System.Drawing, Version=4.0.0.0,'),'expected Framework System.Drawing')
    for field in ['drawingPath','gdiplusPath','gdiplusVersion']:
        require(isinstance(preflight.get(field),str) and 0<len(preflight[field])<=2048,'missing runtime provenance: '+field)
    require(type(preflight.get('is64BitProcess')) is bool and type(preflight.get('is64BitOperatingSystem')) is bool,'runtime architecture metadata missing')
    for key in ['hostSha256','drawingSha256','gdiplusSha256']:hash_value(preflight.get(key))
    drawing_defaults(preflight.get('freshGraphicsDefaults'),schema)
    require(preflight['hostSha256']==digest(win.read('RasterCapture.exe')),'captured host hash differs from preflight')
    hash_value(summary.get('compilerSha256'))
    require(all(isinstance(summary.get(k),str) and 0<len(summary[k])<=2048 for k in ['compilerPath','compilerVersion']),'compiler provenance missing')
    require(Path(source_contract).stat().st_size<=65536,'source contract exceeds size limit')
    contract=parse_json(Path(source_contract).read_bytes())
    require(contract.get('schema')==schema,'source contract schema')
    methods=hash_value(contract['generated']['OriginalPictureMethods.cs.in'])
    fast=hash_value(contract['generated']['OriginalFastPixel.cs.in'])
    allowed_ids={case['id'] for case in grid['cases']}
    for name in win.raw:
        if name.startswith('cases/'):
            parts=name.split('/')
            require(len(parts)==3 and parts[1] in allowed_ids,'unknown/nested returned case directory')
    rows=[]
    for case,outcome,mac_row in zip(grid['cases'],outcomes,mac_rows):
        row=dict(id=case['id'],family=case.get('family','picture'),alpha=case['alpha'],operation=case['operation'])
        try:
            require(outcome.get('status')=='captured','Windows case '+str(outcome.get('status'))+': '+str(outcome.get('error',''))[:200])
            successful_stage('case-'+case['id'])
            mr,mb,mhash=validate_case(mac,case,False,grid_hash,schema)
            wr,wb,whash=validate_case(win,case,True,schema=schema)
            require(mhash==mac_row.get('manifestSha256') and whash==outcome.get('manifestSha256'),'summary case manifest hash mismatch')
            require(wr.get('hostSha256')==preflight['hostSha256'] and wr.get('originalMethodsSha256')==methods and wr.get('fastPixelSha256')==fast,'Windows host/source contract mismatch')
            if schema==2:
                require(wr.get('countDownMethodsSha256')==contract['generated']['OriginalCountDownMethods.cs.in'],'Windows CountDown source contract mismatch')
            same_dimensions=(mr['outputWidth'],mr['outputHeight'])==(wr['outputWidth'],wr['outputHeight'])
            comparisons={}
            for api in ['getpixel','fastpixel']:
                comparisons['input-'+api]=difference(wb['input-'+api+'.argb'],mb['input-getpixel.argb'],case['width'],case['height'])
                if same_dimensions:
                    comparisons['output-'+api]=difference(wb['output-'+api+'.argb'],mb['output-getpixel.argb'],wr['outputWidth'],wr['outputHeight'])
            # Baseline loss stays visible; an equal final bitmap cannot hide it.
            exact=same_dimensions and all(value['exact'] for value in comparisons.values())
            row.update(status='exact' if exact else 'different',dimensionsEqual=same_dimensions,
                       outputComparison='Windows raw GDI vs actual CountDown Canvas post-mask' if case.get('family')=='countdown' else 'Windows raw GDI vs actual Rust Bitmap readback',
                       windowsDimensions=[wr['outputWidth'],wr['outputHeight']],macDimensions=[mr['outputWidth'],mr['outputHeight']],comparisons=comparisons,
                       windowsRequestedVsObserved=difference(wb['input-requested.argb'],wb['input-getpixel.argb'],case['width'],case['height']),
                       macRequestedVsObserved=difference(mb['input-requested.argb'],mb['input-getpixel.argb'],case['width'],case['height']))
        except (ValueError,KeyError,TypeError) as error:
            row.update(status='invalid-or-incomplete',error=str(error)[:500])
        rows.append(row)
    complete=not capture_issues and summary.get('status')=='captured-comparison-pending' and count==case_count and all(r['status']!='invalid-or-incomplete' for r in rows)
    exact=complete and all(r['status']=='exact' for r in rows)
    groups={}
    for key,alpha in [('opaque',False),('alpha',True)]:
        selected=[r for r in rows if r['family']=='picture' and r['alpha']==alpha]
        groups[key]=dict(cases=len(selected),exactCases=sum(r['status']=='exact' for r in selected),differentCases=sum(r['status']=='different' for r in selected),invalidCases=sum(r['status']=='invalid-or-incomplete' for r in selected))
    families={family:dict(cases=sum(r['family']==family for r in rows),exactCases=sum(r['family']==family and r['status']=='exact' for r in rows),differentCases=sum(r['family']==family and r['status']=='different' for r in rows),invalidCases=sum(r['family']==family and r['status']=='invalid-or-incomplete' for r in rows)) for family in sorted({r['family'] for r in rows})}
    return dict(schema=schema,passed=exact,windowsParityVerified=exact and not synthetic_fixture,
                status='exact' if exact else 'different' if complete else 'partial-or-invalid',
                validCompleteCapture=complete,caseGridSha256=grid_hash,byteOrder=BYTE_ORDER,
                captureIssues=capture_issues,
                sourceContractSha256=digest(Path(source_contract).read_bytes()),windowsReturnIntegrityVerified=win.return_integrity,
                syntheticConsumerTest=synthetic_fixture,caseCount=case_count,groups=groups,families=families,cases=rows,
                runtime=preflight,macTestExecutableSha256=mac_summary['testExecutableSha256'],macSourceHashes=mac_summary['sourceHashes'],
                scope='Exact straight ARGB8 from Windows GetPixel and original FastPixel versus actual Rust readback. No tolerance/normalization; input differences count. Schema2 CountDown is raw GDI versus actual Canvas post-mask, separately labelled; primitives are direct APIs. Not complete effects, installed Vixen binaries, fonts, encoded images or minimum OS.')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--mac',type=Path,required=True);p.add_argument('--windows',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    p.add_argument('--cases',type=Path,default=ROOT/'tools/mac-raster/picture36/cases.json');p.add_argument('--source-contract',type=Path,default=ROOT/'tools/mac-raster/picture36/source-contract.json')
    a=p.parse_args();require(not a.out.exists(),'fresh report path required')
    try:
        report=compare(a.mac,a.windows,a.cases,a.source_contract)
    except Exception as error:
        report=dict(passed=False,windowsParityVerified=False,status='invalid-evidence',error=str(error)[:1000])
    a.out.parent.mkdir(parents=True,exist_ok=True)
    with a.out.open('x') as stream:json.dump(report,stream,indent=2);stream.write('\n')
    print(json.dumps({k:v for k,v in report.items() if k in ['passed','status','windowsParityVerified','groups','error']}))
    return 0 if report['passed'] else 1 if report['status']=='different' else 2
if __name__=='__main__':sys.exit(main())
