# The mark with a deeper clip that opens into the window, every corner rounded (B: medium).
# Usage: deep.py LEG -> "frame path|piece path" in the 114-unit box.
import math, sys
def rounded(pts, r):
    n = len(pts); out = []
    for i in range(n):
        A, P, B = pts[i - 1], pts[i], pts[(i + 1) % n]
        ua = (A[0]-P[0], A[1]-P[1]); ub = (B[0]-P[0], B[1]-P[1])
        la, lb = math.hypot(*ua), math.hypot(*ub); ua = (ua[0]/la, ua[1]/la); ub = (ub[0]/lb, ub[1]/lb)
        ang = math.acos(max(-1, min(1, ua[0]*ub[0] + ua[1]*ub[1])))
        d = min(r / math.tan(ang / 2), la / 2.05, lb / 2.05); rr = d * math.tan(ang / 2)
        t1 = (P[0] + ua[0]*d, P[1] + ua[1]*d); t2 = (P[0] + ub[0]*d, P[1] + ub[1]*d)
        cross = (P[0]-A[0])*(B[1]-P[1]) - (P[1]-A[1])*(B[0]-P[0])
        sweep = 1 if cross > 0 else 0
        out.append(("L" if out else "M") + f"{t1[0]:.2f} {t1[1]:.2f}A{rr:.2f} {rr:.2f} 0 0 {sweep} {t2[0]:.2f} {t2[1]:.2f}")
    return "".join(out) + "Z"
L = float(sys.argv[1]); s, w = 100, 17            # frame size and wall
c = s - L                                           # the cut line: x - y = c (frame units, top-left origin)
ox, oy = 4, 10.2                                    # into the 114 box
if c > s - w - w:                                   # cut stays in the wall: frame and window separate
    frame = rounded([(0,0),(c,0),(s,s-c),(s,s),(0,s)], 8) + rounded([(w,w),(s-w,w),(s-w,s-w),(w,s-w)], 4)
    pts_shift = None
else:                                               # cut opens into the window: one C-shaped outline
    frame = rounded([(0,0),(c,0),(c+w,w),(w,w),(w,s-w),(s-w,s-w),(s-w,s-w-c),(s,s-c),(s,s),(0,s)], 6)
gap = 9.8 * math.sqrt(2); Lp = L * 0.94; ty = -6.2; tx = c + gap + Lp + ty
piece = rounded([(tx-Lp,ty),(tx,ty),(tx,ty+Lp)], 4)
def shift(d):
    import re
    nums = re.split(r"([MLAZ])", d); out=[]; cmd=None
    for tok in nums:
        if tok in "MLAZ" and tok: cmd=tok; out.append(tok); continue
        v=tok.split()
        if not v: continue
        if cmd in "ML": out.append(f"{float(v[0])+ox:.2f} {float(v[1])+oy:.2f}")
        elif cmd=="A": out.append(" ".join(v[:5])+f" {float(v[5])+ox:.2f} {float(v[6])+oy:.2f}")
    return "".join(out)
print(shift(frame) + "|" + shift(piece))
