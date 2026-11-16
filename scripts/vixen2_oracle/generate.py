#!/usr/bin/env python3
"""Regenerate the Vixen2 effect oracle with a caller-supplied local .NET 10 SDK.
No NuGet packages or external requests are needed; restore uses an empty feed.
"""
import json, pathlib, random, subprocess, sys
root=pathlib.Path(__file__).resolve().parents[2]
build=root/'target/vixen2-oracle';build.mkdir(parents=True,exist_ok=True)
feed=build/'empty-feed';feed.mkdir(exist_ok=True)
r=random.Random(399);cases=[]
for _ in range(150):
    color=-65536;events=[]
    for _ in range(r.randrange(1,120)):
        if r.random()<.07:color=r.choice([-65536,-16744448,-16776961,-15584170])
        events.append([color,r.choice([0,4,5,6,50,100,150,200,255])])
    cases.append(events)
input_file=build/'input.json';input_file.write_text(json.dumps(cases))
project=pathlib.Path(__file__).with_name('oracle.csproj')
dotnet=sys.argv[1]
subprocess.run([dotnet,'restore',str(project),'--source',str(feed)],check=True,stdout=sys.stderr)
output=subprocess.check_output([dotnet,'run','--project',str(project),'--no-restore','--',str(input_file)],cwd=root)
json.loads(output)
(root/'crates/vixen-session/tests/fixtures/vixen2-effects.json').write_bytes(output)
