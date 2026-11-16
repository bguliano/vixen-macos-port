# Headless playback/output endurance

This standalone verification crate links the production Rust libraries without changing the app or its811 pinned inputs. It uses a newly generated silent profile with three Set Level effects, no previews or audio, and exactly one E1.31 controller targeting127.0.0.1:5568. The receiver binds that address exclusively before starting playback; a busy port fails rather than sharing it. No AppCore services, web server, audio device or native UI is started.

`run.py --prepare --out ABSOLUTE_NEW_FOLDER` copies the built executable, renders and serializes the private fixture, records source/binary/fixture hashes, and runs six pure packet controls. It does not start playback or open a socket. Build offline with the local toolchain, two Cargo jobs and a separate target directory:

```
source ./env.sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/target/output-endurance-cargo" cargo build --offline --locked --manifest-path tools/output-endurance/Cargo.toml
```

After independent readiness review, `run.py --out PREPARED_FOLDER` executes twenty minutes of real playback and controller output. Each cycle checks looping, both constant output levels, paused seeks, resume, natural and explicit named-context completion, primary completion exactly once, controller shutdown and restart. Every received packet is checked against an independently written complete E1.31 packet layout; sequence continuity is scoped to each controller-start epoch. Sent/received totals must match. Once a plateau is observed, any opposite payload during its150ms observation window fails. The ShowPlayback owner must be released on teardown.

The Rust process has an independent1240-second deadline. The Python parent has an independent1280-second deadline covering preflight and final I/O, cooperative TERM followed by bounded KILL of its owned process group, and a cancellation latch for TERM/INT. Partial logs survive failure. An exhausted deadline or forced cleanup cannot count as a pass. Root coordinates any required sandbox exception for this loopback-only command.

`cycles.jsonl`, `progress.json`, `native-result.json`, `resources.jsonl` and `result.json` retain evidence. RSS, thread counts, descriptor counts and timing/replaced-frame statistics are diagnostic trends, without invented leak or timing thresholds. This verifies the linked production-library scope, not the signed app's GUI, actual controllers, audible synchronization, Windows parity or a multi-hour live show. The retained20-minute baseline separately exposed an internal replaced-frame accounting defect; no current endurance pass should imply that statistic is correct.
