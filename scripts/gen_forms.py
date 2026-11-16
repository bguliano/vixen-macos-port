#!/usr/bin/env python3
"""Extracts every WinForms *.Designer.cs in upstream Vixen into JSON, as a parity reference.

The SwiftUI windows are written by hand with Mac conventions, but each one must keep the original's
controls, labels, grouping and rough placement. These files list exactly what the original has: controls,
geometry, menus, tooltips and event handler names. They are a checklist and are not used at run time.

Output: docs/reference/forms/<upstream path with / replaced by __>.json, one per designer class, and
docs/reference/forms/_index.json mapping class names to files. Images embedded in .resx files are
extracted to docs/reference/form-images/.
"""
import base64
import json
import pathlib
import re
import sys
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "upstream/Vixen/src"
OUT = ROOT / "docs/reference/forms"
RESX_IMAGES = ROOT / "docs/reference/form-images"

IGNORED_CALLS = {
    "SuspendLayout", "ResumeLayout", "PerformLayout", "BeginInit", "EndInit", "Dispose", "BeginUpdate", "EndUpdate",
}


def read(path: pathlib.Path) -> str:
    return path.read_bytes().decode("utf-8-sig", errors="replace").replace("\r", "")


def split_args(s: str) -> list:
    out, depth, cur, in_str, esc, verbatim = [], 0, "", False, False, False
    i = 0
    while i < len(s):
        ch = s[i]
        if in_str:
            cur += ch
            if verbatim:
                if ch == '"':
                    if i + 1 < len(s) and s[i + 1] == '"':
                        cur += '"'
                        i += 1
                    else:
                        in_str = False
            elif esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
        else:
            if ch == '"':
                in_str = True
                verbatim = cur.endswith("@")
                cur += ch
            elif ch in "([{":
                depth += 1
                cur += ch
            elif ch in ")]}":
                depth -= 1
                cur += ch
            elif ch == "," and depth == 0:
                out.append(cur.strip())
                cur = ""
            else:
                cur += ch
        i += 1
    if cur.strip():
        out.append(cur.strip())
    return out


def unquote(s: str) -> str:
    verbatim = s.startswith("@")
    body = s[1:] if verbatim else s
    body = body[1:-1]
    if verbatim:
        return body.replace('""', '"')
    return (
        body.replace("\\\\", "\0").replace('\\"', '"').replace("\\n", "\n").replace("\\r", "\r").replace("\\t", "\t")
        .replace("\0", "\\")
    )


def strip_cast(e: str) -> str:
    """Removes outer parens and casts such as ((System.Drawing.Image)(x)) or (Image)x."""
    e = e.strip()
    changed = True
    while changed:
        changed = False
        if e.startswith("(") and matching_paren(e, 0) == len(e) - 1:
            e = e[1:-1].strip()
            changed = True
            continue
        m = re.match(r"^\(\s*([\w.]+(?:\[\])?)\s*\)\s*(.+)$", e, flags=re.S)
        if m and not m.group(1)[0].isdigit() and m.group(1) not in ("true", "false"):
            rest = m.group(2).strip()
            # A cast is followed by an operand, not an operator.
            if rest[0] not in "|&+*/-":
                e = rest
                changed = True
    return e


def matching_paren(s: str, start: int) -> int:
    depth, in_str = 0, False
    for i in range(start, len(s)):
        c = s[i]
        if c == '"' and (i == 0 or s[i - 1] != "\\"):
            in_str = not in_str
        if in_str:
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i
    return -1


def short_type(t: str) -> str:
    t = t.strip()
    t = re.sub(r"<.*>", "", t)
    return t.split(".")[-1]


