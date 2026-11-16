"""Verify pinned dependencies; extract unchanged managed helper methods offline."""
from pathlib import Path
from hashlib import sha256
from zipfile import ZipFile
import json

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
OUT = ROOT / 'target/golden/helpers'
OUT.mkdir(parents=True, exist_ok=True)
references = json.loads((HERE / 'references.json').read_text())
for name, info in references.items():
    path = ROOT / 'target/helper-reference' / name if name.endswith('.nupkg') else HERE / 'reference' / name
    if not path.is_file():
        raise SystemExit(f'Missing {path}; run tools/golden/helpers/fetch_reference.py explicitly to cache the pinned package.')
    if sha256(path.read_bytes()).hexdigest() != info['sha256']:
        raise SystemExit(f'Reference hash mismatch: {path}')
with ZipFile(ROOT / 'target/helper-reference/naudio.core.2.3.0.nupkg') as archive:
    lib = OUT / 'lib'
    lib.mkdir(exist_ok=True)
    (lib / 'NAudio.Core.dll').write_bytes(archive.read('lib/netstandard2.0/NAudio.Core.dll'))

def original(path, signature):
    text = path.read_text(encoding='utf-8-sig')
    start = text.index(signature)
    opening = text.index('{', start)
    depth = 0
    for i in range(opening, len(text)):
        depth += (text[i] == '{') - (text[i] == '}')
        if depth == 0:
            return text[start:i + 1]
    raise ValueError(f'Incomplete source method: {path} {signature}')

ref = HERE / 'reference'
matrix = ref / 'Matrix.cs'
methods = [
    'public Matrix(double m11, double m12,',
    'public static Matrix operator *(',
    'public void RotateAt(double angle, double centerX, double centerY)',
    'public Point Transform(Point point)',
    'internal void MultiplyPoint(ref double x, ref double y)',
    'internal static Matrix CreateRotationRadians(double angle, double centerX, double centerY)',
    'private void SetMatrix(double m11, double m12,',
    'private void DeriveMatrixType()',
    '[Conditional("DEBUG")]\n        private void Debug_CheckType()',
]
parts = ['// Generated verbatim method bodies from pinned MIT dotnet/wpf source.\nusing System;\nusing System.Diagnostics;\nusing System.Windows.Media;\nusing MS.Internal;\n']
parts += ['namespace MS.Internal {', original(ref/'MatrixUtil.cs', 'internal enum MatrixTypes'),
          'internal static class MatrixUtil {', original(ref/'MatrixUtil.cs', 'internal static void MultiplyMatrix('), '} }']
parts += ['namespace System.Windows.Media { public partial struct Matrix {']
parts += [original(matrix, signature) for signature in methods]
parts += ['} public abstract partial class GeneralTransform {', original(ref/'GeneralTransform.cs', 'public Point Transform(Point point)'),
          '} public abstract partial class Transform {', original(ref/'Transform.cs', 'public override bool TryTransform(Point inPoint, out Point result)'), '} }']
(OUT/'OriginalWpfMethods.cs').write_text('\n'.join(parts)+'\n')
audio = ROOT/'upstream/Vixen/src/Vixen.Modules/Media/Audio/Audio.cs'
method = original(audio, 'public double[] GetMonoSamples(int startSample, int numSamples)')
(OUT/'OriginalAudioMethods.cs').write_text('using System;\nusing Common.AudioPlayer.SampleProvider;\nusing VixenModules.Media.Audio.SampleProviders;\nnamespace VixenModules.Media.Audio { public sealed partial class Audio {\n'+method+'\n} }\n')
paths = [audio,
    ROOT/'upstream/Vixen/src/Vixen.Common/AudioPlayer/SampleProvider/CachedSoundSampleProvider.cs',
    ROOT/'upstream/Vixen/src/Vixen.Modules/Media/Audio/SampleProviders/MonoSampleProvider.cs',
    ROOT/'upstream/Vixen/src/Vixen.Modules/Effect/Bars/Bars.cs',
    ROOT/'upstream/Vixen/Directory.Packages.props', ROOT/'upstream/Vixen/src/Directory.Build.props', ROOT/'upstream/Vixen/global.json']
(HERE/'upstream-manifest.sha256').write_text(''.join(sha256(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(ROOT))+'\n' for p in paths))
print('Verified NAudio.Core 2.3.0 and WPF v10.0.0; extracted unchanged helper methods.')
