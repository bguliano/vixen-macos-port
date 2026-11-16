#!/usr/bin/env python3
"""Copy production Picture unchanged, append observer, use private offline build."""
import argparse, hashlib, json, shutil, subprocess, tomllib
from pathlib import Path
REPO=Path(__file__).resolve().parents[2]
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--fixtures',type=Path,required=True);a=p.parse_args()
    out=a.out.resolve();fixtures=a.fixtures.resolve();out.mkdir()
    inputs={str(f.relative_to(REPO)):sha(f) for f in (REPO/'crates').rglob('*') if f.is_file() and (f.suffix=='.rs' or f.name=='Cargo.toml')}
    inputs['Cargo.toml']=sha(REPO/'Cargo.toml');inputs['Cargo.lock']=sha(REPO/'Cargo.lock')
    src=REPO/'crates/fx-pixel-graphics';private=out/'private-graphics';shutil.copytree(src,private)
    adapter=Path(__file__).with_name('mac_adapter.rs').read_bytes();original=(private/'src/picture.rs').read_bytes()
    (private/'src/picture.rs').write_bytes(original+b'\n'+adapter)
    cargo=(src/'Cargo.toml').read_text().split('[dev-dependencies]')[0]
    cargo=cargo.replace('version.workspace = true','version = "3.13.4"').replace('edition.workspace = true','edition = "2021"')
    root=tomllib.loads((REPO/'Cargo.toml').read_text())['workspace']['dependencies']
    for name in ['vixen-video','vixen-core','vixen-effects']:
        cargo=cargo.replace('path = "../'+name+'"','path = '+json.dumps(str(REPO/'crates'/name)))
    for name in ['uuid','roxmltree','tiny-skia','image']:
        value=root[name];render=json.dumps(value) if isinstance(value,str) else '{ '+', '.join(k+' = '+json.dumps(v) for k,v in value.items())+' }'
        cargo=cargo.replace(name+'.workspace = true',name+' = '+render)
    cargo+='serde_json = "1"\n\n[workspace]\n'
    (private/'Cargo.toml').write_text(cargo)
    (private/'src/bin').mkdir(exist_ok=True)
    (private/'src/bin/gif_capture.rs').write_text('''fn main(){let a:Vec<_>=std::env::args().collect();let root=std::path::Path::new(&a[1]);let mut rows=Vec::new();for file in ["full.gif","keep.gif","background.gif","previous.gif"] {for speed in [1,3] {rows.push(fx_pixel_graphics::picture::capture_private_gif(root,file,speed));}} println!("{}",serde_json::to_string_pretty(&serde_json::json!({"schema":1,"platform":"macOS","scope":"Production decode and SetupRender cache only","cases":rows})).unwrap());}''')
    module=out/'module-data/Picture';module.mkdir(parents=True)
    for file in fixtures.glob('*.gif'):shutil.copyfile(file,module/file.name)
    manifest={'productionInputs':inputs,'privatePictureOriginalPrefixBytes':len(original),'privatePictureOriginalPrefixSha256':hashlib.sha256(original).hexdigest(),'privatePictureSha256':sha(private/'src/picture.rs'),'adapterSha256':hashlib.sha256(adapter).hexdigest(),'fixtures':{f.name:sha(f) for f in fixtures.iterdir() if f.is_file()},'scope':'Production source unchanged; appended private observer; no full effect-render proof'}
    (out/'prepared.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(str(private/'Cargo.toml'))
if __name__=='__main__':main()
