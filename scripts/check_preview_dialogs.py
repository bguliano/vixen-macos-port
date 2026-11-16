#!/usr/bin/env python3
"""Check persisted results from the private native Preview dialog workflows."""
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def local(node):
    return node.tag.split("}")[-1]


def children(node, name):
    return [child for child in node if local(child) == name]


def text(node, name):
    return children(node, name)[0].text


def shapes(profile):
    root = ET.parse(profile / "SystemData/ModuleStore.xml").getroot()
    return [node for node in root.iter() if local(node) == "Shape"]


def check(root):
    names = ("invalid", "doubleclick", "enum-final", "tree2", "tree-staged", "tree-context", "template4", "sync2",
             "template-cancel", "wizard-final", "fixture-link2", "fixture-clear", "linker3", "canvas-menu2", "live-menu", "single-command")
    reports = {}
    for name in names:
        rows = json.loads((root / f"{name}-out/result.json").read_text())
        assert rows and all(row["ok"] for row in rows), (name, rows)
        reports[name] = rows

    def details(name, action):
        return [row["detail"] for row in reports[name] if row["action"] == action]

    assert "Preview Configuration" in details("invalid", "snapshot")[-1]
    assert "Preview Configuration" not in details("invalid", "windows")[-1]
    assert "lock=true" in details("invalid", "designerInfo")[-1]
    assert any("Locked=bool(value: true)" in value for value in details("doubleclick", "designerProperties"))
    assert 'StringType=enumeration(value: "Pixel")' in details("enum-final", "designerProperties")[-1]
    assert "string=1" in details("sync2", "designerProperties")[0]
    assert "PixelSize=int(value: 9)" in details("sync2", "designerProperties")[-1]
    assert all("tool=single" in value for value in details("single-command", "designerInfo"))
    assert "Preview Configuration" not in details("single-command", "snapshot")[0]
    assert "Preview Configuration" in details("single-command", "snapshot")[1]
    assert [re.search(r"shapes=(\d+)", value).group(1) for value in details("canvas-menu2", "designerInfo")] == ["1", "2", "1"]
    assert all("content=896x450" in value for value in details("live-menu", "liveWindowInfo"))
    assert [re.search(r"Verification (\d+x\d+)", value).group(1) for value in details("live-menu", "windows")] == ["896x482", "896x450", "896x482"]
    tree = ET.parse(root / "profile-tree2/SystemData/SystemConfig.xml").getroot()
    group = next(node for node in tree.iter("Node") if node.get("name") == "Lamp")
    assert [node.get("name") for node in group] == ["Node-3", "Node-2", "Node-1"]
    prop = next(tag for tag in tree.iter("Tag") if tag.get("key") == "prop")
    assert prop.get("displayColor") == "#123456"
    assert all(any(tag.get("id") == prop.get("id") for tag in node.iter("Tag")) for node in group)
    staging = json.loads((root / "tree-staged-check.json").read_text())
    assert staging["unchangedBeforeSave"] and staging["before"] == staging["staged"]

    template = ET.parse(root / "profile-template4/Module Data Files/VixenDisplayPreview/Templates/Native Preview Template.xml").getroot()
    group = children(template, "Shape")[0]
    assert text(group, "Name") == "Native Preview Template"
    assert [text(node, "PixelSize") for node in children(group, "_strings")[0]] == ["4", "8"]
    group = shapes(root / "profile-template4")[0]
    assert [text(node, "PixelSize") for node in children(group, "_strings")[0]] == ["9", "9"]
    assert len(shapes(root / "profile-template-cancel")) == 1
    assert not list((root / "profile-template-cancel/Module Data Files/VixenDisplayPreview/Templates").glob("*.xml"))

    profile = root / "profile-wizard-final"
    system = ET.parse(profile / "SystemData/SystemConfig.xml").getroot()
    assert not list(system.iter("Controller"))
    heads = [node for node in shapes(profile) if any("PreviewMovingHead" in value for value in node.attrib.values())]
    assert len(shapes(profile)) == 5 and len(heads) == 3
    created = {node.get("id") for node in system.iter("Node") if re.fullmatch(r"Test Head[123]", node.get("name", ""))}
    assert len(created) == 3 and {text(head, "NodeId") for head in heads} == created
    assert [(int(text(children(head, "_topLeft")[0], "X")), int(text(children(head, "_topLeft")[0], "Y")))
            for head in heads] == [(360, 80), (0, 0), (80, 0)]
    assert all(int(text(children(head, "_bottomRight")[0], "X")) - int(text(children(head, "_topLeft")[0], "X")) == 80 for head in heads)
    fixture_two = next(node.get("id") for node in system.iter("Node") if node.get("name") == "Fixture Two")
    values = details("fixture-link2", "designerProperties")
    assert fixture_two not in values[0] and fixture_two in values[1] and values[1] == values[2]
    assert "Node=linkedElement(node: nil)" in details("fixture-clear", "designerProperties")[-1]
    linked_head = [node for node in shapes(root / "profile-fixture-link") if any("PreviewMovingHead" in value for value in node.attrib.values())][0]
    clear_head = [node for node in shapes(root / "profile-fixture-clear") if any("PreviewMovingHead" in value for value in node.attrib.values())][0]
    assert text(linked_head, "NodeId") == fixture_two
    assert not children(clear_head, "NodeId")

    states = [json.loads(value) for value in details("linker3", "designerLinkerInfo")]
    assert any(state[0]["name"] == "Fixture Two" and state[1]["name"] == "Lamp-1" for state in states)
    assert any([row["name"] for row in state[:3]] == ["Lamp-1", "Lamp-2", "Lamp-3"] for state in states)
    assert any([row["name"] for row in state[-3:]] == ["Lamp-3", "Lamp-2", "Lamp-1"] for state in states)
    assert all(row["node"] == fixture_two for row in states[-1])
    line = shapes(root / "profile-linker3")[0]
    assert all(node.text == fixture_two for node in line.iter() if local(node) == "NodeId")
    print(f"PASS: {sum(map(len, reports.values()))} native steps; Preview close, tree/tag persistence, templates, "
          "double-click, Sync, fixture creation/linking and linker menu/OK/Cancel")


if __name__ == "__main__":
    check(Path(sys.argv[1]) if len(sys.argv) > 1 else Path("target/preview-partials"))
