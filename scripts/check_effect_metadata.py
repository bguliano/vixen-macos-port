#!/usr/bin/env python3
"""Checks the effect editor's tables against the C# they were ported from.

It re-parses every effect's C# module class, its base classes, its data class and the three effect resource files,
works out what the WPF property grid would show, and compares that with the Rust tables, which it gets by running
`crates/vixen-effect-registry/examples/dump_effect_tables.rs`. Nothing here runs or trusts the old generator.

    scripts/check_effect_metadata.py              # build the dump, compare, print a summary
    scripts/check_effect_metadata.py -v           # also print every accepted difference with its reason
    scripts/check_effect_metadata.py --tables F   # compare against a saved dump

It exits with 1 if it finds anything that isn't on the ACCEPTED list below, each entry of which says why it differs.
"""
import argparse
import collections
import json
import math
import os
import pathlib
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "upstream/Vixen/src"
EFFECTS = SRC / "Vixen.Modules/Effect"
ATTR_DIR = SRC / "Vixen.Modules/EffectEditor/EffectDescriptorAttributes"
CRATES = ROOT / "crates"

# The classes of the C# effects whose names can't be found from the descriptor's ModuleClass alone.
EXTRA_SEARCH = [EFFECTS, SRC / "Vixen.Core", SRC / "Vixen.Modules/App/Polygon", SRC / "Vixen.Modules/App/Fixture", SRC / "Vixen.Modules/App/LipSyncApp", SRC / "Vixen.Modules/Property"]

# A table row whose name is the data member where the C# property has another name. (effect, Rust name) -> C# property.
PROPERTY_ALIASES = {
    ("Morph", "MorphPolygonData"): "MorphPolygons",
    ("Whirlpool", "WhirlData"): "Whirls",
    ("Wave", "WaveformData"): "Waves",
    ("Liquid", "EmitterData"): "EmitterList",
    ("Fixture", "FunctionData"): "Functions",
    ("Liquid/EmitterData", "MarkCollectionId"): "MarkCollectionName",
    ("Wave/WaveformData", "MarkCollectionId"): "MarkCollectionName",
    ("State/CustomStateItemData", "StateItemId"): "StateItem",
}

# The C# class whose attributes describe the rows of a list parameter, by the record's contract name.
ROW_CLASSES = {
    "WaveformData": "Waveform",
    "EmitterData": "Emitter",
    "WhirlData": "Whirl",
    "MorphPolygonData": "MorphPolygon",
    "FixtureFunctionData": "FixtureFunctionExpando",
    "CustomStateItemData": "CustomStateItem",
}

# (effect, name, check) -> why the table deliberately differs. `*` as a check accepts all of a row's differences.
ACCEPTED = {
    ("State", "Iterations", "default"): "the member's getter clamps 0 to 1, so the value a new effect shows and saves is 1",
    ("Custom Value", "Value64Bit", "kind"): "a ulong can exceed ParamKind::Long, so it stays unmodelled text that survives a load and save",
    ("Fixture/FixtureFunctionData", "Intensity", "missing"): "the row's Intensity curve is stored as Range by UpdateModelData, so the one Range row stands for both",
}

KNOWN_COLORS = {
    "Empty": (0, 0, 0, 0), "Transparent": (0, 255, 255, 255),
    "AliceBlue": (255, 240, 248, 255), "Aqua": (255, 0, 255, 255), "Black": (255, 0, 0, 0), "Blue": (255, 0, 0, 255),
    "BlueViolet": (255, 138, 43, 226), "Brown": (255, 165, 42, 42), "Coral": (255, 255, 127, 80),
    "CornflowerBlue": (255, 100, 149, 237), "Crimson": (255, 220, 20, 60), "Cyan": (255, 0, 255, 255),
    "DarkBlue": (255, 0, 0, 139), "DarkGray": (255, 169, 169, 169), "DarkGreen": (255, 0, 100, 0),
    "DarkOrange": (255, 255, 140, 0), "DarkRed": (255, 139, 0, 0), "DeepPink": (255, 255, 20, 147),
    "DeepSkyBlue": (255, 0, 191, 255), "DimGray": (255, 105, 105, 105), "DodgerBlue": (255, 30, 144, 255),
    "ForestGreen": (255, 34, 139, 34), "Gold": (255, 255, 215, 0), "Gray": (255, 128, 128, 128),
    "Green": (255, 0, 128, 0), "GreenYellow": (255, 173, 255, 47), "HotPink": (255, 255, 105, 180),
    "Indigo": (255, 75, 0, 130), "LawnGreen": (255, 124, 252, 0), "LightBlue": (255, 173, 216, 230),
    "LightGray": (255, 211, 211, 211), "LightGreen": (255, 144, 238, 144), "LightYellow": (255, 255, 255, 224),
    "Lime": (255, 0, 255, 0), "LimeGreen": (255, 50, 205, 50), "Magenta": (255, 255, 0, 255), "Maroon": (255, 128, 0, 0),
    "Navy": (255, 0, 0, 128), "Olive": (255, 128, 128, 0), "Orange": (255, 255, 165, 0), "OrangeRed": (255, 255, 69, 0),
    "Orchid": (255, 218, 112, 214), "Pink": (255, 255, 192, 203), "Purple": (255, 128, 0, 128), "Red": (255, 255, 0, 0),
    "RoyalBlue": (255, 65, 105, 225), "SeaGreen": (255, 46, 139, 87), "Silver": (255, 192, 192, 192),
    "SkyBlue": (255, 135, 206, 235), "Snow": (255, 255, 250, 250), "SteelBlue": (255, 70, 130, 180),
    "Teal": (255, 0, 128, 128), "Tomato": (255, 255, 99, 71), "Turquoise": (255, 64, 224, 208),
    "Violet": (255, 238, 130, 238), "White": (255, 255, 255, 255), "Yellow": (255, 255, 255, 0),
    "YellowGreen": (255, 154, 205, 50), "LightSalmon": (255, 255, 160, 122), "Salmon": (255, 250, 128, 114),
}


# ---------------------------------------------------------------------------------------------- C# text

def read(path):
    return path.read_bytes().decode("utf-8-sig", errors="replace").replace("\r", "")


