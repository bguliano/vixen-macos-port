//! Compares the port with the C++ output in tests/golden/analysis for every synthetic case. Regenerate those files with
//! tools/qm-golden/generate.sh.

mod common;

use common::{generate, load_goldens, run_golden_case, run_in_parallel, Differences};

#[test]
fn every_synthetic_case_matches_the_cpp() {
    let cases = load_goldens(|g| g["input"]["kind"] != "raw");
    assert!(cases.len() >= 80, "only {} golden cases found", cases.len());

    let results = run_in_parallel(&cases, |_, golden| run_golden_case(golden, &generate(&golden["input"])));

    let mut total = Differences::default();
    let mut failures = Vec::new();
    for (name, (diff, errors)) in &results {
        total.merge(*diff);
        if !errors.is_empty() {
            let shown: Vec<&str> = errors.iter().take(4).map(String::as_str).collect();
            failures.push(format!("{name}: {} mismatches, first: {}", errors.len(), shown.join("; ")));
        }
    }

    println!(
        "{} cases, {} features: largest timestamp difference {} ns, largest value difference {:e}",
        results.len(),
        total.features,
        total.max_timestamp_ns,
        total.max_value_abs
    );
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), results.len(), failures.join("\n"));
}
