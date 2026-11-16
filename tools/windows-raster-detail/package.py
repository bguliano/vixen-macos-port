#!/usr/bin/env python3
"""Create a fresh source-only Windows raster pack; no runtime or user data bundled."""
import argparse,hashlib,json,shutil,subprocess,sys,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--checks',type=Path,required=True);a=p.parse_args()
    out=a.out.resolve();assert out.is_relative_to(ROOT/'target') and not out.exists()
    report=json.loads(a.checks.read_text());assert report['passed'] and not report['windowsDrawingExecuted']
    for name,digest in report['sourceHashes'].items():assert sha(ROOT/name)==digest,'Local-check source drift: '+name
    for name,digest in report['externalTestInputs'].items():assert sha(ROOT/name)==digest,'Local fixture drift: '+name
    subprocess.run([sys.executable,str(HERE/'extract_sources.py'),'--check'],capture_output=True,check=True,timeout=10)
    subprocess.run([sys.executable,str(HERE/'generate_cases.py'),'--check'],capture_output=True,check=True,timeout=10)
    folder=out/'Vixen-Windows-Raster-Detail';folder.mkdir(parents=True)
    def copy(path,name):
        dest=folder/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,dest)
    names=['Run-Raster.cmd','Watch-Raster.ps1','Run-Raster.ps1','Compile-Supervisor.ps1','Pack-Worker.ps1','SupervisorEngine.cs.in','RasterWorkflow.cs.in','RasterCapture.cs.in','Shared.cs.in','OriginalPictureMethods.cs.in','OriginalCountDownMethods.cs.in','OriginalFastPixel.cs.in','cases.json','source-contract.json','START-HERE.txt','README.md']
    for name in names:copy(HERE/name,name)
    for name in ['extract_sources.py','generate_cases.py','package.py','check_local.py','check_package.py']:copy(HERE/name,'audit/'+name)
    for path in sorted((HERE/'tests').glob('*')):
        if path.is_file():copy(path,'audit/tests/'+path.name)
    copy(ROOT/'tools/windows-pcm/v3-tests/fault_child.py','audit/tests/fault_child.py')
    contract=json.loads((HERE/'source-contract.json').read_text())
    copy(ROOT/contract['picture']['path'],'reference/Picture.cs.txt')
    copy(ROOT/contract['countDown']['path'],'reference/CountDown.cs.txt')
    copy(ROOT/contract['fastPixel']['path'],'reference/FastPixel.cs.txt')
    copy(ROOT/contract['supervisor']['path'],'reference/Supervisor-parent.cs.txt')
    copy(ROOT/'upstream/Vixen/License.txt','licenses/Vixen-License.txt')
    copy(a.checks.resolve(),'audit/local-result.json')
    provenance={'schema':3,'purpose':'80 focused numeric captures:60 exact-dimension Picture DrawImage overload resizes,4 original grayscale alpha/tie swatches,8 native ellipse/path comparisons and8 fractional forward/reverse lines; source-only programs compiled using existing Windows csc.exe','referenceKind':'checked-out-source-helper-and-framework-primitives','installedVixenBinaryContract':False,'upstreamCommit':contract['upstreamCommit'],'sourceContractSha256':sha(HERE/'source-contract.json'),'caseGridSha256':sha(HERE/'cases.json'),'localChecksSha256':sha(a.checks.resolve()),'localChecks':len(report['checks']),'windowsDrawingExecuted':False,'frameworkCompilerExecuted':False,'powershellExecuted':False,'nativeWindowsJobObjectsExecuted':False,'localBuildReference':'Cached net10 references with C# language version5; not Framework4 compile proof','runtimeDownloadOrInstall':False,'privateUserFilesIncluded':False,'runtimeBinariesIncluded':False,'scope':'Resize cases use original Picture integer destination-size DrawImage overload with deliberately independent dimensions to distinguish width dispatch; they are numeric primitives, not ScalePictureImage aspect-fit results. Grayscale uses the unmodified original method with explicit alpha rows and RGB swatches. Ellipse paths record float32 bits, native paths and flattened variants. CountDown source is provenance only and is not executed. No full-effect, installed Vixen, raster parity or Windows execution claim.'}
    (folder/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
    files=[dict(path=str(f.relative_to(folder)).replace('\\','/'),bytes=f.stat().st_size,sha256=sha(f)) for f in sorted(folder.rglob('*')) if f.is_file()]
    assert len(files)<100 and all(f['bytes']<=1048576 for f in files)
    (folder/'pack-manifest.json').write_text(json.dumps(dict(schema=1,files=files),indent=2)+'\n')
    archive=out/'Vixen-Windows-Raster-Detail.zip'
    with zipfile.ZipFile(archive,'x',compression=zipfile.ZIP_DEFLATED) as z:
        for f in sorted(folder.rglob('*')):
            if f.is_file():z.write(f,f.relative_to(out).as_posix())
    with zipfile.ZipFile(archive) as z:
        members=z.namelist();assert len(members)==len(set(members))==len(files)+1
        for f in files:
            raw=z.read('Vixen-Windows-Raster-Detail/'+f['path']);assert len(raw)==f['bytes'] and hashlib.sha256(raw).hexdigest()==f['sha256']
    subprocess.run([sys.executable,str(HERE/'check_package.py'),str(archive)],capture_output=True,check=True,timeout=10)
    result=dict(zip=str(archive),sha256=sha(archive),bytes=archive.stat().st_size,files=len(files)+1,sourceOnly=True,windowsExecuted=False,delivered=False)
    (out/'package-result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
if __name__=='__main__':main()