def strip_comments(text):
    """Removes comments and #region style lines, leaving strings alone."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == "/" and i + 1 < n and text[i + 1] == "/":
            while i < n and text[i] != "\n":
                i += 1
        elif c == "/" and i + 1 < n and text[i + 1] == "*":
            j = text.find("*/", i + 2)
            i = n if j < 0 else j + 2
        elif c == '"':
            verbatim = i > 0 and text[i - 1] == "@"
            j = i + 1
            while j < n:
                if verbatim and text[j] == '"' and j + 1 < n and text[j + 1] == '"':
                    j += 2
                elif not verbatim and text[j] == "\\":
                    j += 2
                elif text[j] == '"':
                    break
                else:
                    j += 1
            out.append(text[i:j + 1])
            i = j + 1
        elif c == "'":
            j = i + 1
            if j < n and text[j] == "\\":
                j += 1
            j += 1
            if j < n and text[j] == "'":
                out.append(text[i:j + 1])
                i = j + 1
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def skip_string(text, i):
    verbatim = i > 0 and text[i - 1] == "@"
    j = i + 1
    while j < len(text):
        if verbatim and text[j] == '"' and j + 1 < len(text) and text[j + 1] == '"':
            j += 2
        elif not verbatim and text[j] == "\\":
            j += 2
        elif text[j] == '"':
            return j + 1
        else:
            j += 1
    return j


def matching(text, i, opener, closer):
    """The index just past the bracket matching the one at i."""
    depth, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            i = skip_string(text, i)
            continue
        if c == opener:
            depth += 1
        elif c == closer:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def split_args(s):
    out, depth, cur, i, n = [], 0, "", 0, len(s)
    while i < n:
        c = s[i]
        if c == '"':
            j = skip_string(s, i)
            cur += s[i:j]
            i = j
            continue
        if c in "([{<":
            depth += 1
        elif c in ")]}>":
            depth -= 1
        elif c == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
            i += 1
            continue
        cur += c
        i += 1
    if cur.strip():
        out.append(cur.strip())
    return out


def unquote(s):
    s = s.strip().lstrip("@")
    if s.startswith('"') and s.endswith('"'):
        return s[1:-1].replace('""', '"').replace('\\"', '"')
    return s


class Member:
    def __init__(self, kind, name, type_, attrs, modifiers, index, body, initializer, klass, params=""):
        self.kind, self.name, self.type, self.attrs = kind, name, type_, attrs
        self.modifiers, self.index, self.body, self.initializer, self.klass = modifiers, index, body, initializer, klass
        self.params = params

    def getter(self):
        if self.body is None:
            return None
        if self.body.startswith("=>"):
            return self.body[2:]
        m = re.search(r"\bget\b\s*(\{|=>|;)", self.body)
        if not m:
            return None
        if m.group(1) == ";":
            return ""
        if m.group(1) == "=>":
            return self.body[m.end():self.body.index(";", m.end())]
        return self.body[m.end() - 1:matching(self.body, m.end() - 1, "{", "}")]

    def setter(self):
        if self.body is None or self.body.startswith("=>"):
            return None
        m = re.search(r"\bset\b\s*(\{|=>|;)", self.body)
        if not m:
            return None
        if m.group(1) == ";":
            return ""
        if m.group(1) == "=>":
            return self.body[m.end():self.body.index(";", m.end())]
        return self.body[m.end() - 1:matching(self.body, m.end() - 1, "{", "}")]


def parse_attributes(text):
    """`[A(x), B]` as [(name, args string)]."""
    attrs = []
    for part in split_args(text[1:-1]):
        m = re.match(r"(?:[\w.]+\.)?(\w+?)(?:Attribute)?\s*(?:\((.*)\))?\s*$", part, flags=re.S)
        if m:
            attrs.append((m.group(1), m.group(2) if m.group(2) is not None else ""))
    return attrs


def scan_members(body, klass):
    """The fields, properties and methods directly inside a class body."""
    members = []
    attrs = []
    i, n, index = 0, len(body), 0
    while i < n:
        c = body[i]
        if c.isspace():
            i += 1
        elif c == "#":
            j = body.find("\n", i)
            i = n if j < 0 else j + 1
        elif c == "[":
            j = matching(body, i, "[", "]")
            attrs.extend(parse_attributes(body[i:j]))
            i = j
        elif c == ";" or c == "}":
            i += 1
        else:
            # The header runs to the first { ; = or ( outside brackets.
            j, angle = i, 0
            while j < n:
                d = body[j]
                if d == "<":
                    angle += 1
                elif d == ">":
                    angle -= 1
                elif d == "[":
                    j = matching(body, j, "[", "]")
                    continue
                elif d in "{;=(" and angle <= 0:
                    break
                j += 1
            if j >= n:
                break
            header = " ".join(body[i:j].split())
            stop = body[j]
            if stop == "(":
                open_paren = j
                j = matching(body, j, "(", ")")
                # A method: past its constructor initializer, up to its body or `;` or `=>`.
                k = j
                while k < n and body[k] not in "{;=":
                    k += 1
                if k < n and body[k] == "{":
                    k = matching(body, k, "{", "}")
                else:
                    k = body.find(";", k) + 1
                words = header.split()
                if words:
                    members.append(Member("method", words[-1], "", attrs, words[:-1], index, body[j:k], None, klass, body[open_paren + 1:j - 1].strip()))
                attrs = []
                index += 1
                i = k
                continue
            words = header.split()
            if not words or words[0] == "using":
                i = body.find(";", j) + 1 if stop != "{" else matching(body, j, "{", "}")
                attrs = []
                continue
            if any(w in ("class", "struct", "enum", "interface", "record") for w in words):
                i = matching(body, body.find("{", j), "{", "}") if "{" in body[j:] else n
                attrs = []
                continue
            name = words[-1]
            mods = [w for w in words[:-1] if w in ("public", "private", "protected", "internal", "static", "override", "virtual", "new", "abstract", "sealed", "readonly", "const", "partial", "unsafe", "required", "extern", "volatile")]
            type_ = " ".join(w for w in words[:-1] if w not in mods)
            if stop == "{":
                k = matching(body, j, "{", "}")
                prop_body = body[j:k]
                init = None
                rest = body[k:].lstrip()
                if rest.startswith("="):
                    e = k + body[k:].index("=") + 1
                    end = statement_end(body, e)
                    init = body[e:end].strip()
                    k = end + 1
                members.append(Member("property", name, type_, attrs, mods, index, prop_body, init, klass))
                i = k
            elif stop == ";":
                members.append(Member("field", name, type_, attrs, mods, index, None, None, klass))
                i = j + 1
            else:
                if body[j:j + 2] == "=>":
                    end = statement_end(body, j)
                    members.append(Member("property", name, type_, attrs, mods, index, body[j:end + 1], None, klass))
                    i = end + 1
                else:
                    end = statement_end(body, j + 1)
                    members.append(Member("field", name, type_, attrs, mods, index, None, body[j + 1:end].strip(), klass))
                    i = end + 1
            attrs = []
            index += 1
    return members


def statement_end(text, i):
    depth, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            i = skip_string(text, i)
            continue
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == ";" and depth == 0:
            return i
        i += 1
    return n


class LazyClass:
    """A class found while indexing. Its members are parsed the first time they're needed: the source tree is large."""

    def __init__(self, name, base, text, start, end, path):
        self.name, self.base, self.path = name, base, path
        self._parts = [(text, start, end)]
        self._members = None

    def add_part(self, other):
        """Another part of a partial class."""
        self._parts.extend(other._parts)
        self.base = self.base or other.base

    @property
    def body(self):
        return "\n".join(text[start + 1:end - 1] for text, start, end in self._parts)

    @property
    def members(self):
        if self._members is None:
            self._members = scan_members(self.body, self)
        return self._members

    @property
    def by_name(self):
        out = {}
        for m in self.members:
            out.setdefault(m.name, m)
        return out


def index_classes():
    classes = collections.defaultdict(list)
    enums = collections.defaultdict(list)
    for root in EXTRA_SEARCH:
        for f in root.rglob("*.cs"):
            if f.name.endswith(".Designer.cs"):
                continue
            text = strip_comments(read(f))
            for m in re.finditer(r"\bclass\s+(\w+)\s*(?:<[^>{]*>)?\s*(?::\s*([^{]+?))?\s*(?:where\s[^{]*)?\{", text):
                start = text.index("{", m.end() - 1)
                end = matching(text, start, "{", "}")
                base = None
                if m.group(2):
                    base = re.sub(r"<.*", "", m.group(2).split(",")[0].strip()).split(".")[-1]
                part = LazyClass(m.group(1), base, text, start, end, f)
                same_folder = next((c for c in classes[m.group(1)] if c.path.parent == f.parent), None)
                if same_folder is not None:
                    same_folder.add_part(part)
                else:
                    classes[m.group(1)].append(part)
            for m in re.finditer(r"\benum\s+(\w+)\s*(?::\s*\w+\s*)?\{", text):
                start = text.index("{", m.start())
                end = matching(text, start, "{", "}")
                members = []
                for part in split_args(text[start + 1:end - 1]):
                    desc, member_value = None, None
                    d = re.search(r'\[\s*(?:System\.ComponentModel\.)?Description\s*\(\s*(@?"[^"]*")\s*\)\s*\]', part)
                    if d:
                        desc = unquote(d.group(1))
                    v = re.search(r'\[\s*EnumMember\s*\(\s*Value\s*=\s*"([^"]*)"\s*\)\s*\]', part)
                    if v:
                        member_value = v.group(1)
                    cleaned = re.sub(r"\[[^\]]*\]", "", part).strip().split("=")[0].strip()
                    if re.fullmatch(r"\w+", cleaned):
                        members.append((member_value or cleaned, desc if desc is not None else cleaned))
                enums[m.group(1)].append((f, members))
    return classes, enums


