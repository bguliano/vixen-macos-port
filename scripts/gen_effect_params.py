#!/usr/bin/env python3
"""Generates crates/vixen-effects/src/generated/* from the upstream C# effect modules.

It reads each effect's descriptor, data class and property attributes, plus the three effect
resource files, so category names, labels, slider ranges and XML member names match upstream.

The output is a starting point: anything it cannot translate is marked `TODO(gen)`, and hand
edits to the generated files are expected. Re-running overwrites them, so don't re-run casually.
"""
import json
import pathlib
import re
import sys
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "upstream/Vixen/src"
EFFECTS = SRC / "Vixen.Modules/Effect"
CRATES = ROOT / "crates"
BATCHES = {
    "fx-basic": "pulse set_level chase spin alternating wipe twinkle dissolve strobe candle custom_value launcher rds",
    "fx-pixel-simple": "butterfly color_wash curtain fire garlands life plasma snow_storm spiral spirograph tree circles",
    "fx-pixel-motion": "bars pin_wheel balls fireworks meteors shockwave",
    "fx-pixel-graphics": "snowflakes border shapes text count_down picture video glediator",
    "fx-pixel-complex": "whirlpool wave pattern",
    "fx-pixel-physics": "morph liquid",
    "fx-audio-state": "waveform vertical_meter vu_meter lip_sync state",
    "fx-fixture": "line_dance fixture spin_color_wheel gobo set_position fixture_strobe prism frost set_zoom",
}
BATCH_OF = {m: c for c, ms in BATCHES.items() for m in ms.split()}
ATTR_DIR = SRC / "Vixen.Modules/EffectEditor/EffectDescriptorAttributes"

KNOWN_COLORS = {
    "AliceBlue": (240, 248, 255), "Aqua": (0, 255, 255), "Black": (0, 0, 0), "Blue": (0, 0, 255),
    "BlueViolet": (138, 43, 226), "Brown": (165, 42, 42), "Coral": (255, 127, 80),
    "CornflowerBlue": (100, 149, 237), "Crimson": (220, 20, 60), "Cyan": (0, 255, 255),
    "DarkBlue": (0, 0, 139), "DarkGray": (169, 169, 169), "DarkGreen": (0, 100, 0),
    "DarkOrange": (255, 140, 0), "DarkRed": (139, 0, 0), "DeepPink": (255, 20, 147),
    "DeepSkyBlue": (0, 191, 255), "DimGray": (105, 105, 105), "DodgerBlue": (30, 144, 255),
    "ForestGreen": (34, 139, 34), "Gold": (255, 215, 0), "Gray": (128, 128, 128),
    "Green": (0, 128, 0), "GreenYellow": (173, 255, 47), "HotPink": (255, 105, 180),
    "Indigo": (75, 0, 130), "LawnGreen": (124, 252, 0), "LightBlue": (173, 216, 230),
    "LightGray": (211, 211, 211), "LightGreen": (144, 238, 144), "LightYellow": (255, 255, 224),
    "Lime": (0, 255, 0), "LimeGreen": (50, 205, 50), "Magenta": (255, 0, 255), "Maroon": (128, 0, 0),
    "Navy": (0, 0, 128), "Olive": (128, 128, 0), "Orange": (255, 165, 0), "OrangeRed": (255, 69, 0),
    "Orchid": (218, 112, 214), "Pink": (255, 192, 203), "Purple": (128, 0, 128), "Red": (255, 0, 0),
    "RoyalBlue": (65, 105, 225), "SeaGreen": (46, 139, 87), "Silver": (192, 192, 192),
    "SkyBlue": (135, 206, 235), "SteelBlue": (70, 130, 180), "Teal": (0, 128, 128),
    "Tomato": (255, 99, 71), "Turquoise": (64, 224, 208), "Violet": (238, 130, 238),
    "White": (255, 255, 255), "Yellow": (255, 255, 0), "YellowGreen": (154, 205, 50),
    "Snow": (255, 250, 250), "LightSalmon": (255, 160, 122), "Salmon": (250, 128, 114),
}


def read(path: pathlib.Path) -> str:
    text = path.read_bytes().decode("utf-8-sig", errors="replace").replace("\r", "")
    return text


def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"(^|[^:\"])//.*$", r"\1", text, flags=re.M)


def load_resx(name: str) -> dict:
    out = {}
    for d in ET.parse(ATTR_DIR / name).getroot().findall("data"):
        v = d.find("value")
        if v is not None and v.text is not None:
            out[d.get("name")] = v.text
    return out


CATEGORIES = load_resx("EffectCategoryDescriptors.resx")
DISPLAY_NAMES = load_resx("EffectDisplayNameDescriptors.resx")
DESCRIPTIONS = load_resx("EffectDescriptionDescriptors.resx")


