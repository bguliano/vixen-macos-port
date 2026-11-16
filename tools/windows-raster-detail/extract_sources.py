#!/usr/bin/env python3
"""Deterministically extract unchanged Picture helpers and the proven scheduler.

No original source is modified. --check refuses drift in checked-in extracts.
"""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def section(text,signature):
    a=text.index(signature);b=text.index('{',a);depth=1;i=b+1
    while depth:
        depth+=(text[i]=='{')-(text[i]=='}');i+=1
    return text[a:i],text[:a].count('\n')+1,text[:i].count('\n')+1

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    picture=ROOT/'upstream/Vixen/src/Vixen.Modules/Effect/Picture/Picture.cs'
    fast=ROOT/'upstream/Vixen/src/Vixen.Common/FastPixel/FastPixel.cs'
    countdown=ROOT/'upstream/Vixen/src/Vixen.Modules/Effect/CountDown/CountDown.cs'
    supervisor=ROOT/'tools/windows-pcm/Supervisor.cs'
    assert sha(picture.read_bytes())=='a5102426351a0f1b75c7f06369b1b6d2eb0c06c7491a72dba4bb655a191ecab9'
    assert sha(fast.read_bytes())=='74afded565f8dc3aa8c68061dbeec0d753f6cb9380c93ae4dac4d394d90bbffb'
    assert sha(supervisor.read_bytes())=='53804924a18c76a0186a2ce7b26b73a5bd0039fe74dbc3bbe0168f4246bcf5a1'
    assert sha(countdown.read_bytes())=='b5ea8344844b6b024091a6359c601455250052868e3505f10b7a8cfec5c182e2'
    methods=[];bodies=[]
    for signature in ['public static Bitmap ScalePictureImage(Image image, int maxWidth, int maxHeight)','public static Bitmap ScaleImage(Image image, double scale)','public static Image ConvertToGrayScale(Image srce)']:
        body,start,end=section(picture.read_text(encoding='utf-8-sig'),signature);bodies.append(body)
        methods.append(dict(signature=signature,startLine=start,endLine=end,sha256=sha(body.encode())))
    files={'OriginalPictureMethods.cs.in':'using System;\nusing System.Drawing;\nusing System.Drawing.Imaging;\ninternal sealed class OriginalPictureMethods {\n'+'\n'.join(bodies)+'\n}\n',
           'OriginalFastPixel.cs.in':'using System;\n'+fast.read_text(encoding='utf-8-sig')}
    count_methods=[];count_bodies=[]
    for signature in ['private void DrawFilmSpinner(','private void DrawFilmSpinnerCrossHash(Graphics graphics, Rectangle rect, Pen pen)','private void DrawFilmSpinnerRadialLine(Graphics graphics, Pen pen, int angleInDegrees, int lineStartX, int lineStartY, int lineLength)']:
        body,start,end=section(countdown.read_text(encoding='utf-8-sig'),signature);count_bodies.append(body)
        count_methods.append(dict(signature=signature,startLine=start,endLine=end,sha256=sha(body.encode())))
    files['OriginalCountDownMethods.cs.in']='using System;\nusing System.Drawing;\ninternal sealed class OriginalCountDownMethods {\n'+'''// Explicit uniform-white brush fixture; original gradient selection is outside this oracle.
private Brush CreateCrossHashBrush(Graphics graphics, Rectangle rectangle) { return new SolidBrush(Color.White); }
public void Cross(Graphics graphics, Rectangle rectangle, Pen pen) { DrawFilmSpinnerCrossHash(graphics, rectangle, pen); }
public void Spinner(Graphics graphics, Rectangle rectangle, int length, int angle) { DrawFilmSpinner(graphics, rectangle, 0, length, angle); }
'''+ '\n'.join(count_bodies)+'\n}\n'
    s=supervisor.read_text();prefix=s[:s.index('namespace VixenCapture')]
    types=s[s.index('    public sealed class StageSpec'):s.index('    [DataContract] public sealed class InventoryChoice')]
    console=section(s,'internal static class ConsoleProgress')[0]
    engine=s[s.index('    public sealed class CaptureSupervisor'):s.index('        public static string OptionalProbeSkipReason')]
    # Only the per-stream cap and an explicit reserve setter differ in the core.
    engine=engine.replace('32L * 1024 * 1024','1L * 1024 * 1024')
    engine+='        public void Reserve(double seconds) { reserveSeconds = Math.Max(0, seconds); }\n'
    for signature in ['public static T ReadJson<T>','public static string ReadText']:
        engine+='        '+section(s,signature)[0]+'\n'
    engine+='    }\n'
    native='\n'.join(section(s,signature)[0] for signature in ['public static class Paths','public sealed class WindowsLauncher','public sealed class WindowsChild','internal static class Native'])
    files['SupervisorEngine.cs.in']=prefix+'namespace VixenCapture {\n'+types+console+'\n'+engine+native+'\n}\n'
    provenance={'schema':2,'scope':'Original Picture and CountDown method signatures/bodies preserved exactly after UTF-8 BOM/newline decoding; complete FastPixel with explicit using System. CountDown brush selection replaced by explicit constant-white fixture adapter. Primitives directly call Framework drawing APIs. No whole Vixen effects or binary release equivalence claimed.',
                'upstreamCommit':'e111ae41963abfa48b70f884445950e94e432e02','picture':{'path':str(picture.relative_to(ROOT)),'sha256':sha(picture.read_bytes()),'methods':methods},
                'fastPixel':{'path':str(fast.relative_to(ROOT)),'sha256':sha(fast.read_bytes()),'adaptation':'Explicit using System; original class body unchanged.'},
                'countDown':{'path':str(countdown.relative_to(ROOT)),'sha256':sha(countdown.read_bytes()),'methods':count_methods,'adaptation':'Unchanged private DrawFilmSpinner, cross hash and radial line bodies, public forwarding wrappers and explicit uniform-white CreateCrossHashBrush adapter. Original gradient-selection behavior excluded.'},
                'supervisor':{'path':str(supervisor.relative_to(ROOT)),'sha256':sha(supervisor.read_bytes()),'scope':'StageSpec/StageResult/interfaces, ConsoleProgress, CaptureSupervisor constructor/stage/checkpoint/parser, Paths and Windows native launcher extracted. PCM/inventory workflow and entry point omitted. Summary DTO supplied separately. Per-stream cap 32MiB→1MiB; explicit Reserve setter added.'},
                'generated':{name:sha(text.encode()) for name,text in files.items()}}
    files['source-contract.json']=json.dumps(provenance,indent=2)+'\n'
    for name,text in files.items():
        if args.check: assert (HERE/name).read_bytes()==text.encode(),name
        else:(HERE/name).write_text(text)
    print(json.dumps({'passed':True,'checked':args.check,'files':list(files)}))
if __name__=='__main__':main()