def value(e: str, resx: dict):
    """Translates a designer expression to JSON."""
    e = strip_cast(e)
    if re.fullmatch(r'@?"(?:[^"\\]|\\.|"")*"', e, flags=re.S):
        return unquote(e)
    if e in ("true", "false"):
        return e == "true"
    if e == "null":
        return None
    m = re.fullmatch(r"-?\d+(?:\.\d+)?(?:[fFdDmMLlUu]*)", e)
    if m:
        n = float(re.sub(r"[fFdDmMLlUu]+$", "", e))
        return int(n) if n.is_integer() and "." not in e else n
    m = re.fullmatch(r"new\s+(?:[\w.]*\.)?(Size|SizeF|Point|PointF)\s*\((.*)\)", e, flags=re.S)
    if m:
        a = [value(x, resx) for x in split_args(m.group(2))]
        return {"t": m.group(1).rstrip("F").lower(), "x": a[0], "y": a[1]} if len(a) == 2 else {"t": "raw", "v": e}
    m = re.fullmatch(r"new\s+(?:[\w.]*\.)?Padding\s*\((.*)\)", e, flags=re.S)
    if m:
        a = [value(x, resx) for x in split_args(m.group(1))]
        if len(a) == 1:
            a = a * 4
        return {"t": "padding", "l": a[0], "t_": a[1], "r": a[2], "b": a[3]}
    m = re.fullmatch(r"new\s+(?:[\w.]*\.)?Rectangle\s*\((.*)\)", e, flags=re.S)
    if m:
        a = [value(x, resx) for x in split_args(m.group(1))]
        return {"t": "rect", "x": a[0], "y": a[1], "w": a[2], "h": a[3]}
    m = re.fullmatch(r"new\s+(?:[\w.]*\.)?Font\s*\((.*)\)", e, flags=re.S)
    if m:
        a = split_args(m.group(1))
        out = {"t": "font", "name": value(a[0], resx) if a else "Segoe UI", "size": value(a[1], resx) if len(a) > 1 else 9}
        for x in a[2:]:
            if "Bold" in x:
                out["bold"] = True
            if "Italic" in x:
                out["italic"] = True
            if "Underline" in x:
                out["underline"] = True
        return out
    m = re.fullmatch(r"(?:System\.Drawing\.)?Color\.FromArgb\((.*)\)", e, flags=re.S)
    if m:
        a = [value(re.sub(r"\(\(int\)\(\(byte\)\((.*)\)\)\)", r"\1", x), resx) for x in split_args(m.group(1))]
        return {"t": "color", "argb": a}
    m = re.fullmatch(r"(?:System\.Drawing\.)?Color\.(\w+)", e)
    if m:
        return {"t": "color", "name": m.group(1)}
    m = re.fullmatch(r"(?:System\.Drawing\.)?SystemColors\.(\w+)", e)
    if m:
        return {"t": "color", "system": m.group(1)}
    # Resources.Name, Properties.Resources.Name, Common.Resources.Properties.Resources.Name...
    m = re.fullmatch(r"(?:[\w.]*\.)?(?:Properties\.)?Resources\.(\w+)", e)
    if m and m.group(1) not in ("ResourceManager",):
        return {"t": "res", "name": m.group(1)}
    m = re.fullmatch(r"resources\.GetObject\(\s*\"([^\"]+)\"\s*\)", e)
    if m:
        v = resx.get(m.group(1))
        return v if v is not None else {"t": "resx", "key": m.group(1)}
    m = re.fullmatch(r"resources\.GetString\(\s*\"([^\"]+)\"\s*\)", e)
    if m:
        return resx.get(m.group(1), "")
    m = re.fullmatch(r"new\s+([\w.<>]+)\s*\[\s*\]\s*\{(.*)\}", e, flags=re.S)
    if m:
        return [value(x, resx) for x in split_args(m.group(2))]
    m = re.fullmatch(r"new\s*\[\s*\]\s*\{(.*)\}", e, flags=re.S)
    if m:
        return [value(x, resx) for x in split_args(m.group(1))]
    if "|" in e and "new" not in e:
        return {"t": "flags", "v": [p.strip().split(".")[-1] for p in e.split("|")]}
    m = re.fullmatch(r"(?:[\w]+\.)*(\w+)\.(\w+)", e)
    if m:
        # Enum values and other member references.
        return {"t": "enum", "type": m.group(1), "v": m.group(2)}
    if re.fullmatch(r"\w+", e):
        return {"t": "ref", "name": e}
    return {"t": "raw", "v": re.sub(r"\s+", " ", e)}