def load_resx(name):
    out = {}
    for d in ET.parse(ATTR_DIR / name).getroot().findall("data"):
        v = d.find("value")
        if v is not None and v.text is not None:
            out[d.get("name")] = v.text
    return out


CATEGORIES = load_resx("EffectCategoryDescriptors.resx")
DISPLAY_NAMES = load_resx("EffectDisplayNameDescriptors.resx")
DESCRIPTIONS = load_resx("EffectDescriptionDescriptors.resx")
CLASSES, ENUMS = index_classes()


def find_class(name, near=None):
    options = CLASSES.get(name, [])
    if not options:
        return None
    if near is not None:
        for c in options:
            if near in c.path.parents:
                return c
    return options[0]


def class_chain(cls):
    """Base first."""
    chain = []
    seen = set()
    while cls is not None and cls.name not in seen:
        seen.add(cls.name)
        chain.insert(0, cls)
        cls = find_class(cls.base) if cls.base else None
    return chain


def attr(attrs, name):
    for a, args in reversed(attrs):
        if a == name:
            return args
    return None


# ------------------------------------------------------------------------------------- what the grid shows

def merged_properties(cls):
    """Properties of a class and its bases. An override keeps the attributes it does not replace, as TypeDescriptor does.
    Returns name -> (Member, merged attrs, declaring index)."""
    props = {}
    order = 0
    for c in class_chain(cls):
        for m in c.members:
            if m.kind != "property" or "static" in m.modifiers:
                continue
            order += 1
            if m.name in props:
                base_attrs = props[m.name][1]
                names = {a for a, _ in m.attrs}
                merged = [x for x in base_attrs if x[0] not in names] + m.attrs
                # A later override carries the getter and setter that run.
                body = m if m.body and m.body.strip() not in ("", "{ get; set; }") else props[m.name][0]
                props[m.name] = (body, merged, props[m.name][2])
            else:
                props[m.name] = (m, m.attrs, order)
    return props


NUMERIC_LIMITS = {
    "byte.MinValue": 0, "byte.MaxValue": 255, "ushort.MinValue": 0, "ushort.MaxValue": 65535,
    "uint.MinValue": 0, "uint.MaxValue": 4294967295, "ulong.MinValue": 0, "ulong.MaxValue": 18446744073709551615,
    "int.MinValue": -2147483648, "int.MaxValue": 2147483647,
}


def number(s):
    s = s.strip()
    if s in NUMERIC_LIMITS:
        return float(NUMERIC_LIMITS[s])
    negative = s.startswith("-")
    s = s.lstrip("-").rstrip("dDfFmM")
    if s.startswith("."):
        s = "0" + s
    try:
        v = float(s)
    except ValueError:
        return None
    return -v if negative else v


def constant(text):
    """A number, or a `Class.Name` constant."""
    m = re.fullmatch(r"(\w+)\.(\w+)", text.strip())
    if m and text.strip() not in NUMERIC_LIMITS:
        cls = find_class(m.group(1))
        member = cls.by_name.get(m.group(2)) if cls else None
        if member is not None and member.initializer is not None:
            return number(member.initializer)
    return number(text)


def expected_editor(attrs):
    pe = attr(attrs, "PropertyEditor")
    rng = attr(attrs, "NumberRange")
    nums = [constant(v) for v in split_args(rng)] if rng else []
    # A read-only row is a label whatever else it says (`EditorCollection.GetEditor`).
    read_only = attr(attrs, "ReadOnly")
    if read_only and "true" in read_only.lower():
        return {"type": "Label"}
    if pe:
        pe = unquote(pe)
        # No class has this name, so the grid goes on to the property's type: Video's `StartTimeSeconds` gets a numeric field.
        if pe == "DoubleSliderEditor":
            pe = None
    if pe:
        if pe == "SliderEditor":
            return {"type": "Slider", "min": nums[0], "max": nums[1], "step": nums[2]} if len(nums) >= 3 else {"type": "Slider", "min": 0.0, "max": 100.0, "step": 1.0}
        if pe == "SliderDoubleEditor":
            return {"type": "SliderDouble", "min": nums[0], "max": nums[1], "step": nums[2]} if len(nums) >= 3 else {"type": "SliderDouble", "min": 0.0, "max": 100.0, "step": 0.1}
        if pe == "LevelEditor":
            return {"type": "Level"}
        if pe == "SelectionEditor":
            return {"type": "Selection"}
        if pe == "Label":
            return {"type": "Label"}
        if pe.endswith("PathEditor"):
            return {"type": "Path", "path": pe[: -len("PathEditor")]}
        return {"type": "Default", "unknown": pe}
    if len(nums) >= 3:
        return {"type": "Number", "min": nums[0], "max": nums[1], "step": nums[2]}
    return {"type": "Default"}


def expected_display(prop):
    """The conversion between a property's stored value and what it shows, from its getter."""
    g = prop.getter()
    if not g:
        return {"type": "Stored"}
    text = " ".join(g.strip("{} \n\t;").split()).replace("return ", "").strip("; ")
    data = r"(?:\(\(\w+\)\s*Data\)|Data|_data|EffectModuleData)\.\w+"
    m = re.fullmatch(data + r"\s*\*\s*(\d+)", text)
    if m:
        return {"type": "Times", "k": int(m.group(1))}
    m = re.fullmatch(r"(\d+)\s*-\s*" + data, text)
    if m:
        return {"type": "Minus", "k": int(m.group(1))}
    return {"type": "Stored"}


def getter_data_member(prop):
    g = prop.getter()
    if not g:
        return None
    m = re.search(r"(?:\(\(\w+\)\s*Data\)|\bData|\b_data|\bEffectModuleData)\.(\w+)", g)
    return m.group(1) if m else None


def expected_row(prop, attrs):
    """What the grid shows for a property, from its attributes. `EditorCollection` picks the editor by type where there is
    no [PropertyEditor], which only matters to this table for a `Percentage`."""
    cat = attr(attrs, "ProviderCategory")
    category, category_order = "Misc", -1
    if cat is not None:
        a = split_args(cat)
        key = unquote(a[0])
        category = CATEGORIES.get(key, key)
        if len(a) > 1 and number(a[1]) is not None:
            category_order = int(number(a[1]))
    elif attr(attrs, "Category") is not None:
        category = unquote(attr(attrs, "Category"))
    label = attr(attrs, "ProviderDisplayName")
    if label is not None:
        label = DISPLAY_NAMES.get(unquote(label), unquote(label))
    elif attr(attrs, "DisplayName") is not None:
        label = unquote(attr(attrs, "DisplayName"))
    else:
        label = prop.name
    desc = attr(attrs, "ProviderDescription")
    if desc is not None:
        desc = DESCRIPTIONS.get(unquote(desc), unquote(desc))
    elif attr(attrs, "Description") is not None:
        desc = unquote(attr(attrs, "Description"))
    else:
        desc = ""
    order = attr(attrs, "PropertyOrder")
    order = int(number(order)) if order is not None and number(order) is not None else 0
    browsable = attr(attrs, "Browsable")
    bool_desc = attr(attrs, "BoolDescription")
    return {
        "category": category,
        "category_order": category_order,
        "label": label,
        "description": desc,
        "order": order,
        "browsable": not (browsable is not None and "false" in browsable),
        "is_value": attr(attrs, "Value") is not None,
        "exclude_from_default": attr(attrs, "ExcludeFromEffectDefault") is not None,
        "editor": {"type": "Percent"} if prop.type.split(".")[-1] == "Percentage" and attr(attrs, "PropertyEditor") is None else expected_editor(attrs),
        "display": expected_display(prop),
        "bool_labels": [unquote(x) for x in split_args(bool_desc)] if bool_desc is not None else None,
    }


# ------------------------------------------------------------------------------ data classes and their defaults

class Unknown:
    def __init__(self, why):
        self.why = why