def rust_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def snake(name: str) -> str:
    s = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s).lower()
    return {"move": "move_", "type": "type_", "box": "box_"}.get(s, s)


# ---------------------------------------------------------------------------- enums

def scan_enums() -> dict:
    enums = {}
    roots = [EFFECTS, SRC / "Vixen.Core", SRC / "Vixen.Modules/App", SRC / "Vixen.Common/Controls"]
    for root in roots:
        for f in root.rglob("*.cs"):
            if f.name.endswith(".Designer.cs"):
                continue
            text = strip_comments(read(f))
            for m in re.finditer(r"\benum\s+(\w+)\s*(?::\s*\w+\s*)?\{(.*?)\}", text, flags=re.S):
                name, body = m.group(1), m.group(2)
                members = []
                for part in body.split(","):
                    desc = None
                    d = re.search(r'\[\s*(?:System\.ComponentModel\.)?Description\s*\(\s*@?"([^"]*)"\s*\)\s*\]', part)
                    if d:
                        desc = d.group(1)
                    enum_member = re.search(r'\[\s*EnumMember\s*\(\s*Value\s*=\s*"([^"]*)"\s*\)\s*\]', part)
                    cleaned = re.sub(r"\[.*?\]", "", part, flags=re.S).strip()
                    cleaned = cleaned.split("=")[0].strip()
                    if not cleaned or not re.fullmatch(r"\w+", cleaned):
                        continue
                    members.append((enum_member.group(1) if enum_member else cleaned, desc or cleaned))
                enums.setdefault(name, []).append((f, members))
    return enums


ENUMS = scan_enums()


def find_enum(name: str, near: pathlib.Path):
    options = ENUMS.get(name)
    if not options:
        return None
    for f, members in options:
        if str(f).startswith(str(near)):
            return members
    return options[0][1]


# ---------------------------------------------------------------------------- defaults

def split_args(s: str) -> list:
    out, depth, cur, in_str = [], 0, "", False
    for ch in s:
        if ch == '"':
            in_str = not in_str
        if not in_str:
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            elif ch == "," and depth == 0:
                out.append(cur.strip())
                cur = ""
                continue
        cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def inner(expr: str, opener: str = "(", closer: str = ")"):
    i = expr.find(opener)
    j = expr.rfind(closer)
    if i < 0 or j < i:
        return None
    return expr[i + 1:j]


def number(s: str):
    s = s.strip().rstrip("dDfFmM")
    if s.startswith("."):
        s = "0" + s
    if s.startswith("-."):
        s = "-0" + s[1:]
    try:
        return float(s)
    except ValueError:
        return None


def f64(v: float) -> str:
    return repr(float(v))


def color_expr(e: str):
    e = e.strip()
    m = re.fullmatch(r"(?:System\.Drawing\.)?Color\.(\w+)", e)
    if m:
        n = m.group(1)
        if n == "Empty":
            return "Color::EMPTY"
        if n == "Transparent":
            return "Color::TRANSPARENT"
        if n in KNOWN_COLORS:
            return "Color::rgb(%d, %d, %d)" % KNOWN_COLORS[n]
        return None
    m = re.fullmatch(r"(?:System\.Drawing\.)?Color\.FromArgb\((.*)\)", e)
    if m:
        a = [x.strip() for x in split_args(m.group(1))]
        if len(a) == 3:
            return "Color::from_rgb_i32(%s, %s, %s)" % tuple(a)
        if len(a) == 4:
            return "Color::from_argb_i32(%s, %s, %s, %s)" % tuple(a)
    return None


def curve_expr(e: str):
    e = re.sub(r"\s+", " ", e.strip())
    if e in ("new Curve()", "new()"):
        return "Curve::default()"
    m = re.fullmatch(r"new (?:Curve)?\(CurveType\.(\w+)\)", e)
    if m:
        return {"RampUp": "Curve::ramp_up()", "RampDown": "Curve::ramp_down()", "Flat100": "Curve::flat100()"}.get(m.group(1))
    m = re.fullmatch(r"new (?:Curve)?\((-?[\d.]+)[dDfF]?\)", e)
    if m:
        return "Curve::flat(%s)" % f64(number(m.group(1)))
    m = re.search(r"new\s*\[\]\s*\{([^}]*)\}\s*,\s*new\s*\[\]\s*\{([^}]*)\}", e)
    if m:
        xs = [number(x) for x in split_args(m.group(1))]
        ys = [number(y) for y in split_args(m.group(2))]
        if None not in xs and None not in ys and len(xs) == len(ys):
            pts = ", ".join("(%s, %s)" % (f64(x), f64(y)) for x, y in zip(xs, ys))
            return "Curve::from_points(vec![%s])" % pts
    return None