def load_resx(path: pathlib.Path, stem: str) -> dict:
    out = {}
    if not path.exists():
        return out
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError:
        return out
    for d in root.findall("data"):
        name = d.get("name")
        v = d.find("value")
        if name is None or v is None or v.text is None:
            continue
        typ = d.get("type") or ""
        mime = d.get("mimetype") or ""
        if "bytearray.base64" in mime and ("Bitmap" in typ or "Icon" in typ or "Image" in typ):
            try:
                raw = base64.b64decode(v.text)
            except Exception:
                continue
            ext = "png"
            if raw[:2] == b"BM":
                ext = "bmp"
            elif raw[:4] == b"\x00\x00\x01\x00":
                ext = "ico"
            elif raw[:3] == b"\xff\xd8\xff":
                ext = "jpg"
            elif raw[:3] == b"GIF":
                ext = "gif"
            fname = re.sub(r"[^\w.-]+", "_", "%s__%s" % (stem, name)) + "." + ext
            RESX_IMAGES.mkdir(parents=True, exist_ok=True)
            (RESX_IMAGES / fname).write_bytes(raw)
            out[name] = {"t": "image", "file": fname}
        elif "ResXFileRef" in typ:
            continue
        elif typ == "" or "System.String" in typ:
            out[name] = v.text
    return out


def statements(body: str) -> list:
    """Splits a method body into statements, respecting strings and nesting."""
    out, cur, depth, in_str, esc, verbatim = [], "", 0, False, False, False
    i = 0
    while i < len(body):
        c = body[i]
        if in_str:
            cur += c
            if verbatim:
                if c == '"':
                    if i + 1 < len(body) and body[i + 1] == '"':
                        cur += '"'
                        i += 1
                    else:
                        in_str = False
            elif esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
        else:
            if c == '"':
                in_str = True
                verbatim = cur.endswith("@")
                cur += c
            elif c == "/" and i + 1 < len(body) and body[i + 1] == "/":
                j = body.find("\n", i)
                i = j if j >= 0 else len(body)
                continue
            elif c == "/" and i + 1 < len(body) and body[i + 1] == "*":
                j = body.find("*/", i)
                i = j + 2 if j >= 0 else len(body)
                continue
            elif c in "([{":
                depth += 1
                cur += c
            elif c in ")]}":
                depth -= 1
                cur += c
            elif c == ";" and depth == 0:
                if cur.strip():
                    out.append(cur.strip())
                cur = ""
            else:
                cur += c
        i += 1
    return out


def norm_target(t: str) -> str:
    t = t.strip()
    while t.startswith("this."):
        t = t[5:]
    return t


