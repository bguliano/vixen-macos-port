#!/usr/bin/env python3
"""Independent fixed 80-case schema3 manifest; no image rendering."""
import argparse,json
from pathlib import Path
HERE=Path(__file__).resolve().parent
def cases():
    all=[]
    def add(**kwargs):
        value=dict(id=None,family=None,operation=None,fillMode=None,width=0,height=0,maxWidth=0,maxHeight=0,alpha=False,scale=0,rect=None,line=None,points=None,angle=0,lineLength=0,constantAlpha=0,swatches=None,inputPattern=None)
        value.update(kwargs);all.append(value)
    for width in [3,4,7,8,9]:
        for target in [6,7,8,9,12,13]:
            for alpha in [False,True]:
                add(id=f'picture-resize-{width}x4-to-{target}x6-'+('alpha' if alpha else 'opaque'),family='picture',operation='resize-exact',width=width,height=4,maxWidth=target,maxHeight=6,alpha=alpha)
    for alpha in [1,127,128,254]:
        add(id=f'picture-gray-alpha-{alpha}',family='picture',operation='grayscale',width=16,height=3,alpha=True,constantAlpha=alpha,inputPattern='gray-swatches-alpha-rows-0-selected-255',swatches=[0, 0, 0, 128, 128, 128, 255, 255, 255, 1, 129, 127, 2, 124, 1, 2, 136, 255, 4, 4, 254, 5, 5, 255, 6, 244, 127, 6, 250, 254, 7, 239, 1, 7, 245, 128, 7, 251, 255, 9, 107, 0, 9, 113, 127, 10, 108, 1])
    for i,rect in enumerate([(0,0,4,4),(2,3,9,9),(1,1,20,12),(5,5,20,12),(12,10,12,12),(-2,2,9,9),(3,4,9,5),(5.5,5.5,20,12)],1):
        add(id=f'detail-ellipse-{i}',family='primitive',operation='ellipse',width=40,height=40,alpha=True,rect=rect)
    for i,line in enumerate([(7.929,6.757,5.786,8.665),(22.071,6.757,24.214,8.665),(2.25,2.25,9.75,5.75),(2.5,2.5,6.5,10.5)],1):
        for reverse in [False,True]:
            add(id=f'detail-line-{i}-'+('reverse' if reverse else 'forward'),family='primitive',operation='line',width=40,height=40,alpha=True,line=line[2:]+line[:2] if reverse else line)
    assert len(all)==80
    return dict(schema=3,byteOrder='ARGB8-row-major-top-down',cases=all)
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();raw=json.dumps(cases(),indent=2)+'\n'
    if a.check:assert (HERE/'cases.json').read_text()==raw,'case manifest drift'
    else:(HERE/'cases.json').write_text(raw)
    print(json.dumps(dict(passed=True,cases=80,checked=a.check)))
if __name__=='__main__':main()
