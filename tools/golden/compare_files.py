#!/usr/bin/env python3
"""Compares what the original Vixen wrote for the original files and for the files the Rust port saved. Reports differences by XML path."""
import collections
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def local(tag):
    return tag.split('}')[-1]


# Members that hold where this test happened to run.
IGNORED = ('FilePath', 'RelativeAudioPath')
# Children that identify a repeated element, so a change of order isn't reported as a change of content.
IDENTITY = ('InstanceId', 'ModuleInstanceId', 'Id')


def identity(element):
    if 'moduleInstance' in element.attrib:
        return element.attrib['moduleType'] + '/' + element.attrib['moduleInstance']
    for child in element:
        if local(child.tag) in IDENTITY and (child.text or '').strip():
            return child.text.strip()
    return None


def flatten(element, path, out):
    """Path and text of every element, with an identity or a position where a name repeats, and attributes that matter."""
    counts = collections.Counter(local(c.tag) for c in element)
    seen = collections.Counter()
    text = (element.text or '').strip()
    attrs = sorted((local(k), v) for k, v in element.attrib.items() if local(k) not in ('Id', 'Ref'))
    if local(element.tag) not in IGNORED:
        out.append((path, text, tuple(attrs)))
    for child in element:
        name = local(child.tag)
        suffix = ''
        if counts[name] > 1:
            suffix = f'[{identity(child) or seen[name]}]'
        seen[name] += 1
        flatten(child, f'{path}/{name}{suffix}', out)


def same(a, b):
    """Numbers are compared as numbers, because the original writes doubles with shortest round-trip digits."""
    if a == b:
        return True
    if a[1] != b[1]:
        return False
    try:
        return float(a[0]) == float(b[0])
    except ValueError:
        return False


def diff(a_path, b_path, limit=12):
    a, b = [], []
    flatten(ET.parse(a_path).getroot(), '', a)
    flatten(ET.parse(b_path).getroot(), '', b)
    a_map, b_map = {p: (t, at) for p, t, at in a}, {p: (t, at) for p, t, at in b}
    only_a = [p for p, _, _ in a if p not in b_map]
    only_b = [p for p, _, _ in b if p not in a_map]
    changed = [p for p, _, _ in a if p in b_map and not same(a_map[p], b_map[p])]
    return len(a), only_a, only_b, changed, a_map, b_map


def report(title, a_path, b_path):
    if a_path.read_bytes() == b_path.read_bytes():
        print(f'{title}: byte-identical')
        return True
    total, only_a, only_b, changed, a_map, b_map = diff(a_path, b_path)
    if not (only_a or only_b or changed):
        print(f'{title}: same XML ({total} elements), different formatting')
        return True
    print(f'{title}: {len(only_a)} only in first, {len(only_b)} only in second, {len(changed)} changed, of {total}')
    def generalise(p):
        return re.sub(r'\[[^\]]*\]', '[]', p)
    for label, paths in (('only in first', only_a), ('only in second', only_b), ('changed', changed)):
        groups = collections.Counter(generalise(p) for p in paths)
        for group, n in groups.most_common(8):
            example = next(p for p in paths if generalise(p) == group)
            detail = ''
            if label == 'changed':
                detail = f'  {a_map[example]!r:.70} -> {b_map[example]!r:.70}'
            print(f'    {label}: x{n} {group[-110:]}{detail}')
    return False


# In the changed copies of the effects the port built, values the original's load logic changes. The port does the same in memory
# but keeps the text as it was until the effect is edited: EnableAudio and PinWheel3D are old members that move to another, and the
# others are out of range or don't apply. Everything else must come back as written.
EXPECTED_AFTER_RELOAD = ('/EnableAudio', '/PinWheel3D', '/FireworksSource', '/EnableGroundLevel', '/MaximumLevel')


def main():
    root = Path(sys.argv[1])
    ok = True
    print('-- What the original wrote for each original file against what it wrote for the file the port saved')
    names = sorted(p.name for p in (root / 'out-port').glob('*.tim'))
    originals = sorted(p.name for p in (root / 'out-original').glob('*.tim'))
    print(f'{len(originals)} and {len(names)} sequences loaded')
    ok &= len(originals) == 17 and names == originals
    for name in originals:
        a = (root / 'out-original' / name).read_bytes().replace(b'target/filecheck/original', b'X')
        b = (root / 'out-port' / name).read_bytes().replace(b'target/filecheck/port', b'X')
        if a == b:
            print(f'{name}: byte-identical apart from the folder name')
        else:
            ok &= report(name, root / 'out-original' / name, root / 'out-port' / name)

    print('-- Differences between the file the port saved and what the original then wrote for it, for reference')
    print('   (the original adds members that older files leave out)')
    for name in names:
        report(name, root / 'port-raw/Sequence' / name, root / 'out-port' / name)

    print('-- Effects the port built, as it reloads them, against what the original wrote for them')
    total, only_port, only_original, changed, _, _ = diff(root / 'new-reloaded.tim', root / 'out-new/every-effect.tim')
    unexpected = [p for p in only_port + only_original + changed if not p.endswith(EXPECTED_AFTER_RELOAD)]
    print(f'{total} elements, {len(only_port) + len(only_original) + len(changed)} differ, {len(unexpected)} unexpectedly')
    for p in unexpected[:30]:
        print('   ', p[-120:])
    ok &= not unexpected

    print('-- System files the original wrote, from the original profile against from the port-saved profile')
    for name in ('SystemConfig.xml', 'ModuleStore.xml'):
        ok &= report(name, root / 'original/SystemData' / name, root / 'port/SystemData' / name)
    print('-- System files as read from the show against as the port saved them')
    for name in ('SystemConfig.xml', 'ModuleStore.xml'):
        ok &= report(name, root / '../show-copy/SystemData' / name, root / 'port-raw' / name)
    print('-- Sizes, to show how much of each file the original could load')
    for name in ('SystemConfig.xml', 'ModuleStore.xml'):
        sizes = [(root / folder / name).stat().st_size for folder in ('../show-copy/SystemData', 'port-raw', 'port/SystemData')]
        print(f'{name}: show {sizes[0]}, port saved {sizes[1]}, original then wrote {sizes[2]}')
    print('PASSED' if ok else 'FAILED')
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