def gradient_expr(e: str):
    e = re.sub(r"\s+", " ", e.strip())
    if e in ("new ColorGradient()", "new()"):
        return "ColorGradient::default()"
    m = re.fullmatch(r"new (?:ColorGradient)?\((.*)\)", e)
    if m:
        c = color_expr(m.group(1))
        if c:
            return "ColorGradient::solid(%s)" % c
    return None


def level_pair_expr(e: str):
    e = re.sub(r"\s+", " ", e.strip())
    if e in ("new GradientLevelPair()", "new()"):
        return "GradientLevelPair::default()"
    m = re.fullmatch(r"new (?:GradientLevelPair)?\((.*)\)", e)
    if not m:
        return None
    a = split_args(m.group(1))
    if len(a) == 1:
        c = color_expr(a[0])
        if c:
            return "GradientLevelPair::solid(%s)" % c
        g = gradient_expr(a[0])
        if g:
            return "GradientLevelPair::new(%s, Curve::default())" % g
    if len(a) == 2:
        c = color_expr(a[0])
        g = "ColorGradient::solid(%s)" % c if c else gradient_expr(a[0])
        cur = curve_expr(a[1]) or curve_expr("new Curve(%s)" % a[1])
        if g and cur:
            return "GradientLevelPair::new(%s, %s)" % (g, cur)
    return None


def list_items(e: str):
    body = None
    if "{" in e:
        body = inner(e, "{", "}")
    return split_args(body) if body else []


def kind_of(cs_type: str, near: pathlib.Path):
    """Returns (rust ParamKind expr, enum name or None, short tag)."""
    t = cs_type.replace("System.Drawing.", "").replace("System.", "").strip()
    if t.endswith("?"):
        t = t[:-1]
    table = {
        "int": ("ParamKind::Int", "int"), "Int32": ("ParamKind::Int", "int"), "byte": ("ParamKind::Int", "int"),
        "long": ("ParamKind::Long", "long"), "double": ("ParamKind::Double", "double"),
        "Double": ("ParamKind::Double", "double"), "float": ("ParamKind::Double", "double"),
        "bool": ("ParamKind::Bool", "bool"), "Boolean": ("ParamKind::Bool", "bool"),
        "string": ("ParamKind::Str", "str"), "String": ("ParamKind::Str", "str"),
        "Curve": ("ParamKind::Curve", "curve"), "ColorGradient": ("ParamKind::Gradient", "gradient"),
        "List<ColorGradient>": ("ParamKind::GradientList", "gradients"),
        "List<GradientLevelPair>": ("ParamKind::GradientLevels", "levels"),
        "Color": ("ParamKind::Color", "color"), "RGB": ("ParamKind::Rgb", "rgb"),
        "List<Color>": ("ParamKind::ColorList", "colors"), "Percentage": ("ParamKind::Percentage", "percentage"),
        "SerializableFont": ("ParamKind::Font", "font"), "Font": ("ParamKind::Font", "font"),
        "Guid": ("ParamKind::Guid", "guid"), "TimeSpan": ("ParamKind::Time", "time"),
    }
    if t in table:
        k, tag = table[t]
        return k, None, tag
    if find_enum(t, near) is not None:
        return "ParamKind::Enum(&ENUM_%s)" % t, t, "enum"
    return "ParamKind::Other(%s)" % rust_str(t), None, "other"


