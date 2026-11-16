#!/usr/bin/env python3
"""Independent fixed 74-case schema2 manifest; no image rendering."""
import argparse,json
from pathlib import Path
HERE=Path(__file__).resolve().parent
def cases():
    all=[]
    def add(**kwargs):
        value=dict(id=None,family=None,operation=None,fillMode=None,width=0,height=0,maxWidth=0,maxHeight=0,alpha=False,scale=0,rect=None,line=None,points=None,angle=0,lineLength=0)
        value.update(kwargs);all.append(value)
    for width,height in [(3,5),(8,4)]:
        for alpha in [False,True]:
            prefix=f'picture-{width}x{height}-'+('alpha-' if alpha else 'opaque-')
            base=dict(family='picture',width=width,height=height,alpha=alpha)
            add(id=prefix+'identity',operation='identity',**base)
            for w,h in [(1,1),(5,3),(13,11)]:add(id=prefix+f'fit-{w}x{h}',operation='fit',maxWidth=w,maxHeight=h,**base)
            for percent in [25,50,100,150]:add(id=prefix+f'scale-{percent}',operation='scale',scale=percent/100,**base)
            add(id=prefix+'grayscale',operation='grayscale',**base)
    for i,rect in enumerate([(0,0,4,4),(2,3,9,9),(1,1,20,12),(-2,2,9,9)],1):
        add(id=f'countdown-cross-{i}',family='countdown',operation='cross-hash',width=32,height=32,alpha=True,rect=rect)
    for i,(rect,length) in enumerate([((2,3,9,9),8),((12,10,12,12),18)],1):
        for angle in [0,44,45,46,90,179,180,359]:add(id=f'countdown-spinner-{i}-angle-{angle}',family='countdown',operation='spinner',width=32,height=32,alpha=True,rect=rect,lineLength=length,angle=angle)
    lines=[(20,20,29,24),(20,20,24,29),(20,20,16,29),(20,20,11,24),(20,20,11,16),(20,20,16,11),(20,20,24,11),(20,20,29,16),(2,2,2,2),(1,3,10,3),(3,1,3,10)]
    for i,line in enumerate(lines,1):add(id=f'primitive-line-{i}',family='primitive',operation='line',width=40,height=40,alpha=True,line=line)
    for i,rect in enumerate([(2,3,4,5),(-2,1,5,3),(1.5,2.5,3.5,4.5)],1):add(id=f'primitive-fill-rectangle-{i}',family='primitive',operation='fill-rectangle',width=40,height=40,alpha=True,rect=rect)
    for operation,rect in [('rectangle',(1,1,4,3)),('ellipse',(5,5,20,12))]:add(id='primitive-'+operation,family='primitive',operation=operation,width=40,height=40,alpha=True,rect=rect)
    for mode in ['Alternate','Winding']:add(id='primitive-polygon-'+mode.lower(),family='primitive',operation='polygon',width=40,height=40,alpha=True,fillMode=mode,points=[20,2,31,35,3,14,37,14,9,35])
    assert len(all)==74
    return dict(schema=2,byteOrder='ARGB8-row-major-top-down',cases=all)
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();raw=json.dumps(cases(),indent=2)+'\n'
    if a.check:assert (HERE/'cases.json').read_text()==raw,'case manifest drift'
    else:(HERE/'cases.json').write_text(raw)
    print(json.dumps(dict(passed=True,cases=74,checked=a.check)))
if __name__=='__main__':main()
