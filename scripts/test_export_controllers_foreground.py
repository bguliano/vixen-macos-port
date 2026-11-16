#!/usr/bin/env python3
"""Bounded foreground Export Controllers test on an isolated, muted profile copy.

Requires user-approved exclusive desktop time. The external Swift driver checks
actual AX key/main/table focus before sending OS CG events and restores the prior
frontmost app and pointer. --prepare-only never launches either application.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess


def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--profile", type=Path, default=Path("target/export-command-a-cg/profile"))
    parser.add_argument("--helper", type=Path, default=Path("target/foreground-export-controls"))
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--run-prepared", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    source, out = args.profile.resolve(), args.out.resolve()
    app, helper = args.app.resolve()/"Contents/MacOS/Vixen", args.helper.resolve()
    assert "target" in source.parts and not out.is_relative_to(source)
    assert app.is_file() and helper.is_file()
    if args.run_prepared or args.verify_only:
        manifest = json.loads((out/"prepared.json").read_text())
        assert digest(app)==manifest["appSha256"] and digest(helper)==manifest["helperSha256"]
    else:
        assert not out.exists()
        out.mkdir(parents=True)
        before = {str(p.relative_to(source)):digest(p) for p in source.rglob("*") if p.is_file()}
        profile = out/"profile"
        shutil.copytree(source,profile)
        (profile/".lock").unlink(missing_ok=True)
        hook = lambda *path:{"action":"hook","name":"wizard","path":list(path)}
        wait = lambda n=.3:{"action":"wait","seconds":n}
        steps = [{"action":"window","id":"administration"},wait(1),
                 {"action":"hook","name":"admin.openSequence","path":[str(profile/"Sequence/Device 01.tim")]},wait(2),
                 {"action":"window","id":"export-wizard"},wait(.7),{"action":"focus","window":"Export Wizard"},
                 {"action":"click","label":"Next >>"},wait(),hook("addAll"),{"action":"click","label":"Next >>"},wait(),
                 {"action":"selectRow","label":"Arduino"},hook("focusControllers"),hook("controllers"),
                 {"action":"snapshot","name":"ready"},wait(8)]
        for name,seconds in [("command-a",5),("space-off",5),("space-on",6),("reordered",.3)]:
            steps += [hook("controllers"),{"action":"snapshot","name":name},wait(seconds)]
        steps += [{"action":"click","label":"Cancel"},wait(.5),{"action":"windows"},{"action":"quit"}]
        (out/"steps.json").write_text(json.dumps({"steps":steps},indent=2)+"\n")
        manifest = {"appSha256":digest(app),"helperSha256":digest(helper),"source":str(source),"sourceHashes":before,
                    "steps":len(steps)-1,"scope":"private PID only, muted outputs; actual foreground key/main/table assertions before CG keys/drag"}
        (out/"prepared.json").write_text(json.dumps(manifest,indent=2)+"\n")
    if args.prepare_only:
        print(out/"prepared.json")
        return
    profile,ui,runtime = out/"profile",out/"ui",out/"runtime"
    if args.verify_only:
        previous_report = json.loads((out/"result.json").read_text())
        preserved = out/"result.initial-check-failure.json"
        if not preserved.exists(): shutil.copyfile(out/"result.json",preserved)
        return_code = previous_report["exitCode"]
    else:
        assert not ui.exists() and not runtime.exists(), "Use a fresh prepared run"
        ui.mkdir();runtime.mkdir()
    env = {k:v for k,v in os.environ.items() if not k.startswith(("VIXEN_","DYLD_"))}
    env.update(VIXEN_DATA=str(profile),VIXEN_MUTE="1",VIXEN_UI_TEST=str(out/"steps.json"),
               VIXEN_UI_TEST_OUT=str(ui),VIXEN_UI_TEST_TRACE="1",TMPDIR=str(runtime)+"/")
    driver = None
    fallback = None
    if not args.verify_only:
        with (out/"process.log").open("w") as log, (out/"foreground.log").open("w") as foreground_log:
            process = subprocess.Popen([str(app)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            try:
                driver = subprocess.Popen([str(helper),str(process.pid),str(out),str(app)],stdout=foreground_log,stderr=subprocess.STDOUT)
                process.wait(timeout=80)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid,signal.SIGKILL)
                    process.wait()
                if driver is not None:
                    try: driver.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        driver.kill();driver.wait()
                        start = out/"foreground-start.json"
                        if start.exists():
                            prior = json.loads(start.read_text())["previousPID"]
                            if prior>0:
                                fallback = subprocess.run([str(helper),"--restore",str(prior)],capture_output=True,text=True,timeout=5)
        return_code = process.returncode
    rows = json.loads((ui/"result.json").read_text())
    foreground = json.loads((out/"foreground.json").read_text()) if (out/"foreground.json").exists() else {}
    report = {"passed":False,"exitCode":return_code,"steps":len(rows),"appSha256":digest(app),
              "foreground":foreground,"fallbackRestore":None if fallback is None else {"returnCode":fallback.returncode,"output":fallback.stdout}}
    (out/"result.json").write_text(json.dumps(report,indent=2)+"\n")
    assert return_code==0 and len(rows)==manifest["steps"] and all(r["ok"] for r in rows),rows
    assert foreground.get("passed") and foreground.get("restoredPrevious") and foreground.get("targetExited"),foreground
    states = [json.loads(r["detail"]) for r in rows if r.get("detail","").startswith("[{")]
    assert len(states)==5 and all(len(s)==2 for s in states),states
    initial,selected,off,on,reordered=states
    assert sum(r["selected"] for r in initial)==1 and all(r["active"] for r in initial)
    assert all(r["selected"] and r["active"] for r in selected)
    assert all(r["selected"] and not r["active"] and r["start"]==r["end"]==0 for r in off)
    assert all(r["active"] for r in on)
    assert [r["id"] for r in reordered]==list(reversed([r["id"] for r in initial]))
    cursor=1
    for row in reordered:
        assert row["active"] and row["start"]==cursor and row["end"]==cursor+row["channels"]-1,row
        cursor += row["channels"]
    windows=[r["detail"] for r in rows if r["action"]=="windows"]
    assert len(windows)==1 and all("Export Wizard" not in w or w.endswith("(hidden)") for w in windows[0].split(" | "))
    assert {str(p.relative_to(source)):digest(p) for p in source.rglob("*") if p.is_file()}==manifest["sourceHashes"]
    assert digest(app)==manifest["appSha256"] and digest(helper)==manifest["helperSha256"]
    report.update(passed=True,states=states,sourceUnchanged=True,revalidatedExisting=args.verify_only)
    (out/"result.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({"passed":True,"steps":len(rows),"report":str(out/"result.json")}))


if __name__=="__main__":main()