def default_for(tag: str, enum_name, expr, near):
    """Rust expression for `|| ParamValue::...`. Returns (rust, todo_text_or_None)."""
    e = expr.strip() if expr else None
    if e is not None:
        e = re.sub(r"\s+", " ", e)

    def todo():
        return "ParamValue::None", e

    if tag == "int" or tag == "long":
        pv = "Int" if tag == "int" else "Long"
        if e is None:
            return "ParamValue::%s(0)" % pv, None
        n = number(e)
        return ("ParamValue::%s(%d)" % (pv, n), None) if n is not None else todo()
    if tag == "double":
        if e is None:
            return "ParamValue::Double(0.0)", None
        n = number(e)
        return ("ParamValue::Double(%s)" % f64(n), None) if n is not None else todo()
    if tag == "bool":
        if e is None:
            return "ParamValue::Bool(false)", None
        return ("ParamValue::Bool(%s)" % e, None) if e in ("true", "false") else todo()
    if tag == "str":
        if e is None or e in ("null", "String.Empty", "string.Empty"):
            return 'ParamValue::Str(String::new())', None
        m = re.fullmatch(r'@?"(.*)"', e)
        return ('ParamValue::Str(%s.to_string())' % rust_str(m.group(1)), None) if m else todo()
    if tag == "enum":
        members = find_enum(enum_name, near)
        if e is None:
            return 'ParamValue::Enum(%s.to_string())' % rust_str(members[0][0] if members else ""), None
        m = re.fullmatch(r"(?:[\w.]+\.)?(\w+)", e)
        if m and any(x[0] == m.group(1) for x in members):
            return 'ParamValue::Enum(%s.to_string())' % rust_str(m.group(1)), None
        return todo()
    if tag == "curve":
        if e is None:
            return "ParamValue::Curve(Curve::default())", None
        c = curve_expr(e)
        return ("ParamValue::Curve(%s)" % c, None) if c else todo()
    if tag == "gradient":
        if e is None:
            return "ParamValue::Gradient(ColorGradient::default())", None
        g = gradient_expr(e)
        return ("ParamValue::Gradient(%s)" % g, None) if g else todo()
    if tag == "gradients":
        if e is None:
            return "ParamValue::GradientList(Vec::new())", None
        gs = [gradient_expr(x) for x in list_items(e)]
        if None in gs:
            return todo()
        return "ParamValue::GradientList(vec![%s])" % ", ".join(gs), None
    if tag == "levels":
        if e is None:
            return "ParamValue::GradientLevels(Vec::new())", None
        gs = [level_pair_expr(x) for x in list_items(e)]
        if None in gs:
            return todo()
        return "ParamValue::GradientLevels(vec![%s])" % ", ".join(gs), None
    if tag == "color":
        if e is None:
            return "ParamValue::Color(Color::EMPTY)", None
        c = color_expr(e)
        return ("ParamValue::Color(%s)" % c, None) if c else todo()
    if tag == "rgb":
        c = color_expr(e) if e else None
        if c:
            return "ParamValue::Rgb(vixen_core::color::Rgb::from_color(%s))" % c, None
        return "ParamValue::Rgb(vixen_core::color::Rgb::new(1.0, 1.0, 1.0))", e
    if tag == "colors":
        if e is None:
            return "ParamValue::ColorList(Vec::new())", None
        cs = [color_expr(x) for x in list_items(e)]
        if None in cs:
            return todo()
        return "ParamValue::ColorList(vec![%s])" % ", ".join(cs), None
    if tag == "percentage":
        if e is None:
            return "ParamValue::Percentage(0.0)", None
        n = number(inner(e) or e)
        return ("ParamValue::Percentage(%s)" % f64(n), None) if n is not None else todo()
    if tag == "font":
        if e:
            m = re.search(r'new Font\("([^"]+)",\s*([\d.]+)', e)
            if m:
                return 'ParamValue::Font(FontDesc::new(%s, %s))' % (rust_str(m.group(1)), repr(float(m.group(2))) + "_f32"), None
        return 'ParamValue::Font(FontDesc::new("Arial", 8.0))', e
    if tag == "guid":
        return "ParamValue::Guid(Uuid::nil())", None
    if tag == "time":
        if e is None or e == "TimeSpan.Zero":
            return "ParamValue::Time(TimeSpan::ZERO)", None
        return "ParamValue::Time(TimeSpan::ZERO)", e
    return "ParamValue::Raw(String::new())", e


# ---------------------------------------------------------------------------- class parsing

def find_class_files(name: str, root: pathlib.Path) -> list:
    out = []
    pat = re.compile(r"\bclass\s+%s\b" % re.escape(name))
    for f in root.rglob("*.cs"):
        if f.name.endswith(".Designer.cs"):
            continue
        if pat.search(strip_comments(read(f))):
            out.append(f)
    return out


def namespace_of(text: str) -> str:
    m = re.search(r"\bnamespace\s+([\w.]+)", text)
    return m.group(1) if m else ""


