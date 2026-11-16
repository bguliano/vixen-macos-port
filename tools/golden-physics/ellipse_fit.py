import math
def rot(angle, cx, cy):
    r = (math.fmod(angle, 360.0)) * (math.pi/180.0)
    s, c = math.sin(r), math.cos(r)
    return s, c, cx*(1.0-c)+cy*s, cy*(1.0-c)-cx*s
def apply(R, p):
    s,c,dx,dy = R
    xa = p[1]*-s + dx
    ya = p[0]*s + dy
    return [p[0]*c + xa, p[1]*c + ya]
def outside(pts, w, h):
    return any(p[0] > w-1 or p[1] > h-1 or p[0] < 0 or p[1] < 0 for p in pts)
def scale(pts, cx, cy, W, H, angle, xs, ys):
    R = rot(-angle, cx, cy)
    pts = [apply(R,p) for p in pts]
    pts = [[p[0]*xs, p[1]*ys] for p in pts]
    cx*=xs; cy*=ys; W*=xs; H*=ys
    R = rot(angle, cx, cy)
    pts = [apply(R,p) for p in pts]
    return pts,cx,cy,W,H
pts=[[-1.0,3.0],[9.0,3.0],[9.0,7.0],[-1.0,7.0]]
cx,cy,W,H,a=4.0,5.0,10.0,4.0,30.0
pts,cx,cy,W,H=scale(pts,cx,cy,W,H,a,1.0,1.0)
print(pts,cx,cy)
n=0
w,h=20,20
while outside(pts,w,h) and n<2000000:
    pts,cx,cy,W,H=scale(pts,cx,cy,W,H,a,(w-1.0)/w,0.0)
    n+=1
    if n<4 or n%2000==0: print(n,pts,cx,cy,W,H)
print("passes",n,pts,cx,cy,W,H, outside(pts,w,h))
print(14/200.0*150, 14/200.0*150/10*10, 0.01*150, 10/1000*500, 10/1000*850)
