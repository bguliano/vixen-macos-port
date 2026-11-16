"""Exact eight-case storage experiment contract; no output raster expectations."""
SEEDS=[[0,17,43,89],[1,88,72,202],[127,159,101,59],[128,48,140,108],[254,177,177,177],[255,190,198,78]]
IDS=['storage-01-owner-only','storage-02-partial-opaque','storage-03-partial-alpha0','storage-04-partial-alpha1','storage-05-partial-image','storage-06-clear','storage-07-antialias','storage-08-drawn-source']
def cases():
 out=[]
 for index,id in enumerate(IDS):
  events=[];obs=[]
  def read(label,w=6,h=3,apis=('getpixel','lockbits-readonly')):
   for api in apis:events.append('read:'+label+':'+api);obs.append(dict(label=label,api=api,width=w,height=h))
  def op(label):events.append('operation:'+label)
  if index==7:
   read('gray-input-before',3,5);op('original-grayscale-a');read('a-before-draw',3,5,('getpixel',));op('draw-a-1to1-integer-overload')
   op('original-grayscale-b');read('b-before-copy',3,5,('getpixel',));op('reconstruct-c-setpixel');read('c-before-draw',3,5,('getpixel',));op('draw-c-1to1-integer-overload')
   for label in ['a-after-draw','b-after-draw','c-after-draw','drawn-a','drawn-c','gray-input-after']:read(label,3,5)
  else:
   read('seed-before')
   if index==0:op('graphics-owner-only');read('after-owner')
   elif index in [1,2,3]:op(['fill-opaque','fill-alpha0','fill-alpha1'][index-1]);read('after-fill')
   elif index==4:read('image-source-before',6,1);op('draw-image-explicit-pixel-rectangles');read('after-image');read('image-source-after',6,1)
   elif index==5:op('clear-alpha254');read('after-clear254');op('clear-alpha0');read('after-clear0')
   elif index==6:op('antialias-fill');read('after-antialias');read('coverage-before');op('antialias-white-coverage');read('coverage-after')
  out.append(dict(id=id,width=3 if index==7 else 6,height=5 if index==7 else 3,events=events,observations=obs))
 return dict(schema=1,byteOrder='ARGB8-row-major-top-down',cases=out)
def requested(case):
 if case['width']==6:return bytes(sum(SEEDS,[])*3)
 return bytes(v for y in range(5) for x in range(3) for v in ([0,1,127,128,254,255][(y*3+x)%6],(71*x+31*y+17)%256,(29*x+97*y+43)%256,(113*x+19*y+89)%256))
if __name__=='__main__':
 import json
 from pathlib import Path
 Path(__file__).with_name('cases.json').write_text(json.dumps(cases(),indent=2)+'\n')