def parse_data_class(name: str, search_roots: list):
    """Returns a chain of (class name, namespace, contract name, contract ns, members, defaults), base first."""
    chain = []
    cur = name
    seen = set()
    while cur and cur not in seen and cur not in ("ModuleDataModelBase",):
        seen.add(cur)
        files = []
        for r in search_roots:
            files = find_class_files(cur, r)
            if files:
                break
        if not files:
            break
        text = strip_comments(read(files[0]))
        m = re.search(r"\bclass\s+%s\b\s*(?::\s*([\w.<>]+))?" % re.escape(cur), text)
        parent = m.group(1) if m else None
        contract = re.search(r"\[\s*DataContract\s*\(([^)]*)\)\s*\]\s*(?:\[[^\]]*\]\s*)*(?:public|internal|private|protected)?\s*(?:abstract\s+|sealed\s+)?(?:partial\s+)?class\s+%s\b" % re.escape(cur), text)
        contract_name, contract_ns = cur, namespace_of(text)
        if contract:
            n = re.search(r'Name\s*=\s*"([^"]*)"', contract.group(1))
            s = re.search(r'Namespace\s*=\s*"([^"]*)"', contract.group(1))
            if n:
                contract_name = n.group(1)
            if s:
                contract_ns = s.group(1)
        members = []
        for mm in re.finditer(r"\[\s*DataMember\s*(?:\(([^)]*)\))?\s*\]\s*(?:\[[^\]]*\]\s*)*(?:public|private|protected|internal)\s+([\w.<>,\[\]? ]+?)\s+(\w+)\s*(?:\{|;|=)", text):
            attrs = mm.group(1) or ""
            n = re.search(r'Name\s*=\s*"([^"]*)"', attrs)
            members.append({"type": mm.group(2).strip(), "name": mm.group(3), "xml": n.group(1) if n else mm.group(3)})
        defaults = {}
        ctor = re.search(r"\b%s\s*\(\s*\)\s*(?::\s*\w+\s*\([^)]*\)\s*)?\{" % re.escape(cur), text)
        if ctor:
            depth, i = 1, ctor.end()
            while i < len(text) and depth:
                depth += {"{": 1, "}": -1}.get(text[i], 0)
                i += 1
            body = text[ctor.end():i - 1]
            for stmt in re.finditer(r"(?:this\.)?(\w+)\s*=\s*((?:[^;\"]|\"[^\"]*\")+);", body):
                defaults[stmt.group(1)] = stmt.group(2).strip()
        for mm in re.finditer(r"\bpublic\s+[\w.<>,\[\]? ]+?\s+(\w+)\s*\{\s*get;\s*set;\s*\}\s*=\s*((?:[^;\"]|\"[^\"]*\")+);", text):
            defaults.setdefault(mm.group(1), mm.group(2).strip())
        chain.insert(0, {"class": cur, "ns": namespace_of(text), "contract_name": contract_name, "contract_ns": contract_ns, "members": members, "defaults": defaults, "file": files[0]})
        cur = parent
    return chain


def attr_value(attrs: list, name: str):
    for a in attrs:
        m = re.fullmatch(r"\[\s*%s(?:Attribute)?\s*(?:\((.*)\))?\s*\]" % name, a, flags=re.S)
        if m:
            return m.group(1) if m.group(1) is not None else ""
    return None


def parse_properties(files: list) -> list:
    props = []
    for f in files:
        lines = strip_comments(read(f)).split("\n")
        attrs = []
        i = 0
        while i < len(lines):
            line = lines[i].strip()
            if line.startswith("[") and line.endswith("]"):
                # A line can hold several attributes.
                for a in re.findall(r"\[[^\[\]]*\]", line):
                    attrs.append(a)
                i += 1
                continue
            m = re.match(r"public\s+(?:override\s+|virtual\s+|new\s+|abstract\s+|sealed\s+)*([\w.<>,\[\]? ]+?)\s+(\w+)\s*(\{.*|=>.*)?$", line)
            if m and "(" not in line.split("{")[0] and "class " not in line and not line.endswith(";") or (m and line.endswith("{ get; set; }")):
                ptype, pname = m.group(1).strip(), m.group(2)
                j = i
                depth = 0
                started = False
                body = []
                while j < len(lines):
                    body.append(lines[j])
                    depth += lines[j].count("{") - lines[j].count("}")
                    if "{" in lines[j]:
                        started = True
                    if started and depth <= 0:
                        break
                    if not started and j > i + 1:
                        break
                    j += 1
                text = "\n".join(body)
                getter = re.search(r"\bget\b\s*(\{.*?\}|=>[^;]*;)", text, flags=re.S)
                data_field = None
                if getter:
                    g = re.search(r"\b(?:_data|EffectModuleData|_effectData)\s*\.\s*(\w+)", getter.group(1))
                    if g:
                        data_field = g.group(1)
                props.append({"type": ptype, "name": pname, "attrs": attrs, "data_field": data_field, "file": f, "body": text})
                attrs = []
                i = max(j, i) + 1
                continue
            if line and not line.startswith("#"):
                attrs = []
            i += 1
    return props


# ---------------------------------------------------------------------------- descriptors

def field(text: str, name: str):
    for pat in (
        r"\b%s\s*\{\s*get\s*\{\s*return\s+@?\"([^\"]*)\"" % name,
        r"\b%s\s*=>\s*@?\"([^\"]*)\"" % name,
        r"\b%s\b[^{;=]*\{[^}]*?return\s+@?\"([^\"]*)\"" % name,
    ):
        m = re.search(pat, text, flags=re.S)
        if m:
            return m.group(1)
    return ""


