#!/usr/bin/env python3
# Generates the shared case list for comparing the Rust Morph with the real C# code.
import json, random, sys
R = random.Random(20261108)
cases = []
def case(id, strings=10, pixels=20, ms=1000, settings=None, shapes=None, **kw):
    c = dict(id=id, strings=strings, pixels=pixels, ms=ms, settings=settings or {}, shapes=shapes or [])
    c.update(kw)
    cases.append(c)

def rect(x0,y0,x1,y1): return [[x0,y0],[x1,y0],[x1,y1],[x0,y1]]
def shape(points, kind="Polygon", **kw):
    d = dict(kind=kind, points=points)
    d.update(kw)
    return d

# ---- the default shape on many element sizes
for strings, pixels in [(10,20),(20,10),(1,12),(12,1),(8,25),(3,3),(2,2),(1,1),(2,1),(1,2),(30,7)]:
    for ms in [1000, 2000, 333, 4950, 50, 49]:
        case(f"default-{strings}x{pixels}-{ms}", strings, pixels, ms)
    case(f"default-v-{strings}x{pixels}", strings, pixels, 1500, orientation="Vertical")

# ---- wipe settings one at a time
for v in [0,1,2,3,4,7,10,25,100]:
    case(f"headlength-{v}", settings=dict(HeadLength=v))
for v in [0,1,2,5,10,20,33,50,75,100]:
    case(f"headduration-{v}", settings=dict(HeadDuration=v))
for v in [-10,-7,-3,-1,1,3,7,10]:
    case(f"acceleration-{v}", settings=dict(Acceleration=v), ms=2000)
    case(f"acceleration-short-{v}", strings=4, pixels=4, ms=500, settings=dict(Acceleration=v))
    for d in [5,50,100]:
        case(f"acceleration-{v}-duration-{d}", settings=dict(Acceleration=v, HeadDuration=d), ms=3000)
# ---- gradients and brightness
case("colors", settings=dict(), head=[[0.0,[0,255,0]],[1.0,[0,0,255]]], tail=[[0.0,[255,128,0]],[0.5,[9,200,77]],[1.0,[128,0,255]]],
     headBrightness=[[0,100],[100,10]], tailBrightness=[[0,0],[50,100],[100,20]], ms=2000)

# ---- patterns
for direction in ["Up","Down","Left","Right"]:
    for count, skip, stagger in [(1,3,0),(2,4,10),(3,2,25),(5,1,100),(4,0,20),(2,30,50)]:
        case(f"repeat-{direction}-{count}-{skip}-{stagger}", settings=dict(RepeatDirection=direction, RepeatCount=count, RepeatSkip=skip, Stagger=stagger),
             shapes=[shape(rect(2,6,12,9))], ms=2000)
case("repeat-accel", settings=dict(RepeatCount=3,RepeatSkip=2,Stagger=10,Acceleration=5), shapes=[shape(rect(2,6,12,9))], ms=2000)
case("repeat-decel", settings=dict(RepeatCount=3,RepeatSkip=2,Stagger=10,Acceleration=-5), shapes=[shape(rect(2,6,12,9))], ms=2000)
case("repeat-stagger-overrun", settings=dict(RepeatCount=5,RepeatSkip=1,Stagger=100), shapes=[shape(rect(0,0,19,3))], ms=1000)
case("pattern-line", settings=dict(RepeatCount=2,RepeatSkip=2,Stagger=10), shapes=[shape([[0,0],[19,9]],"Line")], ms=1000)
case("pattern-triangle", settings=dict(RepeatCount=1,RepeatSkip=3), shapes=[shape([[0,0],[19,0],[10,9]])], ms=1000)
case("pattern-5-points", shapes=[shape([[0,0],[19,0],[19,9],[10,5],[0,9]])], ms=500)
case("pattern-start-offset", settings=dict(Stagger=30), shapes=[shape(rect(0,0,19,9),start=25)], ms=1000)
case("pattern-with-repeat-and-start-offset", settings=dict(Stagger=30,RepeatCount=2,RepeatSkip=1), shapes=[shape(rect(0,0,19,9),start=25)], ms=1000)

# ---- free form
def random_point(w, h, slop=0.25):
    x = R.uniform(-slop*w, (1+slop)*w); y = R.uniform(-slop*h, (1+slop)*h)
    if R.random() < 0.5: x = round(x)
    if R.random() < 0.5: y = round(y)
    if R.random() < 0.2: x = round(x) + 0.5
    if R.random() < 0.2: y = round(y) + 0.5
    return [x, y]