def parse_designer(path: pathlib.Path):
    text = read(path)
    cls = re.search(r"partial\s+class\s+(\w+)", text)
    if not cls:
        return None
    m = re.search(r"void\s+InitializeComponent\s*\(\s*\)\s*\{", text)
    if not m:
        return None
    depth, i = 1, m.end()
    in_str = False
    while i < len(text) and depth:
        c = text[i]
        if c == '"' and text[i - 1] != "\\":
            in_str = not in_str
        if not in_str:
            depth += {"{": 1, "}": -1}.get(c, 0)
        i += 1
    body = text[m.end():i - 1]

    resx = load_resx(path.with_name(path.name.replace(".Designer.cs", ".resx").replace(".designer.cs", ".resx")), cls.group(1))
    form = {"class": cls.group(1), "source": str(path.relative_to(SRC)), "props": {}, "events": {}, "children": []}
    objects = {}

    # Field declarations give precise types when the initialiser is `new()`.
    declared = {}
    for d in re.finditer(r"(?:private|internal|public|protected)\s+([\w.<>]+)\s+(\w+)\s*;", text):
        declared[d.group(2)] = short_type(d.group(1))

    def obj(name: str) -> dict:
        return objects.setdefault(name, {"type": declared.get(name, "Unknown"), "props": {}, "events": {}, "children": []})

    def is_object(name: str) -> bool:
        return name in objects or name in declared

    unknown = []
    for s in statements(body):
        s = re.sub(r"\s+", " ", s) if '"' not in s else s
        # name = new Type(...)
        a = re.match(r"^(?:[\w.<>]+\s+)?((?:this\.)?\w+)\s*=\s*new\s+([\w.<>]+)\s*\((.*)\)\s*(\{.*\})?$", s, flags=re.S)
        if a and "." not in norm_target(a.group(1)):
            name = norm_target(a.group(1))
            # Local variables and fields are treated alike.
            o = obj(name)
            o["type"] = short_type(a.group(2))
            args = split_args(a.group(3))
            if args:
                o["ctor"] = [value(x, resx) for x in args]
            continue
        a = re.match(r"^(?:[\w.<>]+\s+)?((?:this\.)?\w+)\s*=\s*new\s*\(\s*\)$", s)
        if a and "." not in norm_target(a.group(1)):
            obj(norm_target(a.group(1)))
            continue
        # target.Event += handler
        a = re.match(r"^(.+?)\s*\+=\s*(.+)$", s, flags=re.S)
        if a:
            left = norm_target(a.group(1))
            handler = a.group(2).strip()
            h = re.search(r"new\s+[\w.<>]+\s*\(\s*(?:this\.)?(\w+)\s*\)", handler)
            handler = h.group(1) if h else norm_target(handler)
            parts = left.split(".")
            if len(parts) == 1:
                form["events"][parts[0]] = handler
            elif len(parts) == 2 and is_object(parts[0]):
                obj(parts[0])["events"][parts[1]] = handler
            elif len(parts) == 2:
                form["events"][".".join(parts)] = handler
            continue
        # target.Prop.Sub = value / target.Prop = value / Prop = value
        a = re.match(r"^([\w.\[\]]+)\s*=\s*(.+)$", s, flags=re.S)
        if a:
            left = norm_target(a.group(1))
            val = value(a.group(2), resx)
            parts = left.split(".")
            if len(parts) == 1:
                form["props"][parts[0]] = val
            elif is_object(parts[0]):
                obj(parts[0])["props"][".".join(parts[1:])] = val
            else:
                form["props"][".".join(parts)] = val
            continue
        # target.Method(args)
        a = re.match(r"^\(?\(?(?:\([\w.]+\))?([\w.]+)\)?\)?\.(\w+)\s*\((.*)\)$", s, flags=re.S)
        if a or re.match(r"^\w+\s*\(", s):
            if a:
                target = norm_target(a.group(1))
                method = a.group(2)
                args_text = a.group(3)
            else:
                b = re.match(r"^(\w+)\s*\((.*)\)$", s, flags=re.S)
                target, method, args_text = "", b.group(1), b.group(2)
            if method in IGNORED_CALLS:
                continue
            args = split_args(args_text)
            parts = target.split(".") if target else []
            owner = None
            coll = None
            if not parts:
                owner, coll = form, None
            elif parts[-1] in (
                "Controls", "Items", "DropDownItems", "Columns", "ColumnStyles", "RowStyles", "TabPages", "Nodes", "Images",
                "Panel1", "Panel2", "ContentPanel", "TopToolStripPanel", "BottomToolStripPanel", "LeftToolStripPanel",
                "RightToolStripPanel", "DataBindings", "Groups",
            ):
                coll = parts[-1]
                base = parts[:-1]
                owner = form if not base else obj(base[0]) if is_object(base[0]) else form
                if len(base) > 1:
                    coll = ".".join(base[1:] + [coll])
            elif is_object(parts[0]):
                owner = obj(parts[0])
                if len(parts) > 1:
                    coll = ".".join(parts[1:])
            else:
                owner = form
                coll = target
            refname = lambda x: norm_target(strip_cast(x))
            if method == "Add" and coll and coll.endswith("Controls"):
                child = refname(args[0])
                entry = {"name": child}
                if len(args) == 3:
                    entry["col"] = value(args[1], resx)
                    entry["row"] = value(args[2], resx)
                key = "children" if coll == "Controls" else coll
                owner.setdefault(key, []).append(entry)
            elif method == "AddRange" and coll and coll.endswith("Controls"):
                inner = re.search(r"\{(.*)\}", args[0], flags=re.S)
                key = "children" if coll == "Controls" else coll
                for x in split_args(inner.group(1)) if inner else []:
                    owner.setdefault(key, []).append({"name": refname(x)})
            elif method in ("Add", "AddRange") and coll and coll.split(".")[-1] in ("Items", "DropDownItems", "Columns", "TabPages", "Nodes", "Groups"):
                items = []
                for x in args:
                    inner = re.fullmatch(r"new\s+[\w.<>]*\s*\[\s*\]\s*\{(.*)\}", x.strip(), flags=re.S)
                    if inner or x.strip().startswith("new []") or x.strip().startswith("new[]"):
                        body2 = inner.group(1) if inner else re.search(r"\{(.*)\}", x, flags=re.S).group(1)
                        items.extend(split_args(body2))
                    else:
                        items.append(x)
                vals = []
                for x in items:
                    x2 = strip_cast(x)
                    if re.fullmatch(r"(?:this\.)?\w+", x2) and is_object(norm_target(x2)):
                        vals.append({"ref": norm_target(x2)})
                    else:
                        vals.append(value(x2, resx))
                owner.setdefault(coll, []).extend(vals)
            elif method == "Add" and coll and coll.endswith(("ColumnStyles", "RowStyles")):
                st = re.search(r"SizeType\.(\w+)(?:\s*,\s*([\d.]+)F?)?", args[0])
                size = float(st.group(2)) if st and st.group(2) else 0.0
                owner.setdefault(coll, []).append({"type": st.group(1) if st else "AutoSize", "size": size})
            elif method in ("SetColumnSpan", "SetRowSpan", "SetColumn", "SetRow", "SetCellPosition", "SetFlowBreak"):
                child = refname(args[0])
                obj(child)["layout" + method[3:]] = value(args[1], resx)
            elif method in ("SetToolTip", "SetHelpString", "SetShowHelp", "SetIconPadding", "SetIconAlignment", "SetError", "SetHelpKeyword", "SetHelpNavigator"):
                child = refname(args[0])
                prop = {"SetToolTip": "toolTipText", "SetHelpString": "helpString"}.get(method, method)
                obj(child)["props"][prop] = value(args[1], resx)
            elif method in ("SetKeyName", "SetBoundsCore", "SetAutoScrollMargin", "SetStyle", "Clear", "Sort", "AutoResizeColumn"):
                continue
            else:
                unknown.append(s[:200])
            continue
        unknown.append(s[:200])

    # Drop locals that are never part of the tree, such as the `resources` manager and ListViewItems handled above.
    objects.pop("resources", None)
    form["objects"] = objects
    if unknown:
        form["unparsed"] = unknown
    return form


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    designers = sorted(p for p in SRC.rglob("*") if p.name.lower().endswith(".designer.cs"))
    index = {}
    total_controls = 0
    total_unparsed = 0
    types = {}
    skipped = 0
    for p in designers:
        low = p.name.lower()
        if low in ("resources.designer.cs", "settings.designer.cs") or "NShape" in str(p) or "ZedGraph" in str(p):
            skipped += 1
            continue
        if re.search(r"Descriptors\.Designer\.cs$", p.name):
            skipped += 1
            continue
        form = parse_designer(p)
        if not form or not form["objects"] and not form["props"]:
            skipped += 1
            continue
        rel = str(p.relative_to(SRC))
        fname = re.sub(r"\.[dD]esigner\.cs$", "", rel).replace("/", "__") + ".json"
        (OUT / fname).write_text(json.dumps(form, indent=0, ensure_ascii=False))
        index.setdefault(form["class"], []).append(fname)
        total_controls += len(form["objects"])
        total_unparsed += len(form.get("unparsed", []))
        for o in form["objects"].values():
            types[o["type"]] = types.get(o["type"], 0) + 1
    (OUT / "_index.json").write_text(json.dumps(index, indent=0))
    print("forms: %d, controls: %d, skipped files: %d, unparsed statements: %d" % (sum(len(v) for v in index.values()), total_controls, skipped, total_unparsed))
    print("control types:")
    for k, v in sorted(types.items(), key=lambda x: -x[1])[:70]:
        print("  %5d %s" % (v, k))
    return 0


if __name__ == "__main__":
    sys.exit(main())