def type_field(text: str, name: str):
    for pat in (
        r"\b%s\b[^{;=]*\{[^}]*?typeof\s*\(\s*([\w.]+)\s*\)" % name,
        r"\b%s\s*=>\s*typeof\s*\(\s*([\w.]+)\s*\)" % name,
    ):
        m = re.search(pat, text, flags=re.S)
        if m:
            return m.group(1).split(".")[-1]
    return None


def main() -> int:
    index = []
    modules = []
    for f in sorted(EFFECTS.rglob("*.cs")):
        if f.name.endswith(".Designer.cs"):
            continue
        text = strip_comments(read(f))
        if not re.search(r"class\s+\w+\s*:\s*(?:[\w.]*)EffectModuleDescriptorBase", text):
            continue
        name = field(text, "EffectName")
        guid = re.search(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}", text)
        group = re.search(r"EffectGroups\.(\w+)", text)
        module_class = type_field(text, "ModuleClass")
        data_class = type_field(text, "ModuleDataClass")
        if not (name and guid and module_class):
            print("SKIP", f, name, guid, module_class, file=sys.stderr)
            continue
        effect_dir = f.parent
        # A few modules keep the descriptor in a nested project; look up to the effect's own folder.
        rel = f.relative_to(EFFECTS).parts
        top_dir = EFFECTS / rel[0]
        if rel[0] == "Fixture":
            top_dir = EFFECTS / rel[0] / rel[1]
        module_files = find_class_files(module_class, top_dir) or find_class_files(module_class, EFFECTS)
        props = parse_properties(module_files)
        base_effect_file = EFFECTS / "Effect/PixelEffectBase.cs"
        is_pixel = any(re.search(r"class\s+%s\s*:\s*PixelEffectBase" % module_class, strip_comments(read(m))) for m in module_files)
        if is_pixel:
            base_props = {p["name"]: p for p in parse_properties([base_effect_file])}
            for p in props:
                if p["name"] in base_props:
                    p["attrs"] = base_props[p["name"]]["attrs"] + p["attrs"]
            props = [base_props["TargetPositioning"]] + props
        chain = parse_data_class(data_class, [top_dir, EFFECTS, SRC / "Vixen.Modules"]) if data_class else []
        leaf = chain[-1] if chain else None

        # Merge data members and properties.
        by_field = {}
        for p in props:
            if p["data_field"]:
                by_field.setdefault(p["data_field"], p)

        params = []
        enums_used = {}
        todos = []
        seen_names = set()
        for level, cls in enumerate(chain):
            for m in sorted(cls["members"], key=lambda x: x["xml"]):
                p = by_field.get(m["name"])
                pname = p["name"] if p else m["name"]
                kind, enum_name, tag = kind_of(m["type"], top_dir)
                if enum_name:
                    enums_used[enum_name] = find_enum(enum_name, top_dir)
                default_expr = None
                for c in reversed(chain):
                    if m["name"] in c["defaults"]:
                        default_expr = c["defaults"][m["name"]]
                        break
                rust_default, todo = default_for(tag, enum_name, default_expr, top_dir)
                if todo:
                    todos.append("%s = %s" % (m["name"], todo))
                params.append({"p": p, "name": pname, "xml": m["xml"], "kind": kind, "tag": tag, "default": rust_default, "todo": todo, "level": level, "ns": cls["contract_ns"], "cs_type": m["type"]})
                seen_names.add(pname)
        # Editor-visible properties with no backing data member.
        for p in props:
            if p["name"] in seen_names or attr_value(p["attrs"], "ProviderDisplayName") is None:
                continue
            kind, enum_name, tag = kind_of(p["type"], top_dir)
            if enum_name:
                enums_used[enum_name] = find_enum(enum_name, top_dir)
            rust_default, todo = default_for(tag, enum_name, None, top_dir)
            params.append({"p": p, "name": p["name"], "xml": "", "kind": kind, "tag": tag, "default": rust_default, "todo": None, "level": 99, "ns": "", "cs_type": p["type"]})
            seen_names.add(p["name"])

        mod = snake(re.sub(r"Module$", "", module_class)) if module_class else ""
        if not mod:
            mod = snake(name.replace(" ", "").replace("-", ""))
        if mod in modules:
            mod = snake(name.replace(" ", ""))
        modules.append(mod)
        icon = next((x for x in top_dir.rglob("EffectImage*.png")), None)
        out = []
        out.append("//! Generated by scripts/gen_effect_params.py from upstream %s. Hand edits are fine." % module_class)
        out.append("#![allow(non_upper_case_globals, unused_imports, clippy::approx_constant)]")
        out.append("use vixen_effects::framework::*;")
        out.append("use vixen_core::*;")
        out.append("")
        out.append("pub const TYPE_ID: Uuid = uuid::uuid!(%s);" % rust_str(guid.group(0).lower()))
        out.append("pub const EFFECT_NAME: &str = %s;" % rust_str(name))
        out.append("pub const GROUP: EffectGroup = EffectGroup::%s;" % (group.group(1) if group else "Basic"))
        out.append("pub const AUTHOR: &str = %s;" % rust_str(field(text, "Author")))
        out.append("pub const DESCRIPTION: &str = %s;" % rust_str(field(text, "Description")))
        out.append("pub const VERSION: &str = %s;" % rust_str(field(text, "Version")))
        out.append("pub const DATA_TYPE: &str = %s;" % rust_str(leaf["contract_name"] if leaf else ""))
        out.append("pub const DATA_NS: &str = %s;" % rust_str(leaf["contract_ns"] if leaf else ""))
        out.append("pub const ICON: &str = %s;" % rust_str(str(icon.relative_to(SRC)) if icon else ""))
        link = ""
        for mf in module_files:
            mm = re.search(r"InformationLink\b[^{=]*(?:\{[^}]*?return\s+|=>\s*)@?\"([^\"]*)\"", strip_comments(read(mf)), flags=re.S)
            if mm:
                link = mm.group(1)
                break
        out.append("pub const INFO_LINK: &str = %s;" % rust_str(link))
        out.append("")
        for en, members in sorted(enums_used.items()):
            body = ", ".join("EnumMember { name: %s, label: %s }" % (rust_str(a), rust_str(b)) for a, b in members)
            out.append("pub static ENUM_%s: EnumDesc = EnumDesc { name: %s, members: &[%s] };" % (en, rust_str(en), body))
        if enums_used:
            out.append("")
        out.append("pub static PARAMS: [ParamDesc; %d] = [" % len(params))
        for x in params:
            p = x["p"]
            attrs = p["attrs"] if p else []
            cat_key, cat_order = "", 0
            cat = attr_value(attrs, "ProviderCategory")
            if cat:
                a = split_args(cat)
                cat_key = a[0].lstrip("@").strip('"')
                if len(a) > 1:
                    try:
                        cat_order = int(a[1])
                    except ValueError:
                        pass
            lab = attr_value(attrs, "ProviderDisplayName")
            lab_key = lab.lstrip("@").strip('"') if lab else ""
            des = attr_value(attrs, "ProviderDescription")
            des_key = des.lstrip("@").strip('"') if des else ""
            order = attr_value(attrs, "PropertyOrder")
            try:
                order_n = int(order) if order is not None else 0
            except ValueError:
                order_n = 0
            browsable = attr_value(attrs, "Browsable")
            has_ui = lab is not None
            is_browsable = has_ui and (browsable is None or "false" not in browsable)
            editor = "Editor::Default"
            pe = attr_value(attrs, "PropertyEditor")
            rng = attr_value(attrs, "NumberRange")
            if pe:
                pe = pe.strip('"')
                nums = [number(v) for v in split_args(rng)] if rng else []
                nums = nums if len(nums) >= 2 and None not in nums else None
                if pe == "SliderEditor":
                    editor = "Editor::Slider { min: %s, max: %s, step: %s }" % (f64(nums[0]), f64(nums[1]), f64(nums[2] if len(nums) > 2 else 1)) if nums else "Editor::Slider { min: 0.0, max: 100.0, step: 1.0 }"
                elif pe in ("SliderDoubleEditor", "DoubleSliderEditor"):
                    editor = "Editor::SliderDouble { min: %s, max: %s, step: %s }" % (f64(nums[0]), f64(nums[1]), f64(nums[2] if len(nums) > 2 else 0.1)) if nums else "Editor::SliderDouble { min: 0.0, max: 100.0, step: 0.1 }"
                elif pe == "LevelEditor":
                    editor = "Editor::Level"
                elif pe == "SelectionEditor":
                    editor = "Editor::Selection"
                elif pe == "Label":
                    editor = "Editor::Label"
                elif pe.endswith("PathEditor"):
                    editor = 'Editor::Path(%s)' % rust_str(pe[: -len("PathEditor")])
            elif rng:
                nums = [number(v) for v in split_args(rng)]
                if len(nums) >= 2 and None not in nums:
                    editor = "Editor::Slider { min: %s, max: %s, step: %s }" % (f64(nums[0]), f64(nums[1]), f64(nums[2] if len(nums) > 2 else 1))
            comment = "  // TODO(gen) default: %s" % x["todo"].replace("\n", " ") if x["todo"] else ""
            out.append("    ParamDesc {")
            out.append("        name: %s," % rust_str(x["name"]))
            out.append("        xml: %s," % rust_str(x["xml"]))
            out.append("        xml_ns: %s," % rust_str(x["ns"]))
            out.append("        xml_level: %d," % x["level"])
            out.append("        kind: %s," % x["kind"])
            out.append("        category: %s," % rust_str(CATEGORIES.get(cat_key, cat_key)))
            out.append("        category_order: %d," % cat_order)
            out.append("        label: %s," % rust_str(DISPLAY_NAMES.get(lab_key, lab_key)))
            out.append("        description: %s," % rust_str(DESCRIPTIONS.get(des_key, des_key)))
            out.append("        order: %d," % order_n)
            out.append("        browsable: %s," % ("true" if is_browsable else "false"))
            out.append("        is_value: %s," % ("true" if attr_value(attrs, "Value") is not None else "false"))
            out.append("        exclude_from_default: %s," % ("true" if attr_value(attrs, "ExcludeFromEffectDefault") is not None else "false"))
            out.append("        editor: %s," % editor)
            out.append("        default: || %s,%s" % (x["default"], comment))
            out.append("    },")
        out.append("];")
        out.append("")
        out.append("/// All the table data. Effect modules override `render` and friends with struct update syntax.")
        out.append("pub const BASE: EffectDescriptor = EffectDescriptor {")
        out.append("    type_id: TYPE_ID,")
        out.append("    name: EFFECT_NAME,")
        out.append("    group: GROUP,")
        out.append("    author: AUTHOR,")
        out.append("    description: DESCRIPTION,")
        out.append("    version: VERSION,")
        out.append("    data_type: DATA_TYPE,")
        out.append("    data_ns: DATA_NS,")
        out.append("    info_link: INFO_LINK,")
        out.append("    icon: ICON,")
        out.append("    params: &PARAMS,")
        out.append("    render: not_ported,")
        out.append("    visibility: None,")
        out.append("    param_changed: None,")
        out.append("    validate: None,")
        out.append("    force_visual: false,")
        out.append("    visual: None,")
        out.append("    supports_location: false,")
        out.append("    ported: false,")
        out.append("};")
        batch = BATCH_OF[mod]
        gen_dir = CRATES / batch / "src/generated"
        gen_dir.mkdir(parents=True, exist_ok=True)
        (gen_dir / (mod + ".rs")).write_text("\n".join(out) + "\n")

        csharp_lines = sum(len(read(p).split("\n")) for p in top_dir.rglob("*.cs") if not p.name.endswith(".Designer.cs"))
        index.append({
            "module": mod, "name": name, "class": module_class, "data_class": data_class,
            "group": group.group(1) if group else "Basic", "is_pixel": is_pixel, "dir": str(top_dir.relative_to(SRC)),
            "module_files": [str(m.relative_to(SRC)) for m in module_files], "csharp_lines": csharp_lines,
            "params": len(params), "todos": todos,
        })

    for batch, ms in BATCHES.items():
        ms = sorted(ms.split())
        crate = CRATES / batch
        (crate / "src/generated/mod.rs").write_text("".join("pub mod %s;\n" % m for m in ms))
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "%s"\nversion.workspace = true\nedition.workspace = true\n\n'
            '[dependencies]\nvixen-core = { path = "../vixen-core" }\nvixen-effects = { path = "../vixen-effects" }\n'
            'uuid.workspace = true\n' % batch)
        lib = crate / "src/lib.rs"
        if not lib.exists():
            lines = ["//! Effect modules: %s. Generated tables live in `generated`.\n" % ", ".join(ms), "\n", "pub mod generated;\n"]
            lines += ["pub mod %s;\n" % m for m in ms]
            lines += ["\nuse vixen_effects::framework::EffectDescriptor;\n\npub fn descriptors() -> Vec<&'static EffectDescriptor> {\n    vec!["]
            lines += ["&%s::DESCRIPTOR, " % m for m in ms]
            lines += ["]\n}\n"]
            lib.write_text("".join(lines))
        for m in ms:
            f = crate / "src" / (m + ".rs")
            if not f.exists():
                f.write_text(
                    "use crate::generated::%s as gen;\nuse vixen_effects::framework::EffectDescriptor;\n\n"
                    "pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor { ..gen::BASE };\n" % m)
    (ROOT / "docs/effect_index.json").write_text(json.dumps(index, indent=1))
    total_todo = sum(len(i["todos"]) for i in index)
    print("generated %d effects, %d parameters, %d defaults need attention" % (len(index), sum(i["params"] for i in index), total_todo))
    for i in index:
        for t in i["todos"]:
            print("   ", i["module"], ":", t[:110])
    return 0


if __name__ == "__main__":
    sys.exit(main())
