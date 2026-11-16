#!/usr/bin/env python3
"""Assert native F5/F8/F9 and Space transport routes and displayed playheads."""
import json
from pathlib import Path
import sys


def check(root: Path) -> None:
    rows = json.loads((root / "result.json").read_text())
    assert len(rows) == 60 and all(row["ok"] for row in rows), rows
    states = [row["detail"] for row in rows if row.get("detail", "").startswith("running=")]
    expected = [(False, False, False), (False, False, True), (False, False, False),
                (True, False, False), (True, True, False), (True, False, False),
                (False, False, False), (True, False, False), (False, False, False),
                (True, False, False), (True, True, False), (True, False, False),
                (False, False, False)]
    assert states == [f"running={str(r).lower()} paused={str(p).lower()} looping={str(l).lower()}"
                      for r, p, l in expected], states
    playback = [json.loads(row["detail"]) for row in rows if row.get("detail", "").startswith("{")]
    assert len(playback) == len(expected), playback
    for state, (running, paused, _) in zip(playback, expected):
        assert state["transport"] == ("paused" if paused else "playing" if running else "stopped"), state
        if running:
            assert state["playbackLineX"] >= 0 and state["positionMs"] > 0, state
        else:
            # Stop hides the playhead but retains the owning editor's last clock.
            assert state["playbackLineX"] == -1 and state["positionMs"] >= 0, state
    assert all(state["positionMs"] == 0 for state in playback[:3])
    for before, after in [(4, 5), (10, 11)]:
        assert playback[after]["positionMs"] > playback[before]["positionMs"]
    routes = [row["detail"] for row in rows if row.get("detail", "").startswith("posted ")]
    assert len(routes) == 12
    assert all("route=table" in route and "handled=true" in route for route in routes), routes
    print("PASS: 60 native steps; F5/F8/F9, Space, Option-Space, pause/resume and cleared playheads")


if __name__ == "__main__":
    check(Path(sys.argv[1]))