def color_value(expr):
    expr = expr.strip()
    m = re.fullmatch(r"(?:System\.Drawing\.)?Color\.(\w+)", expr)
    if m:
        return KNOWN_COLORS.get(m.group(1), Unknown("colour " + m.group(1)))
    m = re.fullmatch(r"(?:System\.Drawing\.)?Color\.FromArgb\((.*)\)", expr, flags=re.S)
    if m:
        a = [number(x) for x in split_args(m.group(1))]
        if None in a:
            return Unknown(expr)
        if len(a) == 3:
            return (255, int(a[0]), int(a[1]), int(a[2]))
        if len(a) == 4:
            return tuple(int(x) for x in a)
    return Unknown(expr)


class Interpreter:
    """Runs a data class's constructor and initializers far enough to know the values they leave."""

    def __init__(self, cls_name, near):
        self.cls_name, self.near = cls_name, near

    def new_object(self, cls_name):
        cls = find_class(cls_name, self.near)
        obj = {"__class__": cls_name}
        if cls is None:
            return obj
        for c in class_chain(cls):
            for m in c.members:
                if m.initializer is not None and m.kind in ("property", "field") and "const" not in m.modifiers and "static" not in m.modifiers:
                    obj[m.name] = self.eval(m.initializer, obj, {}, m.type)
            ctor = None
            for m in c.members:
                if m.kind == "method" and m.name == c.name and m.params == "":
                    ctor = m
            if ctor is not None:
                self.run_block(ctor.body, obj, {}, c)
        return obj

    def run_block(self, body, obj, local, cls):
        start = body.find("{")
        if start < 0:
            return
        end = matching(body, start, "{", "}")
        text = body[start + 1:end - 1]
        i = 0
        while i < len(text):
            e = statement_end(text, i)
            statement = " ".join(text[i:e].split())
            i = e + 1
            if statement:
                self.run_statement(statement, obj, local, cls)

    def run_statement(self, s, obj, local, cls):
        decl = re.fullmatch(r"([\w<>\[\],.]+)\s+(\w+)\s*=\s*(.*)", s, flags=re.S)
        if decl and decl.group(1) not in ("this",):
            local[decl.group(2)] = self.eval(decl.group(3), obj, local, decl.group(1))
            return
        m = re.fullmatch(r"(?:this\.)?(\w+)\s*=\s*(.*)", s, flags=re.S)
        if m:
            obj[m.group(1)] = self.eval(m.group(2), obj, local, None)
            return
        m = re.fullmatch(r"(?:this\.)?(\w+)\.(\w+)\.(\w+)\((.*)\)", s, flags=re.S)
        if m:
            container = local if m.group(1) in local else obj
            self.call(container, m.group(1), "%s.%s" % (m.group(2), m.group(3)), m.group(4), obj, local)
            return
        m = re.fullmatch(r"(?:this\.)?(\w+)\.(\w+)\((.*)\)", s, flags=re.S)
        if m:
            name, method, args = m.groups()
            container = local if name in local else obj
            if isinstance(container.get(name), list) and method == "Add":
                container[name].append(self.eval(args, obj, local, None))
                return
            if isinstance(container.get(name), dict):
                self.call(container, name, method, args, obj, local)
            return
        m = re.fullmatch(r"(\w+)\[(\d+)\]\.(\w+)\s*=\s*(.*)", s, flags=re.S)
        if m:
            target = self.resolve_path("%s[%s]" % (m.group(1), m.group(2)), obj, local)
            if isinstance(target, dict):
                target[m.group(3)] = self.eval(m.group(4), obj, local, None)
            return
        m = re.fullmatch(r"(\w+)\.(\w+)\s*=\s*(.*)", s, flags=re.S)
        if m:
            target = local.get(m.group(1), obj.get(m.group(1)))
            if isinstance(target, dict):
                target[m.group(2)] = self.eval(m.group(3), obj, local, None)
            return
        m = re.fullmatch(r"(\w+)\[(\d+)\]\.(\w+)\.(\w+)\.(\w+)\((.*)\)", s, flags=re.S)
        if m:
            target = self.resolve_path("%s[%s]" % (m.group(1), m.group(2)), obj, local)
            if isinstance(target, dict):
                self.call(target, m.group(3), "%s.%s" % (m.group(4), m.group(5)), m.group(6), obj, local)
            return
        m = re.fullmatch(r"(\w+)\(\s*\)", s)
        if m and cls is not None:
            helper = cls.by_name.get(m.group(1))
            if helper is not None and helper.kind == "method":
                self.run_block(helper.body, obj, local, cls)

    def resolve_path(self, path, obj, local):
        m = re.fullmatch(r"(\w+)\[(\d+)\]", path)
        if m:
            target = local.get(m.group(1), obj.get(m.group(1)))
            if isinstance(target, list) and int(m.group(2)) < len(target):
                return target[int(m.group(2))]
            return None
        return local.get(path, obj.get(path))

    def call(self, container, name, method, args, obj, local):
        """Methods on a gradient's Colors or a curve's Points, as `X.Colors.Add(...)`."""
        value = container.get(name)
        if not isinstance(value, dict):
            return
        if method in ("Colors.Clear",):
            value["colors"] = []
        elif method == "Colors.Add":
            value.setdefault("colors", []).append(self.color_point(args))
        elif method == "Points.Clear":
            value["points"] = []
        elif method == "Points.Add":
            m = re.search(r"PointPair\((.*)\)", args)
            if m:
                value.setdefault("points", []).append([number(x) for x in split_args(m.group(1))])
        elif method == "Clear" and "colors" in value:
            value["colors"] = []
        elif method == "Add" and "colors" in value:
            value["colors"].append(self.color_point(args))
        elif method == "Clear" and "points" in value:
            value["points"] = []
        elif method == "Add" and "points" in value:
            m = re.search(r"PointPair\((.*)\)", args)
            if m:
                value["points"].append([number(x) for x in split_args(m.group(1))])

    def color_point(self, args):
        m = re.search(r"ColorPoint\((.*)\)", args)
        a = split_args(m.group(1)) if m else []
        return {"color": color_value(a[0]) if a else Unknown(args), "position": number(a[1]) if len(a) > 1 else 0.0}

    def gradient_of(self, args, obj, local):
        a = split_args(args)
        if len(a) == 0:
            return {"colors": [{"color": KNOWN_COLORS["White"], "position": 0.0}], "__type": "gradient"}
        if len(a) == 1:
            c = color_value(a[0])
            if not isinstance(c, Unknown):
                return {"colors": [{"color": c, "position": 0.0}], "__type": "gradient"}
            blend = local.get(a[0])
            if isinstance(blend, dict) and "Colors" in blend and "Positions" in blend:
                return {"colors": [{"color": x, "position": y} for x, y in zip(blend["Colors"], blend["Positions"])], "__type": "gradient"}
        return Unknown("new ColorGradient(%s)" % args)

    def curve_of(self, args):
        a = split_args(args)
        if not a:
            return {"points": [[0.0, 0.0], [100.0, 100.0]], "__type": "curve"}
        m = re.fullmatch(r"CurveType\.(\w+)", a[0])
        if m:
            pts = {"RampUp": [[0.0, 0.0], [100.0, 100.0]], "RampDown": [[0.0, 100.0], [100.0, 0.0]], "Flat100": [[0.0, 100.0], [100.0, 100.0]]}.get(m.group(1))
            return {"points": pts, "__type": "curve"} if pts else Unknown(a[0])
        m = re.search(r"new\s*\[\]\s*\{([^}]*)\}\s*,\s*new\s*\[\]\s*\{([^}]*)\}", args)
        if m:
            xs = [number(x) for x in split_args(m.group(1))]
            ys = [number(y) for y in split_args(m.group(2))]
            if None not in xs and None not in ys:
                return {"points": [[x, y] for x, y in zip(xs, ys)], "__type": "curve"}
        if len(a) == 1 and number(a[0]) is not None:
            return {"points": [[0.0, number(a[0])], [100.0, number(a[0])]], "__type": "curve"}
        return Unknown("new Curve(%s)" % args)

    def eval(self, expr, obj, local, type_hint):
        e = " ".join(expr.strip().split())
        if e == "[]":
            return []
        if e in ("true", "false"):
            return e == "true"
        if e == "null":
            return None
        if e in ("string.Empty", "String.Empty"):
            return ""
        if e == "Guid.Empty":
            return "00000000-0000-0000-0000-000000000000"
        if e.startswith('"') or e.startswith('@"'):
            return unquote(e)
        n = number(e)
        if n is not None:
            return int(n) if re.fullmatch(r"-?\d+", e) else n
        m = re.fullmatch(r"\(\s*\w+\s*\)\s*(\w+)\s*/\s*(\d+)", e)
        if m:
            base = local.get(m.group(1), obj.get(m.group(1)))
            if isinstance(base, (int, float)):
                # `(float)x / 100` is a float, so it is compared with the Rust f32 widened to a double.
                return float(_to_f32(float(base) / float(m.group(2))))
        if e in local:
            return local[e]
        if e in obj:
            return obj[e]
        c = color_value(e)
        if not isinstance(c, Unknown):
            return c
        m = re.fullmatch(r"new (?:System\.Drawing\.)?Color\[\]\s*\{(.*)\}|\{(.*)\}", e)
        if m:
            items = split_args(m.group(1) if m.group(1) is not None else m.group(2))
            if type_hint and "Color" in type_hint:
                return [color_value(x) for x in items]
            return [self.eval(x, obj, local, None) for x in items]
        m = re.fullmatch(r"new (?:[\w.]*\.)?ColorGradient\((.*)\)", e)
        if m:
            return self.gradient_of(m.group(1), obj, local)
        m = re.fullmatch(r"new (?:[\w.]*\.)?Curve\((.*)\)", e)
        if m:
            return self.curve_of(m.group(1))
        m = re.fullmatch(r"new (?:[\w.]*\.)?GradientLevelPair\((.*)\)", e)
        if m:
            a = split_args(m.group(1))
            if len(a) == 0:
                return {"gradient": self.gradient_of("", obj, local), "curve": self.curve_of("CurveType.Flat100"), "__type": "level"}
            g = self.gradient_of(a[0], obj, local) if color_value(a[0]).__class__ is tuple else self.eval(a[0], obj, local, None)
            curve = self.curve_of(a[1]) if len(a) > 1 else self.curve_of("")
            return {"gradient": g, "curve": curve, "__type": "level"}
        m = re.fullmatch(r"new (?:[\w.]*\.)?ColorBlend\(\s*\)", e)
        if m:
            return {"__type": "blend"}
        m = re.fullmatch(r"new (?:List<[^>]+(?:<[^>]*>)?>|[\w.]+\[\])\s*(?:\(\s*\))?\s*\{(.*)\}", e)
        if m:
            return [self.eval(x, obj, local, None) for x in split_args(m.group(1))]
        m = re.fullmatch(r"new (?:System\.Collections\.Generic\.)?List<[^>]+(?:<[^>]*>)?>\s*\(\s*\)", e)
        if m:
            return []
        m = re.fullmatch(r"new (?:Serializable)?Font\((.*)\)", e)
        if m:
            inner = re.search(r'new Font\(\s*"([^"]+)"\s*,\s*([\d.]+)[fF]?', e) or re.fullmatch(r'new Font\(\s*"([^"]+)"\s*,\s*([\d.]+)[fF]?.*\)', e)
            if inner:
                return {"name": inner.group(1), "size": float(inner.group(2)), "__type": "font"}
            return Unknown(e)
        if e == "new Guid()":
            return "00000000-0000-0000-0000-000000000000"
        m = re.fullmatch(r"new (\w+)\(\s*\)", e)
        if m:
            return self.new_object(m.group(1))
        m = re.fullmatch(r"new (?:System\.Drawing\.)?Size\((.*)\)", e)
        if m:
            a = [number(x) for x in split_args(m.group(1))]
            if len(a) == 2 and None not in a:
                return {"__class__": "Size", "width": int(a[0]), "height": int(a[1])}
            return Unknown(e)
        m = re.fullmatch(r"(?:[\w.]+\.)?(\w+)\.(\w+)", e)
        if m:
            return m.group(2)  # An enum member.
        m = re.fullmatch(r"new\(\s*\)", e)
        if m and type_hint:
            return self.eval("new %s()" % type_hint, obj, local, type_hint)
        return Unknown(e)


