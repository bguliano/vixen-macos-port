#!/usr/bin/env python3
"""Exact separate decoded/cache ARGB comparison; source schedule is not original-effect execution."""
import argparse,hashlib,json
from pathlib import Path
def validate_frame(row,key,index):
    assert row[key]==index and row['width']==4 and row['height']==4
    assert len(row['argb'])==64 and all(type(b) is int and 0<=b<=255 for b in row['argb'])
def compare(contract,mac,windows):
    assert contract['schema']==1 and len(contract['cases'])==8 and mac['schema']==1 and len(mac['cases'])==8 and len(windows)==8
    results=[]
    for expected,m,w in zip(contract['cases'],mac['cases'],windows):
        assert w['schema']==1 and w['id']==expected['id'] and w['inputSha256']==expected['inputSha256']
        assert w['installedEffectExecuted'] is False and w['delaysCentiseconds']==[1,7,13,2]
        assert (m['file'],m['gifSpeed'])==(w['file'],w['gifSpeed'])==(expected['file'],expected['gifSpeed'])
        assert w['sourceSchedule']==expected['expectedSourceSchedule']
        record={'id':expected['id']}
        for family,key,indices in [('decodedFrames','sourceFrame',list(range(4))),('cache','effectFrame',[s['effectFrame'] for s in expected['expectedSourceSchedule']])]:
            assert len(m[family])==len(w[family])==len(indices)
            errors=[]
            for index,a,b in zip(indices,m[family],w[family]):
                validate_frame(a,key,index);validate_frame(b,key,index)
                for offset,(x,y) in enumerate(zip(a['argb'],b['argb'])):
                    if x!=y:errors.append({'frame':index,'pixel':offset//4,'channel':'ARGB'[offset%4],'mac':x,'windows':y})
            record[family]={'exact':not errors,'channelMismatches':len(errors),'mismatches':errors}
        results.append(record)
    return {'schema':1,'status':'exact-within-scope' if all(r[f]['exact'] for r in results for f in ('decodedFrames','cache')) else 'mismatch','cases':results,'installedEffectVerified':False,'scope':'Windows GDI+ decode/draw vs Mac production decode/cache; scheduling source-derived on Windows, not installed effect or GUI'}
def main():
    p=argparse.ArgumentParser();p.add_argument('--contract',type=Path,required=True);p.add_argument('--mac',type=Path,required=True);p.add_argument('--windows',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--pack-pins',type=Path,required=True);a=p.parse_args()
    c=json.loads(a.contract.read_text());run=json.loads((a.windows/'run.json').read_text(encoding='utf-8-sig'))
    assert run['status']=='captured-comparison-pending' and run['inputUnchanged'] is True and run['errors']==[]
    assert run['packPinsSha256']==hashlib.sha256(a.pack_pins.read_bytes()).hexdigest()
    pack=json.loads(a.pack_pins.read_text());assert next(x['sha256'] for x in pack['files'] if x['name']=='cases.json')==hashlib.sha256(a.contract.read_bytes()).hexdigest()
    assert [s['name'] for s in run['stages']]==['compile']+[x['id'] for x in c['cases']]
    assert len(run['stages'])==9 and all(s['exitCode']==0 and s['timedOut'] is False for s in run['stages'])
    w=[json.loads((a.windows/(x['id']+'.json')).read_text(encoding='utf-8-sig')) for x in c['cases']]
    report=compare(c,json.loads(a.mac.read_text()),w)
    report['inputSha256']={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [a.contract,a.mac,a.pack_pins,a.windows/'run.json']+[a.windows/(x['id']+'.json') for x in c['cases']]}
    with a.out.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
    print(json.dumps({'status':report['status'],'cases':len(report['cases'])}));raise SystemExit(0 if report['status']=='exact-within-scope' else 1)
if __name__=='__main__':main()
