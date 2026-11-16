#!/usr/bin/env python3
"""Four tiny explicit GIF89a streams; no image editor/library optimization."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

PALETTE = [(17, 23, 31), (255, 0, 0), (0, 255, 0), (0, 0, 255)]
DELAYS = [1, 7, 13, 2]  # centiseconds; deliberately unequal


def lzw(indices):
    # Minimum code size2, Clear4, End5. Clearing before every literal keeps
    # every code exactly3bits and avoids a second implementation of LZW state.
    codes = [c for value in indices for c in (4, value)] + [5]
    value = sum(code << (3 * i) for i, code in enumerate(codes))
    data = value.to_bytes((len(codes) * 3 + 7) // 8, 'little')
    assert len(data) < 256
    return bytes([2, len(data)]) + data + b'\x00'


def build(frames):
    result = bytearray(b'GIF89a' + struct.pack('<HHBBB', 4, 4, 0xf1, 0, 0))
    result.extend(bytes(c for rgb in PALETTE for c in rgb))
    result.extend(b'\x21\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00')
    for index, frame in enumerate(frames):
        x, y, w, h, disposal, transparent, pixels = frame
        assert len(pixels) == w * h and x + w <= 4 and y + h <= 4
        result.extend(b'\x21\xf9\x04' + struct.pack('<BHB', disposal * 4 + int(transparent), DELAYS[index], 0) + b'\x00')
        result.extend(b'\x2c' + struct.pack('<HHHHB', x, y, w, h, 0))
        result.extend(lzw(pixels))
    result.append(0x3b)
    return bytes(result)


def frames(disposal=None):
    if disposal is None:
        return [(0, 0, 4, 4, 1, False, [color] * 16) for color in (1, 2, 3)] + [
            (0, 0, 4, 4, 1, False, [1, 2, 3, 1] * 4)]
    return [(0, 0, 4, 4, 1, True, [1] * 16),
            (1, 1, 2, 2, disposal, True, [2, 0, 0, 2]),
            (0, 0, 2, 2, 1, True, [3, 0, 0, 3]),
            (3, 3, 1, 1, 1, True, [2])]


def source_schedule(speed):
    # Independent description of pinned original CalculateImageNumberByPosition
    # and SetupRender's > currentImage guard, including the wrap quirk.
    previous = -1
    result = []
    for frame in range(12):
        selected = round(((frame * 50 / 600 * speed) % 1) * 3)
        if selected > previous:
            result.append(dict(effectFrame=frame, sourceFrame=selected))
        previous = selected
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    a.out.mkdir()
    records, cases = [], []
    for name, disposal in [('full', None), ('keep', 1), ('background', 2), ('previous', 3)]:
        f = frames(disposal)
        content = build(f)
        path = a.out / (name + '.gif')
        path.write_bytes(content)
        digest = hashlib.sha256(content).hexdigest()
        records.append(dict(file=path.name, bytes=len(content), sha256=digest, logicalSize=[4, 4],
                            globalPalette=PALETTE, backgroundIndex=0, transparentIndex=0 if disposal else None,
                            delaysCentiseconds=DELAYS, repeat=0,
                            frames=[dict(left=x, top=y, width=w, height=h, disposal=d, transparency=t, indices=px)
                                    for x, y, w, h, d, t, px in f]))
        for speed in (1, 3):
            cases.append(dict(id=name + '-speed' + str(speed), file=path.name, inputSha256=digest,
                              gifSpeed=speed, frameTimeMs=50, durationMs=600, effectFrames=12,
                              width=4, height=4, scalePercent=100, scaleToGrid=False, stretchToGrid=False,
                              expectedSourceSchedule=source_schedule(speed)))
    contract = dict(schema=1, byteOrder='A,R,G,B; top-down row-major', fixtures=records, cases=cases,
                    scope='GIF decode/compositing and original Picture SetupRender cache/schedule; full effect/GUI not claimed',
                    decodedPixelsSeparateFromCache=True, windowsParityVerified=False)
    (a.out / 'cases.json').write_text(json.dumps(contract, indent=2) + '\n')
    print(json.dumps(dict(fixtures=len(records), cases=len(cases), output=str(a.out))))


if __name__ == '__main__':
    main()