def _to_f32(v):
    import struct
    return struct.unpack("f", struct.pack("f", v))[0]


def data_chain_members(cls):
    """[DataMember]s by member name, across the data class and its bases. The instance and type ids are the module system's."""
    out = {}
    for c in class_chain(cls):
        if c.name == "ModuleDataModelBase":
            continue
        for m in c.members:
            if m.kind in ("property", "field") and attr(m.attrs, "DataMember") is not None:
                args = attr(m.attrs, "DataMember") or ""
                n = re.search(r'Name\s*=\s*"([^"]*)"', args)
                out[m.name] = (m, n.group(1) if n else m.name, c)
    return out


def same_number(a, b, tolerance=1e-6):
    return a == b or (isinstance(a, (int, float)) and isinstance(b, (int, float)) and abs(a - b) <= tolerance)


def same_color(a, b):
    # Through XYZ and back a channel can land one off.
    return len(a) == len(b) and all(abs(int(x) - int(y)) <= 1 for x, y in zip(a, b))


def same_default(expected, actual, kind):
    """Returns None when they agree, or what differs."""
    k = kind["kind"]
    if isinstance(expected, Unknown):
        return "unevaluated"
    if k in ("Int", "Long", "Double", "Percentage"):
        if expected is None:
            expected = 0
        if isinstance(expected, dict) and k == "Percentage":
            return "unevaluated"
        return None if same_number(expected, actual) else "%r != %r" % (actual, expected)
    if k == "Bool":
        return None if bool(expected) == bool(actual) else "%r != %r" % (actual, expected)
    if k == "Str":
        if expected is None:
            expected = ""
        return None if expected == actual else "%r != %r" % (actual, expected)
    if k == "Enum":
        names = [m[0] for m in kind["members"]]
        if expected is None:
            expected = names[0] if names else ""
        return None if expected == actual else "%r != %r" % (actual, expected)
    if k == "Curve":
        if expected is None:
            return None if actual is None else "%r != null" % (actual,)
        if not isinstance(actual, dict):
            return "%r != %r" % (actual, expected["points"])
        if len(actual["points"]) != len(expected["points"]) or not all(same_number(a[0], b[0]) and same_number(a[1], b[1]) for a, b in zip(actual["points"], expected["points"])):
            return "%r != %r" % (actual["points"], expected["points"])
        return None
    if k == "Gradient":
        return same_gradient(expected, actual)
    if k == "GradientList":
        if expected is None:
            expected = []
        if len(expected) != len(actual or []):
            return "%d gradients != %d" % (len(actual or []), len(expected))
        for e, a in zip(expected, actual):
            d = same_gradient(e, a)
            if d:
                return d
        return None
    if k == "GradientLevels":
        if expected is None:
            expected = []
        if len(expected) != len(actual or []):
            return "%d pairs != %d" % (len(actual or []), len(expected))
        for e, a in zip(expected, actual):
            d = same_gradient(e["gradient"], a["gradient"]) or same_default(e["curve"], a["curve"], {"kind": "Curve"})
            if d:
                return d
        return None
    if k == "Color":
        if expected is None or expected == 0:
            expected = KNOWN_COLORS["Empty"]
        if isinstance(expected, str):
            expected = KNOWN_COLORS.get(expected, expected)
        return None if same_color(expected, actual) else "%r != %r" % (actual, expected)
    if k == "ColorList":
        expected = expected or []
        return None if len(expected) == len(actual) and all(same_color(a, b) for a, b in zip(expected, actual)) else "%r != %r" % (actual, expected)
    if k == "Font":
        if expected is None:
            return None
        if isinstance(expected, dict) and expected.get("__type") == "font":
            return None if expected["name"] == actual["name"] and same_number(expected["size"], actual["size"]) else "%r != %r" % (actual, expected)
        return "unevaluated"
    if k == "Record":
        if expected is None:
            return None if actual is None else "a value != null"
        if isinstance(expected, dict) and "__class__" in expected and actual is not None:
            return same_record(expected, actual, kind["record"])
        return None
    if k == "RecordList":
        expected = expected or []
        actual = actual or []
        if len(expected) != len(actual):
            return "%d rows != %d" % (len(actual), len(expected))
        for e, a in zip(expected, actual):
            d = same_record(e, a, kind["record"])
            if d:
                return d
        return None
    if k == "Guid":
        e = expected or "00000000-0000-0000-0000-000000000000"
        return None if e == actual else "%r != %r" % (actual, e)
    return None


