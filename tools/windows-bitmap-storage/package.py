#!/usr/bin/env python3
"""Prepare a fresh source-only supplement; never launches Windows or Vixen."""
from pathlib import Path
import argparse,hashlib,json,shutil,zipfile
R=Path(__file__).resolve().parents[2];S=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--checks',type=Path,required=True);a=p.parse_args();out=a.out.resolve();assert out.is_relative_to(R/'target') and not out.exists()
 checks=json.loads(a.checks.read_text());assert checks['passed'] and checks['windowsDrawingExecuted'] is False
 for name,h in checks['sourceHashes'].items():assert sha(R/name)==h,'Input changed after local checks: '+name
 folder=out/'Vixen-Windows-Bitmap-Storage-01';folder.mkdir(parents=True)
 for f in sorted(S.iterdir()):
  if f.is_file():shutil.copy2(f,folder/f.name)
 shutil.copy2(R/'scripts/raster_picture_common.py',folder/'raster_picture_common.py')
 (folder/'reference').mkdir();shutil.copy2(R/'upstream/Vixen/src/Vixen.Modules/Effect/Picture/Picture.cs',folder/'reference/Picture.cs.txt');shutil.copy2(R/'upstream/Vixen/License.txt',folder/'reference/Vixen-License.txt')
 shutil.copy2(a.checks,folder/'local-checks.json')
 records=[dict(path=f.relative_to(folder).as_posix(),bytes=f.stat().st_size,sha256=sha(f)) for f in sorted(folder.rglob('*')) if f.is_file()]
 assert 10<=len(records)<=100 and all(r['bytes']<=1048576 for r in records)
 (folder/'pack-manifest.json').write_text(json.dumps(dict(schema=1,files=records),indent=2)+'\n');archive=out/'Vixen-Windows-Bitmap-Storage-01.zip'
 with zipfile.ZipFile(archive,'x',compression=zipfile.ZIP_DEFLATED) as z:
  for f in sorted(folder.rglob('*')):
   if f.is_file():z.write(f,f.relative_to(out).as_posix())
 with zipfile.ZipFile(archive) as z:
  assert len(z.namelist())==len(set(z.namelist()))==len(records)+1
  for item in records:
   b=z.read(folder.name+'/'+item['path']);assert len(b)==item['bytes'] and hashlib.sha256(b).hexdigest()==item['sha256']
 report=dict(passed=True,archive=str(archive),sha256=sha(archive),bytes=archive.stat().st_size,entries=len(records)+1,sourceOnly=True,windowsExecuted=False,delivered=False)
 (out/'package-result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
if __name__=='__main__':main()
