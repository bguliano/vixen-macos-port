#!/usr/bin/env python3
# Cases for comparing the Rust Liquid with the real Liquid.cs: a JSON list for C#, and the same list as Rust code.
import json, random, sys, uuid
R = random.Random(20261109)
PARTICLE = ["Elastic","Powder","Tensile","Spring","Viscous","StaticPressure","Water","Reactive","Repulsive"]
NOZZLE = ["FixedAngle","Oscillate","SpinClockwise","SpinCounterClockwise"]
FLOW = ["Continuous","Pulsating","UseMarks","Musical"]

def rcurve(lo=0, hi=100):
    kind = R.random()
    if kind < 0.5:
        v = R.choice([lo, hi, R.uniform(lo,hi), R.randint(lo,hi)])
        return [[0,v],[100,v]]
    if kind < 0.8:
        return [[0,R.uniform(lo,hi)],[100,R.uniform(lo,hi)]]
    return [[0,R.uniform(lo,hi)],[R.uniform(10,90),R.uniform(lo,hi)],[100,R.uniform(lo,hi)]]

def rcolor(): return [R.randint(0,255),R.randint(0,255),R.randint(0,255)]
def rgrad():
    n = R.choice([1,1,2,3])
    if n == 1: return [[0.0,rcolor()]]
    return [[i/(n-1),rcolor()] for i in range(n)]

def base_emitter():
    return dict(Color=rgrad(), ParticleVelocity=rcurve(), Y=rcurve())

def random_emitter(marks):
    e = base_emitter()
    e["ParticleType"] = R.choice(PARTICLE)
    if R.random() < 0.25:
        e["UseColorArray"] = True
        e["FramesPerColor"] = R.randint(1,5)
    if R.random() < 0.5: e["Brightness"] = rcurve()
    if R.random() < 0.5: e["Lifetime"] = rcurve()
    if R.random() < 0.6:
        e["Animate"] = True
        e["AnimateXStart"] = R.randint(0,100)
        e["AnimateYStart"] = R.randint(0,100)
        if R.random() < 0.5: e["EdgeHandling"] = "Wrap"
        if R.random() < 0.8: e["VelocityX"] = rcurve()
        if R.random() < 0.8: e["VelocityY"] = rcurve()
    if R.random() < 0.7: e["X"] = rcurve()
    if R.random() < 0.7: e["Flow"] = rcurve()
    if R.random() < 0.5: e["SourceSize"] = R.choice([rcurve(0,10), [[0,3],[100,3]], rcurve()])
    nozzle = R.choice(NOZZLE)
    e["NozzleMovement"] = nozzle
    if R.random() < 0.6: e["NozzleAngle"] = rcurve()
    if nozzle == "Oscillate":
        e["OscillateStartAngle"] = R.randint(0,180)
        e["OscillateEndAngle"] = R.randint(0,360)
    if R.random() < 0.6: e["OscillationSpeed"] = rcurve()
    flow = R.choice(FLOW)
    e["FlowControl"] = flow
    if flow == "Pulsating":
        e["OnTime"] = R.choice([50,100,120,300,2000])
        e["OffTime"] = R.choice([50,75,100,250,2000])
    if flow == "UseMarks" or R.random() < 0.1:
        choice = R.random()
        if marks and choice < 0.6:
            e["MarkCollectionId"] = R.choice(marks)["id"]
        elif choice < 0.8:
            e["MarkCollectionId"] = str(uuid.UUID(int=R.getrandbits(128)))
    return e

cases = []
def add(id, strings, pixels, ms, emitters, settings=None, **kw):
    c = dict(id=id, strings=strings, pixels=pixels, ms=ms, emitters=emitters, settings=settings or {})
    c.update(kw)
    cases.append(c)

# ---- hand picked
add("nothing", 8, 25, 500, [])
add("one-still", 8, 25, 500, [base_emitter()])
add("empty-color-list", 8, 25, 500, [dict(base_emitter(), UseColorArray=True)], colors=[])
add("zero-scale", 4, 10, 500, [base_emitter()], dict(RenderScaleFactor=0), located=True)
for i in range(4):
    add(f"marks-{i}", 8, 25, 1000, [dict(base_emitter(), FlowControl="UseMarks", MarkCollectionId="11111111-1111-1111-1111-111111111111")],
        marks=[dict(id="11111111-1111-1111-1111-111111111111", name="M" if i != 3 else "", items=[[100,50],[300,100]])], startMs=[0,100,0,0][i])
add("marks-none-selected", 8, 25, 500, [dict(base_emitter(), FlowControl="UseMarks")], marks=[dict(id="22222222-2222-2222-2222-222222222222", name="First", items=[[100,50]]), dict(id="33333333-3333-3333-3333-333333333333", name="Second", items=[[300,50]])])
add("marks-no-collections", 8, 25, 500, [dict(base_emitter(), FlowControl="UseMarks")])
for flat in [(1,1),(0,0)]:
    add(f"musical-no-audio-{flat[0]}", 8, 25, 500, [dict(base_emitter(), FlowControl="Musical")])

