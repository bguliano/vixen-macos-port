#!/usr/bin/env python3
"""Compare original C# and Rust meter levels over identical real decoded PCM."""
import csv
import math
from pathlib import Path
import sys

root = Path(sys.argv[1] if len(sys.argv) > 1 else "target/vertical-meter")
effect = "f234e165-a96c-4ff1-8187-f549bc990c00"
original = list(csv.reader((root / "csharp-levels.csv").open()))
ported = list(csv.reader((root / f"{effect}-levels.csv").open()))
assert len(original) == len(ported) == 104
maximum = 0.0
for left, right in zip(original, ported):
    assert left[:2] == right[:2], (left, right)
    a, b = float(left[2]), float(right[2])
    assert math.isfinite(a) and math.isfinite(b), (left, right)
    maximum = max(maximum, abs(a - b))
assert maximum == 0.0, maximum
print(f"PASS: 104 C# / Rust volume samples and indices identical; max delta {maximum} dB")
