#!/usr/bin/env python3
"""Copy a built app into a unique Cocoa preferences domain, without launching it.

Rust settings still require VIXEN_UI_TEST + VIXEN_UI_TEST_OUT, or an explicit
VIXEN_SETTINGS_DIR for normal (non-harness) UI verification. No owner preferences
are read, copied, cleared or changed by this preparation tool.
"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def code_digest(path):
    """Hash every byte before the signature, normalizing only signature sizes.

    Re-signing changes LC_CODE_SIGNATURE and the __LINKEDIT allocation sizes.
    All other Mach-O load commands, code, data and link-edit bytes must match.
    """
    raw = path.read_bytes()
    assert raw[:4] == b'\xcf\xfa\xed\xfe', 'Expected thin little-endian Mach-O64'
    ncmds, sizeofcmds = struct.unpack_from('<II', raw, 16)
    position = 32
    normalized = bytearray(raw)
    signature = None
    for _ in range(ncmds):
        command, size = struct.unpack_from('<II', raw, position)
        assert size >= 8 and position + size <= 32 + sizeofcmds <= len(raw)
        if command == 0x1D:  # LC_CODE_SIGNATURE
            assert signature is None and size == 16
            offset, length = struct.unpack_from('<II', raw, position + 8)
            assert offset >= 32 + sizeofcmds and offset + length <= len(raw)
            assert not any(raw[offset + length:]), 'Unexpected bytes after code signature'
            signature = (offset, length)
            # Keep the offset: a moved code/data boundary is not a signing-only change.
            normalized[position + 12:position + 16] = b'\0' * 4
        elif command == 0x19:  # LC_SEGMENT_64
            assert size >= 72
            if raw[position + 8:position + 24].rstrip(b'\0') == b'__LINKEDIT':
                normalized[position + 32:position + 40] = b'\0' * 8
                normalized[position + 48:position + 56] = b'\0' * 8
        position += size
    assert position == 32 + sizeofcmds and signature is not None
    return hashlib.sha256(normalized[:signature[0]]).hexdigest()


def tree_hashes(app):
    return {str(p.relative_to(app)): digest(p) for p in sorted(app.rglob('*')) if p.is_file()}


def prepare(source, out, *, output_root=None):
    source, out = source.resolve(), out.resolve()
    assert source.is_relative_to(ROOT) and source.name.endswith('.app') and source.is_dir()
    # Portable packs keep their input tree immutable. Their runner can supply
    # its already-created private results directory; CLI callers retain the
    # checkout's target-only boundary.
    allowed_root = ROOT / 'target' if output_root is None else Path(output_root)
    assert allowed_root.is_absolute() and allowed_root.is_dir(), 'Output root must be an existing absolute directory'
    allowed_root = allowed_root.resolve()
    assert not allowed_root.is_relative_to(source), 'Output root must not be inside the source app'
    assert out != allowed_root and out.is_relative_to(allowed_root) and not out.exists()
    assert not out.is_relative_to(source)
    before = tree_hashes(source)
    source_info = plistlib.loads((source / 'Contents/Info.plist').read_bytes())
    executable = 'Contents/MacOS/' + source_info['CFBundleExecutable']
    identifier = 'com.vixenlights.verification.' + uuid.uuid4().hex
    out.mkdir(parents=True)
    app = out / 'Vixen.app'
    shutil.copytree(source, app, symlinks=True)
    info = dict(source_info)
    info['CFBundleIdentifier'] = identifier
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(info, sort_keys=False))
    subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                    '--identifier', identifier, str(app)], check=True, capture_output=True, timeout=30)
    subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)],
                   check=True, capture_output=True, timeout=30)
    after = tree_hashes(app)
    assert set(before) == set(after), 'Unexpected copied bundle files'
    changed = [name for name in before if before[name] != after[name]]
    assert set(changed) <= {'Contents/Info.plist', executable, 'Contents/_CodeSignature/CodeResources'}, changed
    assert code_digest(source / executable) == code_digest(app / executable), 'Executable code/data changed'
    assert tree_hashes(source) == before, 'Source bundle changed'
    report = {
        'passed': True, 'launched': False, 'sourceApp': str(source), 'app': str(app),
        'sourceBundleIdentifier': source_info['CFBundleIdentifier'], 'bundleIdentifier': identifier,
        'sourceExecutableSha256': before[executable], 'executableSha256': after[executable],
        'executableCodeDigest': code_digest(app / executable), 'sourceUnchanged': True,
        'fileCount': len(before), 'changedFiles': changed, 'signatureVerified': True,
        'outputRoot': str(allowed_root),
        'scope': 'Unique Cocoa application-preferences domain; only bundle identifier and code signatures changed. Rust settings isolation is a separate required launch setting.',
    }
    (out / 'private-app.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    options = parser.parse_args()
    print(json.dumps(prepare(options.app, options.out), indent=2))
