#!/usr/bin/env python3
"""Check the native export-wizard fixtures recorded in VERIFICATION.md."""
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def check(root: Path) -> None:
    reports = {}
    for name in ("profiles2", "persistence", "sequences", "controllers-multiple"):
        rows = json.loads((root / f"{name}-out/result.json").read_text())
        assert rows and all(row["ok"] for row in rows), (name, rows)
        reports[name] = rows

    document = ET.parse(root / "profile2/SystemData/ModuleStore.xml")
    names = [node.text for node in document.iter() if node.tag.split("}")[-1] == "Name"
             and (node.text or "").startswith("Native Persistence")]
    assert names == ["Native Persistence Renamed"], names

    counts = [int(match.group(1)) for row in reports["sequences"]
              if (match := re.search(r"stage 1 next true files (\d+)", row.get("detail", "")))]
    assert counts == [1, 1, 6, 5, 4], counts

    states = [json.loads(row["detail"]) for row in reports["controllers-multiple"]
              if row.get("detail", "").startswith("[{")]
    assert states and all(len(state) == 2 for state in states)
    assert any(all(row["selected"] and not row["active"] and row["start"] == row["end"] == 0
                   for row in state) for state in states), states
    assert any(all(row["selected"] and row["active"] for row in state) for state in states), states
    assert all(not row["selected"] and row["active"] for row in states[-1]), states[-1]
    assert [(row["start"], row["end"]) for row in states[-1]] == [(1, 16), (17, 3367)]
    print(f"PASS: {sum(map(len, reports.values()))} native steps; profile persistence, "
          "sequence selection/removal, multi-controller toggles and channel ranges")


if __name__ == "__main__":
    check(Path(sys.argv[1]) if len(sys.argv) > 1 else Path("target/scheduler-profile-ui"))