for i in range(150):
    strings = R.choice([5,8,10,16,25]); pixels = R.choice([5,9,20,32,50])
    n = R.choice([1,1,2,3])
    shapes = []
    for _ in range(n):
        kind = R.choice(["Polygon","Polygon","Triangle","Line"])
        pts = {"Polygon":4,"Triangle":3,"Line":2}[kind]
        s = shape([random_point(pixels,strings) for _ in range(pts)], "Line" if kind=="Line" else "Polygon",
                  start=R.choice([0,0,10,25,60,100]), headLength=R.choice([0,1,4,9]), headDuration=R.choice([0,5,20,50,100]),
                  acceleration=R.choice([0,0,0,-10,-3,4,10]))
        shapes.append(s)
    case(f"freeform-{i}", strings, pixels, R.choice([500,1000,1950,3000]), settings=dict(PolygonType="FreeForm"), shapes=shapes)
for i in range(60):
    strings = R.choice([5,8,10,16]); pixels = R.choice([5,9,20,32])
    pts = [[R.randint(0,pixels-1),R.randint(0,strings-1)] for _ in range(4)]
    case(f"freeform-inside-{i}", strings, pixels, 1000, settings=dict(PolygonType="FreeForm", Margin=R.choice([0,0,10,50])),
         shapes=[shape(pts, headLength=R.choice([1,4,6]), headDuration=R.choice([10,20,60]))])

# ---- margins
for margin in [5,10,25,37,50,100]:
    case(f"margin-{margin}-default", settings=dict(Margin=margin))
    case(f"margin-{margin}-shape", settings=dict(Margin=margin,PolygonType="FreeForm"), shapes=[shape(rect(1,2,25,8))])

# ---- scaling: shapes saved on another size
for w, h in [(10,10),(20,10),(5,40),(1,1),(1,10),(10,1),(100,100)]:
    case(f"scale-{w}x{h}", settings=dict(PolygonType="FreeForm",DisplayElementWidth=w,DisplayElementHeight=h), shapes=[shape(rect(0,0,w-1,h-1))], strings=10, pixels=20)
    case(f"scale-margin-{w}x{h}", settings=dict(PolygonType="FreeForm",DisplayElementWidth=w,DisplayElementHeight=h,Margin=10), shapes=[shape(rect(1,1,w,h))], strings=10, pixels=20)

# ---- locations
for margin in [0,10,50]:
    case(f"location-default-{margin}", strings=4, pixels=10, located=True, settings=dict(Margin=margin))
    case(f"location-shape-{margin}", strings=4, pixels=10, located=True, settings=dict(Margin=margin,PolygonType="FreeForm"), shapes=[shape(rect(0,0,36,12)), shape([[0,6],[36,6]],"Line",start=20)])
    case(f"location-repeat-{margin}", strings=6, pixels=10, located=True, settings=dict(Margin=margin,RepeatCount=2,RepeatSkip=5,RepeatDirection="Right",Stagger=10), shapes=[shape(rect(0,0,10,12))])

# ---- static shapes, through the fake GDI: everything around the raster
BLUE = [[0.0,[0,0,255]]]
def ellipse(cx, cy, w, h, angle=0.0, **kw):
    pts = rect(cx-w/2, cy-h/2, cx+w/2, cy+h/2)
    return shape(pts, "Ellipse", center=[cx,cy], w=w, h=h, angle=angle, **kw)
ff = dict(PolygonType="FreeForm")
case("solid", settings=ff, shapes=[shape(rect(2,1,6,4),fill="Solid",color=BLUE)])
case("outline", settings=ff, shapes=[shape(rect(2,1,6,4),fill="Outline",color=BLUE)])
case("solid-fractional", settings=ff, shapes=[shape([[2.5,1.5],[6.5,1.5],[3.49,4.51]],fill="Solid",color=BLUE)])
case("solid-line", settings=ff, shapes=[shape([[0,0],[19,9]],"Line",fill="Solid",color=BLUE)])
case("one-point-line", settings=ff, shapes=[shape([[3,3]],"Line",fill="Solid",color=BLUE)])
case("one-point-wipe-line", settings=ff, shapes=[shape([[3,3]],"Line")])
case("two-point-polygon", settings=ff, shapes=[shape([[3,3],[8,8]],fill="Solid",color=BLUE)])
case("one-point-polygon", settings=ff, shapes=[shape([[3,3]],fill="Solid",color=BLUE)])
case("static-brightness", settings=ff, ms=2000, shapes=[shape(rect(2,1,6,4),fill="Solid",color=[[0.0,[200,100,50]],[1.0,[10,20,30]]],fillBrightness=[[0,100],[100,20]])])
case("mix-of-everything", settings=ff, ms=2000, shapes=[shape(rect(0,0,19,9)),shape(rect(2,1,6,4),fill="Solid",color=BLUE),ellipse(12,5,6,4,fill="Outline",color=[[0.0,[0,255,0]]]),shape([[0,9],[19,0]],"Line",fill="Outline")])
for count, skip, direction in [(0,0,"Up"),(2,3,"Up"),(2,3,"Down"),(3,2,"Left"),(3,2,"Right")]:
    for fill in ["Solid","Outline"]:
        cfg = dict(FillType=fill,RepeatCount=count,RepeatSkip=skip,RepeatDirection=direction)
        case(f"pattern-{fill}-{count}-{direction}", settings=cfg, shapes=[shape(rect(5,4,9,6),fill=fill)], fillColor=BLUE)
        case(f"pattern-{fill}-line-{count}-{direction}", settings=cfg, shapes=[shape([[5,4],[9,6]],"Line",fill=fill)], fillColor=BLUE)
        case(f"pattern-{fill}-ellipse-{count}-{direction}", settings=cfg, shapes=[ellipse(10,5,6,4,20.0,fill=fill)], fillColor=BLUE)

