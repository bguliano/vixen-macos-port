//! Nominal stroke kernel. Opaque, aliased, solid pens only.
//!
//! Adapted from ChristopherVR/emf-converter, Apache-2.0:
//! src/emf-plus-nominal-line.ts and src/emf-plus-bezier.ts.
//! Copyright 2025-present pptx-viewer contributors.
//! License and adaptation notice: app/Resources/Licenses/emf-converter*.txt.
//! Primary source URLs and hashes are retained with the verification evidence.
//! This Rust adaptation uses checked input limits, bounded raster extents and
//! exact integer scan intersections; it does not claim general GDI+ parity.

type Fixed = (i64, i64);
pub type Pixel = (i32, i32);
const LIMIT: f32 = 1_048_576.0;

fn fixed(v: f32) -> Option<i64> {
    if !v.is_finite() || v.abs() > LIMIT { return None; }
    Some((((v * 256.0 + 0.5).floor() as i64) + 15) >> 4)
}
fn point(p: (f32, f32)) -> Option<Fixed> { Some((fixed(p.0)?, fixed(p.1)?)) }
fn ceil_div(n: i64, d: i64) -> i64 { -(-n).div_euclid(d) }
fn valid_bounds(w: i32, h: i32) -> bool { w > 0 && h > 0 && w <= LIMIT as i32 && h <= LIMIT as i32 }

pub fn line(a: (f32, f32), b: (f32, f32), w: i32, h: i32) -> Option<Vec<Pixel>> {
    let (a,b)=(point(a)?,point(b)?);
    if w <= 0 || h <= 0 { return Some(Vec::new()); }
    if !valid_bounds(w,h) { return None; }
    let inside = ((a.0.min(b.0)-8).div_euclid(16)-1)>=0
        && ((a.1.min(b.1)-8).div_euclid(16)-1)>=0
        && ceil_div(a.0.max(b.0)+8,16)+1<=w as i64-1
        && ceil_div(a.1.max(b.1)+8,16)+1<=h as i64-1;
    Some(dda(a,b,w,h,!inside))
}

fn dda(mut a: Fixed, mut b: Fixed, w: i32, h: i32, clipped: bool) -> Vec<Pixel> {
    let dx=(b.0-a.0) as f32/16.0;
    let dy=(b.1-a.1) as f32/16.0;
    if dx==0.0 && dy==0.0 { return Vec::new(); }
    let xmajor=dy.abs()<dx.abs();
    if if xmajor { dx<0.0 } else { dy<0.0 } { std::mem::swap(&mut a,&mut b); }
    let (mut j0,mut j1,mut n0,mut n1)=if xmajor {(a.0,b.0,a.1,b.1)} else {(a.1,b.1,a.0,b.0)};
    let sign=if n1>=n0 {1i64} else {-1};
    let dj=j1-j0; let dn=(n1-n0)*sign;
    let slope=(n1-n0) as f32/dj as f32;
    let (mut frac0,mut frac1)=(0.0f32,0.0f32);
    let mut e0=0i64;
    if clipped {
        let inv=if slope==0.0 {0.0} else {1.0/slope};
        let (jlo,jhi)=(-8,if xmajor {w as i64*16-8} else {h as i64*16-8});
        let (nlo,nhi)=(-8,if xmajor {h as i64*16-8} else {w as i64*16-8});
        if j0<jlo || j1>jhi {
            if j0>jhi || j1<jlo { return Vec::new(); }
            if j0<jlo {
                let v=(jlo-j0) as f32*slope+n0 as f32;
                n0=v.floor() as i64; j0=jlo; frac0=v-n0 as f32;
            }
            if j1>jhi {
                let v=(jhi-j1) as f32*slope+n1 as f32;
                n1=v.floor() as i64; j1=jhi; frac1=v-n1 as f32;
            }
        }
        let low_start=sign==1;
        let lo=if low_start {n0} else {n1};
        let hi=if low_start {n1} else {n0};
        if lo<nlo || hi>nhi {
            if lo>nhi || hi<nlo {return Vec::new();}
            if lo<nlo {
                let t=((nlo as f32-(lo as f32+if low_start {frac0} else {frac1}))*inv).floor() as i64;
                if low_start {j0+=t;n0=nlo;} else {j1+=t;n1=nlo;}
            }
            if hi>nhi {
                let t=((nhi as f32-(hi as f32+if low_start {frac1} else {frac0}))*inv).floor() as i64;
                if low_start {j1+=t;n1=nhi;} else {j0+=t;n0=nhi;}
            }
        }
        if frac1!=0.0 && n1&15==8 {n1+=1;}
        if frac0!=0.0 {e0=((2*dj*sign) as f32*frac0).floor() as i64;}
    }
    let diagonal=xmajor && dj==dn;
    let tie_up=diagonal && sign==1;
    let diamond=|x:i64,y:i64| {
        let distance=x.abs()+y.abs();
        distance<8 || (distance==8 && ((y==0 && x==if tie_up {-8} else {8})
            || (x==0 && y==8) || (diagonal && (if tie_up {x<0} else {x>0}) && y>0)))
    };
    let k=if tie_up {8} else {7};
    let (mut start,mut end)=((j0+k)&!15,(j1+k)&!15);
    if j0&15<=8 && !diamond(j0-start,n0-((n0+7)&!15)) {start+=16;}
    if j1&15>8 && !diamond(j1-end,n1-((n1+7)&!15)) {end-=16;}
    let first=start>>4; let last=end>>4;
    if last<first {return Vec::new();}
    // The retained native traces reject rounding this product/sum to f32.
    let initial=((start-j0) as f64*slope as f64+n0 as f64+frac0 as f64).floor() as i64;
    let center=(initial+7)&!15; let mut minor=center>>4;
    let mut error=(e0-((center-initial)*sign+8)*2*dj)>>4;
    let end_minor=(((end-j1) as f64*slope as f64+n1 as f64+frac1 as f64).floor() as i64+7)>>4;
    let mut left=(end_minor-minor)*sign;
    let major_limit=if xmajor {w as i64} else {h as i64};
    // Clipping must bound iteration even for a segment far outside the bitmap.
    if last-first>major_limit+2 {return Vec::new();}
    let mut out=Vec::new();
    for major in first..=last {
        if clipped && left<0 {break;}
        let (x,y)=if xmajor {(major,minor)} else {(minor,major)};
        if x>=0 && y>=0 && x<w as i64 && y<h as i64 {out.push((x as i32,y as i32));}
        error+=2*dn;
        if error>0 {minor+=sign;error-=2*dj;left-=1;}
    }
    out
}

