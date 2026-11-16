from pathlib import Path
import hashlib
root = Path(__file__).resolve().parents[3]
source = root / 'upstream/Vixen/src/Vixen.Modules/App/Curves/CurveEditor.cs'
text = source.read_text()
start = text.index('\t\tprivate void btnFunctionCurve_Click(')
end = text.index('\t\tprivate void textBoxThreshold_KeyPress(', start)
methods = text[start:end]
out = root / 'target/golden/ncalc'
out.mkdir(parents=True,exist_ok=True)
(out / 'OriginalCurveMethods.cs').write_text('using System;\nusing NCalc2;\npublic partial class OriginalCurveEditor {\n'+methods+'}\n')
files = [source, root/'upstream/Vixen/Directory.Packages.props'] + sorted((root/'target/ncalc-reference').glob('*.nupkg')) + sorted((root/'target/ncalc-reference/lib').glob('*.dll'))
lines = [hashlib.sha256(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(root)) for p in files]
(root/'tools/golden/ncalc/source-manifest.sha256').write_text('\n'.join(lines)+'\n')