def same_record(expected, actual, record):
    for p in record["params"]:
        if p["name"] in expected and p["xml"]:
            d = same_default(expected[p["name"]], actual.get(p["name"]), p["kind"])
            if d and d != "unevaluated":
                return "%s: %s" % (p["name"], d)
    return None


def same_gradient(expected, actual):
    if expected is None:
        return None if actual is None else "a gradient != null"
    if isinstance(expected, Unknown) or not isinstance(expected, dict) or "colors" not in expected:
        return "unevaluated"
    if not isinstance(actual, dict):
        return "%r != a gradient" % (actual,)
    e, a = expected["colors"], actual["colors"]
    if len(e) != len(a):
        return "%d colour points != %d" % (len(a), len(e))
    for x, y in zip(e, a):
        if not same_color(x["color"], y["color"]) or not same_number(x["position"], y["position"], 1e-5):
            return "%r != %r" % (y, x)
    return None


# ------------------------------------------------------------------------------------------------- the checks

class Report:
    def __init__(self):
        self.findings = []
        self.accepted = []
        self.counts = collections.Counter()
        self.unevaluated = []
        self.expected = []

    def checked(self, check):
        self.counts[check] += 1

    def add(self, effect, name, check, message, expected=None):
        reason = ACCEPTED.get((effect, name, check)) or ACCEPTED.get((effect, name, "*"))
        if reason:
            self.accepted.append((effect, name, check, message, reason))
        else:
            self.findings.append((effect, name, check, message))
            self.expected.append({"effect": effect, "name": name, "check": check, "expected": expected})


def module_class(info):
    near = SRC / info["dir"]
    cls = find_class(info["class"], near)
    return cls, near


def enum_members(enum_name, near):
    options = ENUMS.get(enum_name)
    if not options:
        return None
    for f, members in options:
        if near in f.parents:
            return members
    return options[0][1]


def cs_kind_matches(type_, kind, near):
    t = type_.replace("System.Drawing.", "").replace("System.", "").rstrip("?").strip()
    k = kind["kind"]
    simple = {
        "int": ("Int",), "Int32": ("Int",), "byte": ("Int",), "ushort": ("Long", "Int"), "uint": ("Long", "Int"), "ulong": ("Long", "Int"), "long": ("Long",),
        "double": ("Double",), "Double": ("Double",), "float": ("Double",), "bool": ("Bool",), "Boolean": ("Bool",),
        "string": ("Str",), "String": ("Str",), "Curve": ("Curve",), "ColorGradient": ("Gradient",),
        "List<ColorGradient>": ("GradientList",), "List<GradientLevelPair>": ("GradientLevels",), "Color": ("Color",),
        "List<Color>": ("ColorList",), "RGB": ("Rgb",), "Percentage": ("Percentage",), "SerializableFont": ("Font",),
        "Font": ("Font",), "Guid": ("Guid",), "TimeSpan": ("Time",),
    }
    if t in simple:
        return k in simple[t]
    if k == "Enum":
        return t.split(".")[-1] == kind["enum"]
    return k in ("Record", "RecordList", "Other", "Str")


def check_table(report, effect, params, cls, near, chain_data, aliases, is_row=False):
    """One table of parameters against the class whose attributes describe them."""
    props = merged_properties(cls) if cls is not None else {}
    category_orders = effective_category_orders(cls) if cls is not None else {}
    by_xml = {xml: m for m, xml, _ in (chain_data or {}).values()}

    # Every property the grid shows needs a row.
    for name, (m, attrs, _) in props.items():
        if attr(attrs, "Browsable") is not None and "false" in (attr(attrs, "Browsable") or ""):
            continue
        is_editor_prop = any(attr(attrs, a) is not None for a in ("ProviderDisplayName", "ProviderCategory", "ProviderDescription", "Value"))
        if not is_editor_prop or m.kind != "property" or "public" not in m.modifiers:
            continue
        report.checked("row exists")
        if not any(aliases.get((effect, p["name"]), p["name"]) == name for p in params):
            report.add(effect, name, "missing", "the C# property has no row in the table")

    for p in params:
        pname = aliases.get((effect, p["name"]), p["name"])
        if pname not in props:
            # A member with no property: the grid never shows it.
            if p["browsable"] or p["label"] or p["is_value"]:
                report.checked("data only")
                report.add(effect, p["name"], "data-only", "no C# property, but the table gives it a place in the editor")
            continue
        prop, attrs, _ = props[pname]
        exp = expected_row(prop, attrs)
        exp["category_order"] = category_orders.get(exp["category"], exp["category_order"])
        # The attribute goes on the data member, which is where EffectDefaultScrubber looks.
        member_attrs = by_xml[p["xml"]].attrs if p["xml"] in by_xml else []
        exp["exclude_from_default"] = attr(member_attrs, "ExcludeFromEffectDefault") is not None
        if is_row and attr(attrs, "ProviderCategory") is None and attr(attrs, "Category") is None:
            exp["category"], exp["category_order"] = "", 0
        hidden = not exp["browsable"]
        for field in ("category", "category_order", "label", "description", "order", "browsable", "is_value", "exclude_from_default"):
            report.checked(field)
            if p[field] != exp[field]:
                # Nothing shows a hidden property's place, so only the state itself matters there.
                if hidden and field in ("category", "category_order", "order", "label", "description") and not p["browsable"]:
                    continue
                report.add(effect, p["name"], field, "%r, the C# says %r" % (p[field], exp[field]), exp[field])
        report.checked("editor")
        e = exp["editor"]
        if "unknown" in e:
            report.add(effect, p["name"], "editor", "unknown C# editor " + e["unknown"])
        else:
            a = p["editor"]
            same = a["type"] == e["type"] and a.get("path") == e.get("path") and all(same_number(a.get(k), e.get(k)) for k in ("min", "max", "step"))
            if not same:
                report.add(effect, p["name"], "editor", "%s, the C# says %s" % (json.dumps(a), json.dumps(e)), e)
        report.checked("display")
        if p["display"] != exp["display"]:
            report.add(effect, p["name"], "display", "%s, the C# says %s" % (json.dumps(p["display"]), json.dumps(exp["display"])), exp["display"])
        report.checked("bool_labels")
        if p["bool_labels"] != exp["bool_labels"]:
            report.add(effect, p["name"], "bool_labels", "%r, the C# says %r" % (p["bool_labels"], exp["bool_labels"]), exp["bool_labels"])
        # The stored member: its name and type.
        member = getter_data_member(prop)
        if member and p["xml"] and chain_data is not None and member in chain_data:
            _, xml_name, _ = chain_data[member]
            report.checked("xml name")
            if p["xml"] != xml_name:
                report.add(effect, p["name"], "xml", "%r, the C# stores %r" % (p["xml"], xml_name))


def effective_category_orders(cls):
    """`CollectCategories`: a category takes the order of the first of its properties that TypeDescriptor returns, and that
    walks the most derived class first, in declaration order."""
    orders = {}
    for c in reversed(class_chain(cls)):
        merged = merged_properties(c)
        for m in c.members:
            if m.kind == "property" and m.name in merged:
                exp = expected_row(merged[m.name][0], merged[m.name][1])
                orders.setdefault(exp["category"], exp["category_order"])
    return orders