# ---- ellipses
for w, h, angle in [(6,4,0.0),(8,8,0.0),(5,3,0.0),(7,7,45.0),(9,3,30.0),(9,3,-77.5),(16,1,90.0),(0,4,0.0),(4,0,0.0),(3.7,2.2,12.0)]:
    for fill in ["Solid","Outline"]:
        case(f"ellipse-{fill}-{w}x{h}-{angle}", 21, 21, 100, settings=ff, shapes=[ellipse(10.0,10.0,w,h,angle,fill=fill,color=BLUE)])
case("ellipse-off-center", 21, 21, 100, settings=ff, shapes=[ellipse(3.7,17.2,9,5,10.0,fill="Solid",color=BLUE)])
# wipes through the mask
for w, h, angle in [(10,6,0.0),(8,8,0.0),(12,4,30.0),(5,9,-60.0)]:
    case(f"ellipse-wipe-{w}x{h}-{angle}", 21, 21, 2000, settings=ff, shapes=[ellipse(10.0,10.0,w,h,angle)])
    case(f"ellipse-wipe-margin-{w}x{h}-{angle}", 21, 21, 2000, settings=dict(PolygonType="FreeForm",Margin=20), shapes=[ellipse(14.0,14.0,w,h,angle)])
    case(f"ellipse-wipe-accel-{w}x{h}-{angle}", 21, 21, 2000, settings=dict(PolygonType="FreeForm"), shapes=[ellipse(10.0,10.0,w,h,angle,acceleration=5,headDuration=50)])
# a wipe whose lines go off the top or right of the buffer, where the mask lookup throws
case("ellipse-wipe-off-right", 21, 21, 2000, settings=ff, shapes=[ellipse(19.0,10.0,10,6,0.0)])
case("ellipse-wipe-off-top", 21, 21, 2000, settings=ff, shapes=[ellipse(10.0,1.0,10,6,0.0)])
case("ellipse-wipe-off-left", 21, 21, 2000, settings=ff, shapes=[ellipse(1.0,10.0,10,6,0.0)])
case("ellipse-wipe-off-bottom", 21, 21, 2000, settings=ff, shapes=[ellipse(10.0,19.0,10,6,0.0)])
for direction in ["Up","Down","Left","Right"]:
    case(f"ellipse-pattern-wipe-{direction}", 21, 21, 2000, settings=dict(RepeatCount=2,RepeatSkip=5,RepeatDirection=direction,Stagger=10), shapes=[ellipse(10.0,10.0,6,4,0.0)])
    case(f"ellipse-pattern-wipe-far-{direction}", 21, 21, 2000, settings=dict(RepeatCount=4,RepeatSkip=5,RepeatDirection=direction,Stagger=10), shapes=[ellipse(10.0,10.0,6,4,0.0)])
case("ellipse-too-few-points", 21, 21, 500, settings=ff, shapes=[shape(rect(0,0,5,5)[:3],"Ellipse",center=[3,3],w=5,h=5,angle=0.0)])
# scaling
for w, h in [(21,21),(11,31),(41,11)]:
    case(f"ellipse-scale-{w}x{h}", 21, 21, 200, settings=dict(PolygonType="FreeForm",DisplayElementWidth=w,DisplayElementHeight=h), shapes=[ellipse(5.0,5.0,4,4,0.0,fill="Solid",color=BLUE)])
    case(f"ellipse-scale-rotated-{w}x{h}", 21, 21, 200, settings=dict(PolygonType="FreeForm",DisplayElementWidth=w,DisplayElementHeight=h), shapes=[ellipse(5.0,5.0,4,4,30.0,fill="Solid",color=BLUE)])
    case(f"ellipse-scale-wide-{w}x{h}", 21, 21, 200, settings=dict(PolygonType="FreeForm",DisplayElementWidth=w,DisplayElementHeight=h), shapes=[ellipse(w/2.0-0.5,h/2.0-0.5,w-1,h-1,0.0,fill="Solid",color=BLUE)])

