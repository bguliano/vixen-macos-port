"""Extract unchanged original binary helper methods and hash the exact reference inputs."""
from pathlib import Path
import hashlib
root = Path(__file__).resolve().parents[3]
out = root / 'target/golden/defaults'
out.mkdir(parents=True, exist_ok=True)
source = root / 'upstream/Vixen/src/Vixen.Core/Services/EffectDefaults/EffectDefaultsService.cs'
text = source.read_text(encoding='utf-8-sig')
methods = []
for signature in ['internal static byte[] WriteBinary(', 'internal static object ReadBinary(']:
    start = text.index(signature)
    opening = text.index('{', start)
    depth = 0
    for end in range(opening, len(text)):
        depth += (text[end] == '{') - (text[end] == '}')
        if depth == 0:
            methods.append(text[start:end+1])
            break
(out / 'OriginalBinaryMethods.cs').write_text('using System.Runtime.Serialization;\nusing System.Xml;\ninternal static class OriginalBinary {\n'+'\n'.join(methods)+'\n}\n')
paths = [source, source.with_name('EffectDefaultsStore.cs'), root/'upstream/Vixen/src/Vixen.Core/Module/ModuleDataModelBase.cs']
paths += [root/'upstream/Vixen/src/Vixen.Modules/Effect'/p for p in ['Pulse/PulseData.cs','Pattern/WeaveData.cs','Wave/Wave/WaveData.cs','Wave/Wave/WaveformData.cs','Wave/Wave/Waveform.cs']]
paths += sorted((root/'target/golden/bin').glob('*.dll'))
paths += [root/'tools/golden/defaults'/p for p in ['DefaultsGolden.csproj','Program.cs','prepare.py','verify.py','build.sh']]
paths += [root/'crates/vixen-session/examples/effect_defaults_oracle.rs', out/'OriginalBinaryMethods.cs']
(out / 'source-manifest.sha256').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(root))+'\n' for p in paths))