def check_order(report, effect, params, cls, aliases):
    """What the grid shows, in order: categories by order then name, properties by [PropertyOrder] then display name."""
    props = merged_properties(cls)
    category_orders = effective_category_orders(cls)
    def csharp_key(p):
        name = aliases.get((effect, p["name"]), p["name"])
        if name not in props:
            return None
        exp = expected_row(props[name][0], props[name][1])
        if not exp["browsable"] and not p["browsable"]:
            return None
        return (category_orders.get(exp["category"], exp["category_order"]), exp["category"].lower(), exp["order"], exp["label"].lower(), name)
    shown = [p for p in params if csharp_key(p) is not None]
    expected = [p["name"] for p in sorted(shown, key=csharp_key)]
    rust_key = lambda ip: (ip[1]["category_order"], ip[1]["category"].lower(), ip[1]["order"], ip[1]["label"].lower(), ip[0])
    actual = [p["name"] for i, p in sorted(enumerate(shown), key=rust_key)]
    report.checked("display order")
    if expected != actual:
        for e, a in zip(expected, actual):
            if e != a:
                report.add(effect, a, "order-of-rows", "shown where the C# shows %s" % e)
                break


def check_defaults(report, effect, params, data_cls, near):
    interp = Interpreter(data_cls.name, near)
    obj = interp.new_object(data_cls.name)
    members = data_chain_members(data_cls)
    by_xml = {xml: name for name, (_, xml, _) in members.items()}
    for p in params:
        if not p["xml"]:
            continue
        member = by_xml.get(p["xml"])
        if member is None:
            report.add(effect, p["name"], "xml", "the data class has no member stored as %r" % p["xml"])
            continue
        m, _, _ = members[member]
        report.checked("member type")
        if not cs_kind_matches(m.type, p["kind"], near):
            report.add(effect, p["name"], "kind", "%s, the C# member is %s" % (p["kind"]["kind"], m.type))
        report.checked("default")
        expected = obj.get(member)
        d = same_default(expected, p["default"], p["kind"])
        if d == "unevaluated":
            report.unevaluated.append((effect, p["name"], expected.why if isinstance(expected, Unknown) else repr(expected)))
        elif d:
            report.add(effect, p["name"], "default", d)
    for name, (m, xml, _) in members.items():
        report.checked("member has a row")
        if not any(p["xml"] == xml for p in params):
            report.add(effect, name, "missing-member", "the data class stores %r and the table has no row for it" % xml)


def check_row_defaults(report, label, record, row_cls, data_cls, near):
    """A new row, which is what `new Row()` on the view model holds, or its data class where only that sets a value."""
    interp = Interpreter(row_cls.name, near)
    row = interp.new_object(row_cls.name)
    data = interp.new_object(data_cls.name) if data_cls is not None else {}
    for p in record["params"]:
        if not p["xml"]:
            continue
        name = p["xml"]
        candidates = []
        if name in row:
            candidates.append(row[name])
        elif ("_" + name[0].lower() + name[1:]) in row:
            candidates.append(row["_" + name[0].lower() + name[1:]])
        if name in data:
            candidates.append(data[name])
        if not candidates:
            continue
        report.checked("row default")
        # A file's row starts from the data class, and the editor's add button from the view model, so either is right.
        results = [same_default(expected, p["default"], p["kind"]) for expected in candidates]
        if None in results:
            continue
        if all(r == "unevaluated" for r in results):
            report.unevaluated.append((label, p["name"], "no value it could work out"))
        else:
            report.add(label, p["name"], "row-default", next(r for r in results if r != "unevaluated"))


def check_enums(report, effect, params, near):
    for p in params:
        kind = p["kind"]
        if kind["kind"] == "Enum":
            members = enum_members(kind["enum"], near)
            report.checked("enum labels")
            if members is None:
                report.add(effect, p["name"], "enum", "no C# enum called " + kind["enum"])
                continue
            actual = [tuple(m) for m in kind["members"]]
            if actual != [tuple(m) for m in members]:
                report.add(effect, p["name"], "enum", "%r, the C# says %r" % (actual, members))
        elif kind["kind"] in ("Record", "RecordList"):
            check_enums(report, effect, kind["record"]["params"], near)


def information_of(cls):
    info, link = "", "http://vixenlights.com"
    for c in class_chain(cls):
        for m in c.members:
            if m.kind == "property" and m.name in ("Information", "InformationLink"):
                g = m.getter() or ""
                s = re.search(r'return\s+(@?"(?:[^"\\]|\\.)*")|^\s*(@?"(?:[^"\\]|\\.)*")', g)
                value = None
                if s:
                    raw = s.group(1) or s.group(2)
                    value = bytes(unquote(raw), "utf-8").decode("unicode_escape") if not raw.startswith("@") else unquote(raw)
                elif "_helpURL" in g:
                    value = "__help"
                if value is not None:
                    if m.name == "Information":
                        info = value
                    else:
                        link = value
    return info, link


def base_call_help_url(cls):
    """The URL a fixture effect passes to its FixtureEffectBase constructor."""
    for c in reversed(class_chain(cls)):
        for m in c.members:
            if m.kind == "method" and m.name == c.name:
                u = re.search(r'base\(\s*(@?"[^"]*")', m.body)
                if u:
                    return unquote(u.group(1))
    return None


def descriptor_class(cls, near):
    """The effect's EffectModuleDescriptorBase, the one naming this module class."""
    for options in CLASSES.values():
        for c in options:
            if c.base == "EffectModuleDescriptorBase" and near in c.path.parents and re.search(r"typeof\s*\(\s*(?:[\w.]+\.)?%s\s*\)" % re.escape(cls.name), c.body):
                return c
    return None


def literal_getter(c, name):
    m = c.by_name.get(name)
    if m is None:
        return None
    g = m.getter() or ""
    s = re.search(r'(@?"(?:[^"\\]|\\.)*")', g)
    return unquote(s.group(1)) if s else None


def check_descriptor(report, d, cls, near):
    desc = descriptor_class(cls, near)
    report.checked("descriptor class")
    if desc is None:
        report.add(d["name"], "(descriptor)", "descriptor", "no C# descriptor names " + cls.name)
    else:
        for field, prop in (("name", "EffectName"), ("author", "Author"), ("description", "Description"), ("version", "Version")):
            report.checked("descriptor text")
            value = literal_getter(desc, prop)
            if value is not None and value != d[field]:
                report.add(d["name"], "(descriptor)", field, "%r, the C# says %r" % (d[field], value), value)
        group = re.search(r"EffectGroups\.(\w+)", desc.body)
        report.checked("descriptor text")
        if group and group.group(1) != d["group"]:
            report.add(d["name"], "(descriptor)", "group", "%r, the C# says %r" % (d["group"], group.group(1)), group.group(1))
        guid = re.search(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}", desc.body)
        report.checked("descriptor text")
        if guid and guid.group(0).lower() != d["type_id"]:
            report.add(d["name"], "(descriptor)", "type_id", "%r, the C# says %r" % (d["type_id"], guid.group(0)), guid.group(0).lower())
    # ForceGenerateVisualRepresentation and RenderEffectByLocation.
    chain = class_chain(cls)
    force = None
    for c in chain:
        m = c.by_name.get("ForceGenerateVisualRepresentation")
        if m is not None and "public" in m.modifiers:
            force = " ".join((m.getter() or "").strip("{} \n\t;").split()).replace("return ", "").strip("; ")
    report.checked("forced visual")
    if force == "true" and not (d["force_visual"] and d["has_visual"]):
        report.add(d["name"], "(descriptor)", "force_visual", "the C# always draws its own thumbnail")
    elif force not in (None, "true", "false") and not d["force_visual_when"]:
        report.add(d["name"], "(descriptor)", "force_visual", "the C# decides per effect: " + force)
    elif force in (None, "false") and d["force_visual"]:
        report.add(d["name"], "(descriptor)", "force_visual", "the C# does not force a thumbnail")
    locates = any(m.kind == "method" and m.name == "RenderEffectByLocation" for c in chain if c.name != "PixelEffectBase" for m in c.members)
    report.checked("location support")
    if locates != d["supports_location"]:
        report.add(d["name"], "(descriptor)", "supports_location", "%r, the C# says %r" % (d["supports_location"], locates))
    report.checked("information")
    info, link = information_of(cls)
    if d["information"] != info:
        report.add(d["name"], "(descriptor)", "information", "%r, the C# says %r" % (d["information"], info), info)
    if link == "__help":
        link = base_call_help_url(cls) or ""
    if d["info_link"] != link:
        report.add(d["name"], "(descriptor)", "info_link", "%r, the C# says %r" % (d["info_link"], link), link)