fn flatten(p: &[Fixed], out: &mut Vec<Fixed>) -> Option<()> {
    let left=p.iter().map(|p|p.0).min()? -16;
    let top=p.iter().map(|p|p.1).min()? -16;
    let mut es=[[0i64;4];2];
    for (axis,origin) in [left,top].into_iter().enumerate() {
        let v:Vec<_>=p.iter().map(|p|if axis==0 {p.0-origin} else {p.1-origin}).collect();
        es[axis]=[v[0]*1024,(v[3]-v[0])*1024,6*(v[1]-2*v[2]+v[3])*1024,6*(v[0]-2*v[1]+v[2])*1024];
    }
    let error=|es:&[[i64;4];2]|es.iter().flat_map(|e|[e[2].abs(),e[3].abs()]).max().unwrap();
    let (mut steps,mut shift)=(1usize,0u32);
    while error(&es)>0x6000i64.checked_shl(shift)? {
        shift+=2;
        if shift>=40 {return None;}
        for e in &mut es {e[2]=(e[2]+e[3])>>1;e[1]=(e[1]-(e[2]>>shift))>>1;}
        steps*=2;
    }
    for e in &mut es {
        e[0]*=8;e[1]*=8;
        for i in [2,3] {e[i]=if shift>=3 {e[i]>>(shift-3)} else {e[i]<<(3-shift)};}
    }
    let step=|e:&mut [i64;4]| {e[0]+=e[1];let t=e[2];e[1]+=t;e[2]=2*t-e[3];e[3]=t;};
    for e in &mut es {step(e);} steps-=1;
    for _ in 0..4096 {
        out.push((((es[0][0]+0x1000)>>13)+left,((es[1][0]+0x1000)>>13)+top));
        if steps==0 {return Some(());}
        if error(&es)>0x30000 {
            for e in &mut es {e[2]=(e[2]+e[3])>>3;e[1]=(e[1]-e[2])>>1;e[3]>>=2;}
            steps*=2;
        }
        while steps%2==0 && es.iter().all(|e|e[3].abs().max((2*e[2]-e[3]).abs())<=0xc000) {
            for e in &mut es {e[3]*=4;e[1]=e[2]+2*e[1];e[2]=8*e[2]-e[3];}
            steps/=2;
        }
        steps-=1;for e in &mut es {step(e);}
    }
    None
}