# ---- random
for i in range(400):
    strings, pixels = R.choice([(8,25),(10,10),(20,5),(1,30),(30,1),(4,10),(6,10)])
    located = R.random() < 0.25
    if located: strings, pixels = R.choice([(4,10),(6,10),(3,7)])
    marks = []
    for m in range(R.choice([0,0,1,2])):
        t = 0; items = []
        for _ in range(R.randint(0,5)):
            t += R.randint(0,400); d = R.randint(0,300)
            items.append([t,d]); t += d
        marks.append(dict(id=str(uuid.UUID(int=R.getrandbits(128))), name=R.choice(["A","B",""]) , items=items))
    emitters = [random_emitter(marks) for _ in range(R.choice([1,1,2,3,4]))]
    s = {}
    for k in ["BottomBarrier","TopBarrier","LeftBarrier","RightBarrier"]:
        if R.random() < 0.3: s[k] = True
    if R.random() < 0.3: s["MixColors"] = False
    if R.random() < 0.3: s["ParticleSize"] = R.choice([1,250,1000,5000])
    if R.random() < 0.3: s["WarmUpFrames"] = R.randint(1,12)
    if located: s["RenderScaleFactor"] = R.randint(1,5)
    if R.random() < 0.3: s["Gain"] = R.randint(0,5)
    if R.random() < 0.3: s["AttackTime"] = R.choice([0,10,100])
    if R.random() < 0.3: s["DecayTime"] = R.choice([0,300,3000])
    if R.random() < 0.3: s["Normalize"] = False
    if R.random() < 0.2: s["LowPass"] = True; s["LowPassFreq"] = R.choice([100,500,2000])
    if R.random() < 0.2: s["HighPass"] = True; s["HighPassFreq"] = R.choice([100,500,2000])
    kw = {}
    if R.random() < 0.3 and not located: kw["orientation"] = "Vertical"
    if located: kw["located"] = True
    if R.random() < 0.3: kw["startMs"] = R.choice([50,100,700])
    if R.random() < 0.3: kw["colors"] = [rgrad() for _ in range(R.randint(1,4))]
    if any(e.get("FlowControl") == "Musical" for e in emitters) and R.random() < 0.85:
        n = R.randint(1,6); segs = []
        for _ in range(n): segs.append([R.randint(0,5000), R.choice([0.0,0.001,0.1,0.5,-0.5,1.0,0.25])])
        kw["audio"] = dict(rate=R.choice([8000,4000]), samples=segs)
    add(f"random-{i}", strings, pixels, R.choice([100,333,500,1000,1950]), emitters, s, marks=marks, **kw)

json.dump(cases, open(sys.argv[1], "w"))

# ---- the same cases as Rust
def rs_curve(c): return "curve(&[" + ",".join(f"({float(a)!r},{float(b)!r})" for a,b in c) + "])"
def rs_grad(g): return "grad(&[" + ",".join(f"({float(p)!r},({c[0]},{c[1]},{c[2]}))" for p,c in g) + "])"
out = []
out.append("fn cases() -> Vec<Case> { let mut v: Vec<Case> = Vec::new();")
for c in cases:
    lines = []
    for e in c["emitters"]:
        parts = ["{ let mut e = Params::defaults(&gen::EMITTER_PARAMS); e.set(\"RandomStartingPosition\", ParamValue::Bool(false));"]
        for k, val in e.items():
            if k in ("Color",):
                parts.append(f"e.set(\"{k}\", ParamValue::Gradient({rs_grad(val)}));")
            elif k in ("ParticleVelocity","Y","Brightness","Lifetime","VelocityX","VelocityY","X","Flow","SourceSize","NozzleAngle","OscillationSpeed"):
                parts.append(f"e.set(\"{k}\", ParamValue::Curve({rs_curve(val)}));")
            elif k in ("ParticleType","EdgeHandling","NozzleMovement","FlowControl"):
                parts.append(f"e.set(\"{k}\", ParamValue::Enum(\"{val}\".to_string()));")
            elif k in ("UseColorArray","Animate"):
                parts.append(f"e.set(\"{k}\", ParamValue::Bool({'true' if val else 'false'}));")
            elif k == "MarkCollectionId":
                parts.append(f"e.set(\"{k}\", ParamValue::Guid(Uuid::parse_str(\"{val}\").unwrap()));")
            else:
                parts.append(f"e.set(\"{k}\", ParamValue::Int({val}));")
        parts.append("e }")
        lines.append(" ".join(parts))
    edit = ["p.set(\"EmitterData\", ParamValue::RecordList(vec![" + ",\n".join(lines) + "]));"]
    for k, val in c["settings"].items():
        if isinstance(val, bool):
            edit.append(f"p.set(\"{k}\", ParamValue::Bool({'true' if val else 'false'}));")
        else:
            edit.append(f"p.set(\"{k}\", ParamValue::Int({val}));")
    edit.append(f"p.set(\"MixColors\", ParamValue::Bool({'false' if c['settings'].get('MixColors') is False else 'true'}));")
    if "colors" in c:
        edit.append("p.set(\"Colors\", ParamValue::GradientList(vec![" + ",".join(rs_grad(g) for g in c["colors"]) + "]));")
    if c.get("located"):
        edit.append("p.set(\"TargetPositioning\", ParamValue::Enum(\"Locations\".to_string()));")
    if c.get("orientation"):
        edit.append(f"p.set(\"StringOrientation\", ParamValue::Enum(\"{c['orientation']}\".to_string()));")
    marks = "vec![" + ",".join(f"marks(\"{m['id']}\",\"{m['name']}\",&[" + ",".join(f"({a},{b})" for a,b in m["items"]) + "])" for m in c.get("marks",[])) + "]"
    audio = "None"
    if "audio" in c:
        a = c["audio"]
        audio = f"Some(audio({a['rate']},&[" + ",".join(f"({n},{float(val)!r})" for n,val in a["samples"]) + "]))"
    out.append(f"v.push(Case {{ id: \"{c['id']}\", strings: {c['strings']}, pixels: {c['pixels']}, ms: {c['ms']}, located: {'true' if c.get('located') else 'false'}, start_ms: {c.get('startMs',0)}, marks: {marks}, audio: {audio}, edit: Box::new(|p: &mut Params| {{ " + " ".join(edit) + " }) });")
out.append("v }")
open(sys.argv[2], "w").write("\n".join(out))
print(len(cases), "liquid cases")