def browsable_names(cls):
    """Every name a class's SetBrowsable tables mention. PixelEffectBase's two only count for effects that use them."""
    names = set()
    chain = class_chain(cls)
    own_text = "\n".join(c.body for c in chain if c.name != "PixelEffectBase")
    for c in chain:
        if c.name == "PixelEffectBase":
            if "EnableTargetPositioning(" in own_text:
                names.add("TargetPositioning")
                names.add("StringOrientation")
            elif "UpdateStringOrientationAttributes(" in own_text:
                names.add("StringOrientation")
            continue
        for m in re.finditer(r'\{\s*(?:nameof\(([\w.]+)\)|"(\w+)")\s*,', c.body):
            names.add((m.group(1) or m.group(2)).split(".")[-1])
        for m in re.finditer(r'\.Add\(\s*(?:nameof\(([\w.]+)\)|"(\w+)")\s*,', c.body):
            names.add((m.group(1) or m.group(2)).split(".")[-1])
    return names


def rust_sources(crate):
    return {f: f.read_text() for f in (CRATES / crate / "src").rglob("*.rs") if "generated" not in f.parts}


def effect_source_text(d, sources):
    """The Rust files holding an effect's visibility rules: its own module, and anything its table's name appears in."""
    module = re.sub(r"(?<!^)(?=[A-Z])", "_", d["name"].replace(" ", "").replace("-", "")).lower()
    texts = []
    for f, text in sources.items():
        if f.stem == module or f.parent.name == module or f.stem in ("audio_helper", "fixture_base", "common"):
            texts.append(text)
    return "\n".join(texts) if texts else "\n".join(sources.values())


CRATE_OF = {
    "fx-basic": "pulse set_level chase spin alternating wipe twinkle dissolve strobe candle custom_value launcher rds",
    "fx-pixel-simple": "butterfly color_wash curtain fire garlands life plasma snow_storm spiral spirograph tree circles",
    "fx-pixel-motion": "bars pin_wheel balls fireworks meteors shockwave",
    "fx-pixel-graphics": "snowflakes border shapes text count_down picture video glediator",
    "fx-pixel-complex": "whirlpool wave pattern",
    "fx-pixel-physics": "morph liquid",
    "fx-audio-state": "waveform vertical_meter vu_meter lip_sync state",
    "fx-fixture": "line_dance fixture spin_color_wheel gobo set_position fixture_strobe prism frost set_zoom",
}
CRATE_BY_MODULE = {m: c for c, ms in CRATE_OF.items() for m in ms.split()}


def check_visibility_names(report, d, info, cls, aliases):
    """A name the C# shows and hides has to appear in the Rust effect's rules."""
    crate = CRATE_BY_MODULE.get(info["module"])
    if crate is None:
        return
    sources = rust_sources(crate)
    module = info["module"]
    text = ""
    for f, t in sources.items():
        if f.stem == module or f.parent.name == module or (crate == "fx-audio-state" and f.stem == "audio_helper"):
            text += t
    table = {p["name"] for p in d["params"]}
    reverse = {v: k[1] for k, v in aliases.items() if k[0] == d["name"]}
    for name in sorted(browsable_names(cls)):
        rust = reverse.get(name, name)
        if rust in table:
            report.checked("visibility rule")
            if ('"%s"' % rust) not in text:
                report.add(d["name"], rust, "visibility", "the C# shows and hides it, and the Rust rules never mention it")


def run_dump(args):
    if args.tables:
        return json.loads(pathlib.Path(args.tables).read_text())
    env = dict(os.environ)
    env.setdefault("RUSTUP_HOME", str(ROOT / ".tools/rustup"))
    env.setdefault("CARGO_HOME", str(ROOT / ".tools/cargo"))
    env["PATH"] = str(ROOT / ".tools/cargo/bin") + os.pathsep + env.get("PATH", "")
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/fx-editor"))
    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "vixen-effect-registry", "--example", "dump_effect_tables"],
        cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    )
    if out.returncode != 0:
        sys.stderr.write(out.stderr)
        sys.exit(2)
    return json.loads(out.stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--tables", help="a saved dump from dump_effect_tables, instead of building one")
    parser.add_argument("-v", "--verbose", action="store_true", help="also print accepted differences and defaults it could not evaluate")
    parser.add_argument("--effect", help="only this effect")
    parser.add_argument("--findings-json", help="also write the findings with their expected values to this file")
    args = parser.parse_args()

    tables = run_dump(args)
    index = {i["name"]: i for i in json.loads((ROOT / "docs/effect_index.json").read_text())}
    report = Report()
    missing_classes = []
    for d in tables:
        if args.effect and d["name"] != args.effect:
            continue
        info = index.get(d["name"])
        if info is None:
            missing_classes.append(d["name"])
            continue
        cls, near = module_class(info)
        if cls is None:
            missing_classes.append(d["name"])
            continue
        data_cls = find_class(info["data_class"], near) if info.get("data_class") else None
        chain_data = data_chain_members(data_cls) if data_cls is not None else None
        check_descriptor(report, d, cls, near)
        check_table(report, d["name"], d["params"], cls, near, chain_data, PROPERTY_ALIASES)
        check_order(report, d["name"], d["params"], cls, PROPERTY_ALIASES)
        check_enums(report, d["name"], d["params"], near)
        if data_cls is not None:
            check_defaults(report, d["name"], d["params"], data_cls, near)
        check_visibility_names(report, d, info, cls, PROPERTY_ALIASES)
        # The rows of list parameters.
        for p in d["params"]:
            if p["kind"]["kind"] in ("Record", "RecordList"):
                record = p["kind"]["record"]
                row = ROW_CLASSES.get(record["xml_type"])
                data = find_class(record["xml_type"], near)
                label = "%s/%s" % (d["name"], record["xml_type"])
                if row:
                    row_cls = find_class(row, near)
                    report.checked("row class")
                    if row_cls is None:
                        report.add(label, "(class)", "row", "no C# class " + row)
                        continue
                    check_table(report, label, record["params"], row_cls, near, data_chain_members(data) if data else None, PROPERTY_ALIASES, is_row=True)
                    report.checked("row visibility")
                    if browsable_names(row_cls) and not record["has_visibility"]:
                        report.add(label, "(class)", "row-visibility", "%s shows and hides properties, and the record has no visibility rule" % row)
                    check_row_defaults(report, label, record, row_cls, data, near)

    width = 28
    print("Effect editor metadata check: C# upstream against the Rust tables\n")
    print("effects compared: %d" % len(tables))
    print("values compared:  %d" % sum(report.counts.values()))
    for check, n in sorted(report.counts.items()):
        print("  %-*s %6d" % (width, check, n))
    print("accepted differences: %d" % len(report.accepted))
    print("defaults it could not evaluate: %d" % len(report.unevaluated))
    if missing_classes:
        print("effects with no C# class found: %s" % ", ".join(missing_classes))
    if args.verbose:
        for e, n, c, m, why in report.accepted:
            print("accepted  %s.%s [%s] %s: %s" % (e, n, c, m, why))
        for e, n, why in report.unevaluated:
            print("not evaluated  %s.%s: %s" % (e, n, why[:120]))
    print("findings: %d" % len(report.findings))
    by_check = collections.Counter(f[2] for f in report.findings)
    for check, n in by_check.most_common():
        print("  %-*s %6d" % (width, check, n))
    for e, n, c, m in report.findings:
        print("%s.%s [%s] %s" % (e, n, c, m[:400]))
    if args.findings_json:
        pathlib.Path(args.findings_json).write_text(json.dumps(report.expected, indent=1))
    return 1 if report.findings or missing_classes else 0


if __name__ == "__main__":
    sys.exit(main())