const DIAMOND:[Fixed;4]=[(0,-8),(-8,0),(0,8),(8,0)];
fn quadrant(a:Fixed,b:Fixed)->usize {
    let (x,y)=(b.0-a.0,b.1-a.1);
    if y.abs()>x.abs() {if y<0 {1} else {3}} else if x<0 {2} else {0}
}
fn offset(p:Fixed,q:usize,sign:i64)->Fixed {let d=DIAMOND[q%4];(p.0+sign*d.0,p.1+sign*d.1)}
fn outline(p:&[Fixed])->Vec<Fixed> {
    let mut q=quadrant(p[0],p[1]);
    let mut front=vec![offset(p[0],q,-1),offset(p[0],q+1,1)];let mut back=Vec::new();
    for (i,ab) in p.windows(2).enumerate() {
        let (a,b)=(ab[0],ab[1]);
        front.extend([offset(a,q,1),offset(b,q,1)]);back.extend([offset(a,q,-1),offset(b,q,-1)]);
        if i+2==p.len() {break;}
        let c=p[i+2];let nq=quadrant(b,c);
        if nq!=q {
            if (b.0-a.0)*(c.1-b.1)>=(b.1-a.1)*(c.0-b.0) {
                if (q+3)%4!=nq {front.push(offset(b,q+3,1));}back.push(b);
            } else {
                if (q+1)%4!=nq {back.push(offset(b,q+1,-1));}front.push(b);
            }
            q=nq;
        }
    }
    let last=*p.last().unwrap();front.extend([offset(last,q+3,1),offset(last,q,-1)]);
    front.extend(back.into_iter().rev());front
}

fn fill(poly:&[Fixed],w:i32,h:i32,out:&mut Vec<Pixel>) {
    let first=ceil_div(poly.iter().map(|p|p.1).min().unwrap(),16).max(0);
    let end=ceil_div(poly.iter().map(|p|p.1).max().unwrap(),16).min(h as i64);
    let mut crossings=Vec::with_capacity(poly.len());
    for y in first..end {
        crossings.clear();let fy=y*16;
        for (&a,&b) in poly.iter().zip(poly.iter().cycle().skip(1)) {
            if a.1.min(b.1)<=fy && fy<a.1.max(b.1) {
                let sign=if b.1>a.1 {1i64} else {-1};
                let den=(b.1-a.1)*sign;
                let num=(a.0*(b.1-a.1)+(fy-a.1)*(b.0-a.0))*sign;
                crossings.push((num,den,sign));
            }
        }
        crossings.sort_by(|a,b|((a.0 as i128)*(b.1 as i128)).cmp(&((b.0 as i128)*(a.1 as i128))));
        let mut winding=0i64;let mut previous=(0i64,1i64);
        for &(num,den,direction) in &crossings {
            if winding!=0 {
                let left=ceil_div(previous.0,previous.1*16).clamp(0,w as i64);
                let right=ceil_div(num,den*16).clamp(0,w as i64);
                out.extend((left..right).map(|x|(x as i32,y as i32)));
            }
            winding+=direction;previous=(num,den);
        }
    }
}

pub fn ellipse_points(x:f32,y:f32,w:f32,h:f32)->[(f32,f32);13] {
    let (rx,ry)=(w*0.5,h*0.5);let (cx,cy)=(x+rx,y+ry);let k=0.552_284_749_830_793_6f32;
    let (dx,dy)=(rx*k,ry*k);
    [(cx+rx,cy),(cx+rx,cy+dy),(cx+dx,cy+ry),(cx,cy+ry),(cx-dx,cy+ry),
     (cx-rx,cy+dy),(cx-rx,cy),(cx-rx,cy-dy),(cx-dx,cy-ry),(cx,cy-ry),
     (cx+dx,cy-ry),(cx+rx,cy-dy),(cx+rx,cy)]
}

pub fn curved_path(points:&[(f32,f32)],w:i32,h:i32)->Option<Vec<Pixel>> {
    if points.len()!=13 || !valid_bounds(w,h) {return None;}
    let fixed:Vec<_>=points.iter().map(|&p|point(p)).collect::<Option<_>>()?;
    let mut flat=vec![fixed[0]];
    for start in [0,3,6,9] {flatten(&fixed[start..start+4],&mut flat)?;}
    if flat.last()!=flat.first() {flat.push(flat[0]);}
    if flat.iter().all(|p|*p==flat[0]) {return Some(Vec::new());}
    let mut out=Vec::new();
    for start in (0..flat.len()-1).step_by(31) {
        fill(&outline(&flat[start..(start+32).min(flat.len())]),w,h,&mut out);
    }
    Some(out)
}
