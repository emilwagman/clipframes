# Rounds every corner of the clipped-frame mark. Usage: round.py R_OUTER R_INNER R_PIECE -> prints the SVG path data.
import math, sys
def rounded(pts, r):
    n = len(pts); out = []
    for i in range(n):
        A, P, B = pts[i - 1], pts[i], pts[(i + 1) % n]
        ua = ((A[0]-P[0]), (A[1]-P[1])); ub = ((B[0]-P[0]), (B[1]-P[1]))
        la, lb = math.hypot(*ua), math.hypot(*ub); ua = (ua[0]/la, ua[1]/la); ub = (ub[0]/lb, ub[1]/lb)
        ang = math.acos(max(-1, min(1, ua[0]*ub[0] + ua[1]*ub[1])))
        d = min(r / math.tan(ang / 2), la / 2.05, lb / 2.05); rr = d * math.tan(ang / 2)
        t1 = (P[0] + ua[0]*d, P[1] + ua[1]*d); t2 = (P[0] + ub[0]*d, P[1] + ub[1]*d)
        out.append(("L" if out else "M") + f"{t1[0]:.2f} {t1[1]:.2f}A{rr:.2f} {rr:.2f} 0 0 1 {t2[0]:.2f} {t2[1]:.2f}")
    return "".join(out) + "Z"
ro, ri, rp = map(float, sys.argv[1:4])
frame = [(4, 10.2), (76, 10.2), (104, 38.2), (104, 110.2), (4, 110.2)]
hole = [(21, 27.2), (87, 27.2), (87, 93.2), (21, 93.2)]
piece = [(83.6, 4), (110, 4), (110, 30.4)]
print(rounded(frame, ro) + rounded(hole, ri) + "|" + rounded(piece, rp))
