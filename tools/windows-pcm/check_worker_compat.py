#!/usr/bin/env python3
"""PS5 source invariants plus actual .NET filesystem probes; not PowerShell execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import xml.sax.saxutils as xml

ROOT = Path(__file__).resolve().parents[2]
CS = r'''
using System;
using System.IO;
using System.Text;
class AtomicProbe {
    static void Save(string text, string path) {
        string temporary = path + ".tmp";
        File.WriteAllText(temporary, text, new UTF8Encoding(false));
        if (File.Exists(path)) {
            string previous = path + ".previous";
            if (File.Exists(previous)) File.Delete(previous);
            File.Replace(temporary, path, previous);
        } else File.Move(temporary, path);
    }
    static void Check(bool value, string name) {
        if (!value) throw new Exception(name); Console.WriteLine("PASS " + name);
    }
    static void Main(string[] args) {
        string path = Path.Combine(args[0], "private checkpoint é.json");
        Save("one", path); Check(File.ReadAllText(path)=="one" && !File.Exists(path+".previous"), "first-move");
        Save("two", path); Check(File.ReadAllText(path)=="two" && File.ReadAllText(path+".previous")=="one", "replace-with-explicit-backup");
        Save("three", path); Check(File.ReadAllText(path)=="three" && File.ReadAllText(path+".previous")=="two", "repeated-backup-rotation");
        File.WriteAllText(path+".tmp", "four");
        bool rejected = false;
        try { File.Replace(path+".tmp", path, ""); } catch (ArgumentException) { rejected = true; }
        Check(rejected && File.ReadAllText(path)=="three" && File.ReadAllText(path+".previous")=="two", "empty-backup-rejected-prior-preserved");
        File.Delete(path+".previous"); Directory.CreateDirectory(path+".previous");
        rejected = false;
        try { Save("four", path); } catch (IOException) { rejected = true; } catch (UnauthorizedAccessException) { rejected = true; }
        Check(rejected && File.ReadAllText(path)=="three" && File.ReadAllText(path+".tmp")=="four", "failed-backup-preserves-current-and-partial");
        Directory.Delete(path+".previous"); Save("five", path);
        Check(File.ReadAllText(path)=="five" && File.ReadAllText(path+".previous")=="three", "later-save-recovers");
    }
}
'''

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    out.mkdir(parents=True)
    worker = ROOT/'tools/windows-pcm/Capture-Worker.ps1'
    source = worker.read_text()
    save = source[source.index('function Save('):source.index('function Say(')]
    checks = []
    def check(name, value):
        assert value, name
        checks.append(name)
    check('explicit-private-backup', "$previous = $path + '.previous'" in save and '[IO.File]::Replace($temporary, $path, $previous)' in save)
    check('no-null-string-argument', not re.search(r'\[IO\.File\]::(?:Replace|Move)\([^\n]*\$null', source))
    check('write-before-replace', save.index('Set-Content -LiteralPath $temporary') < save.index('[IO.File]::Replace'))
    check('prior-preserved-until-replace', '[IO.File]::Delete($path)' not in save and '[IO.File]::Delete($previous)' in save)
    check('first-checkpoint-move', '[IO.File]::Move($temporary, $path)' in save)
    check('missing-inventory-skip', 'skipped-inventory-unavailable' in source and source.index('[IO.File]::Exists($choicePath)') < source.index('(Get-Item -LiteralPath $choicePath).Length'))
    check('incomplete-inventory-skip', "$inventory.status -ne 'complete'" in source)
    check('per-entry-failure-continues', "catch { $data.errors += ('Entry ' + $entry" in source)
    check('per-copy-path-check-protected', re.search(r'try \{\s*\$source = Local \$source', source) is not None)
    check('partials-count-against-copy-budget', '$reservedBytes + $length -gt 67108864' in source and source.index('$reservedBytes += $length') < source.index('$row.sha256Before ='))
    compile_source = (ROOT/'tools/windows-pcm/Compile-Supervisor.ps1').read_text()
    check('compiler-error-separate-encoding', "'compiler-error.txt'" in compile_source and 'AppendAllText($log' not in compile_source)
    (out/'save-function.ps1').write_text(save)
    (out/'AtomicProbe.cs').write_text(CS)
    (out/'AtomicProbe.csproj').write_text('''<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net10.0</TargetFramework><UseAppHost>false</UseAppHost><LangVersion>5</LangVersion><OutputPath>bin/</OutputPath></PropertyGroup></Project>''')
    dotnet = ROOT/'target/golden/dotnet-root/dotnet'
    env = os.environ.copy()
    env.update(DOTNET_ROOT=str(dotnet.parent), DOTNET_CLI_HOME=str(ROOT/'target/golden/cli-home'), DOTNET_CLI_TELEMETRY_OPTOUT='1', MSBuildEnableWorkloadResolver='false')
    with (out/'build.log').open('w') as log:
        subprocess.run([str(dotnet),'build',str(out/'AtomicProbe.csproj'),'-c','Release','--configfile',str(ROOT/'tools/windows-pcm/nuget.config'),'-p:UseSharedCompilation=false'],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=90,check=True)
    fixture = out/'fresh files';fixture.mkdir()
    run = subprocess.run([str(dotnet),str(out/'bin/net10.0/AtomicProbe.dll'),str(fixture)],env=env,capture_output=True,text=True,timeout=20)
    (out/'run.log').write_text(run.stdout+run.stderr)
    run.check_returncode()
    filesystem = [line[5:] for line in run.stdout.splitlines() if line.startswith('PASS ')]
    assert len(filesystem)==6
    report={'passed':True,'staticChecks':checks,'filesystemChecks':filesystem,'workerSha256':hashlib.sha256(worker.read_bytes()).hexdigest(),
            'scope':'Production PowerShell function invariants plus C# equivalent explicit-backup filesystem operations on local .NET; no PowerShell5.1 or Windows execution', 'powershellExecuted':False}
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'staticChecks':len(checks),'filesystemChecks':len(filesystem),'report':str(out/'result.json')}))

if __name__=='__main__':main()