# ---- time based
tb = dict(PolygonType="TimeBased")
case("time-two-polygons", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.0),shape(rect(16,0,20,4),time=1.0)])
case("time-two-polygons-outline", 11, 21, 1000, settings=dict(PolygonType="TimeBased",FillPolygon=False), fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.0),shape(rect(16,0,20,4),time=1.0)])
case("time-three", 11, 21, 2000, settings=tb, fillColor=[[0.0,[255,0,0]],[1.0,[0,0,255]]], fillBrightness=[[0,50],[100,100]], shapes=[shape(rect(0,0,4,4),time=0.1),shape(rect(16,0,20,4),time=0.6),shape(rect(5,5,9,9),time=0.9)])
case("time-before-and-after", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.4),shape(rect(16,0,20,4),time=0.6)])
case("time-lines", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape([[0,5],[4,5]],"Line",time=0.0),shape([[10,5],[14,5]],"Line",time=1.0)])
case("time-triangles", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape([[0,0],[4,0],[4,4]],time=0.0),shape([[10,0],[20,0],[20,10]],time=1.0)])
case("time-ellipses", 41, 41, 1000, settings=tb, fillColor=BLUE, shapes=[ellipse(10.0,20.0,4,4,0.0,time=0.0),ellipse(30.0,20.0,20,10,45.0,time=1.0)])
case("time-ellipses-outline", 41, 41, 1000, settings=dict(PolygonType="TimeBased",FillPolygon=False), fillColor=BLUE, shapes=[ellipse(10.0,20.0,4,4,0.0,time=0.0),ellipse(30.0,20.0,20,10,45.0,time=1.0)])
case("time-single-polygon", 11, 21, 500, settings=tb, fillColor=BLUE, shapes=[shape(rect(2,2,9,8))])
case("time-single-polygon-outline", 11, 21, 500, settings=dict(PolygonType="TimeBased",FillPolygon=False), fillColor=BLUE, shapes=[shape(rect(2,2,9,8))])
case("time-single-ellipse", 11, 21, 500, settings=tb, fillColor=BLUE, shapes=[ellipse(10.0,5.0,8,6,0.0)])
case("time-single-line", 11, 21, 500, settings=tb, fillColor=BLUE, shapes=[shape([[1,1],[9,9]],"Line")])
case("time-default", 11, 21, 500, settings=tb, fillColor=BLUE)
# shapes that don't line up: quirks and exceptions
case("time-polygon-then-line", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.0),shape([[0,0],[4,4]],"Line",time=1.0)])
case("time-line-then-polygon", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape([[0,0],[4,4]],"Line",time=0.0),shape(rect(0,0,4,4),time=1.0)])
case("time-ellipse-then-line", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[ellipse(5.0,5.0,4,4,0.0,time=0.0),shape([[0,0],[4,4]],"Line",time=1.0)])
case("time-line-then-ellipse", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape([[0,0],[4,4]],"Line",time=0.0),ellipse(5.0,5.0,4,4,0.0,time=1.0)])
case("time-different-point-counts", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.0),shape([[0,0],[4,0],[4,4]],time=0.5),shape(rect(9,0,14,4),time=1.0)])
case("time-more-points-later", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape([[0,0],[4,0],[4,4]],time=0.0),shape(rect(9,0,14,4),time=0.5),shape([[0,0],[4,0],[4,4]],time=1.0)])
case("time-same-times", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.5),shape(rect(9,0,14,4),time=0.5)])
case("time-unsorted", 11, 21, 1000, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.8),shape(rect(9,0,14,4),time=0.2),shape(rect(5,5,9,9),time=0.5)])
# locations through the fake GDI
case("location-static", strings=4, pixels=10, located=True, settings=ff, shapes=[shape(rect(0,0,20,8),fill="Solid",color=BLUE)])
case("location-ellipse-wipe", strings=4, pixels=10, located=True, settings=ff, ms=2000, shapes=[ellipse(18.0,6.0,10,6,0.0)])
case("location-time", strings=4, pixels=10, located=True, settings=tb, fillColor=BLUE, shapes=[shape(rect(0,0,4,4),time=0.0),shape(rect(16,0,20,4),time=1.0)])
json.dump(cases, open(sys.argv[1], "w"))
print(len(cases), "cases")
