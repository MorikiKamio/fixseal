#![cfg(unix)]

use fixseal_judge::input::load_lean_bundle;
use std::path::Path;
use std::process::Command;

#[cfg(unix)]
#[test]
fn lean_subprocess_timeouts_have_a_distinct_cli_reason() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("fixseal-timeout-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let bundle = load_lean_bundle(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean"),
        "after_honest",
    )
    .unwrap();
    let corpus: serde_json::Value = serde_json::from_slice(&bundle.corpus).unwrap();
    let regressions: Vec<serde_json::Value> =
        serde_json::from_slice(&bundle.baseline["tests/regression.json"]).unwrap();
    let attacks =
        corpus["public"].as_array().unwrap().len() + corpus["variants"].as_array().unwrap().len();
    // Canned behavior isolates process deadlines from real compiler speed under load.
    let candidate: Vec<_> = std::iter::repeat_n(serde_json::Value::Null, attacks)
        .chain(regressions.iter().map(|test| test["expected"].clone()))
        .collect();
    let baseline = vec![serde_json::json!("/outside"); candidate.len()];
    for (name, values) in [("baseline", baseline), ("candidate", candidate)] {
        let mut lines = String::new();
        for value in values {
            lines.push_str(&serde_json::to_string(&value).unwrap());
            lines.push('\n');
        }
        std::fs::write(root.join(format!("{name}.jsonl")), lines).unwrap();
        std::fs::write(
            root.join(format!("{name}.sh")),
            format!("#!/bin/sh\ncat \"$FIXSEAL_TEST_FIXTURES/{name}.jsonl\"\n"),
        )
        .unwrap();
    }
    std::fs::write(root.join("baseline.lean"), &bundle.baseline["Target.lean"]).unwrap();
    let lake = root.join("lake");
    std::fs::write(
        &lake,
        r#"#!/bin/sh
case "$FIXSEAL_TEST_STAGE:$*" in
  build:build\ pathcheck|proof:build|audit:env\ lean\ Audit.lean)
    touch "$FIXSEAL_TEST_MARKER"
    sleep 10
    exit 1 ;;
  runtime:build\ pathcheck)
    mkdir -p .lake/build/bin
    printf '#!/bin/sh\ntouch "$FIXSEAL_TEST_MARKER"\nsleep 10\n' > .lake/build/bin/pathcheck
    chmod +x .lake/build/bin/pathcheck
    exit 0 ;;
esac
case "$*" in
  'build pathcheck')
    mkdir -p .lake/build/bin
    if cmp -s after_honest/Target.lean "$FIXSEAL_TEST_FIXTURES/baseline.lean"; then
      cp "$FIXSEAL_TEST_FIXTURES/baseline.sh" .lake/build/bin/pathcheck
    else
      cp "$FIXSEAL_TEST_FIXTURES/candidate.sh" .lake/build/bin/pathcheck
    fi
    chmod +x .lake/build/bin/pathcheck ;;
  build) exit 0 ;;
  *) exit 99 ;;
esac
"#,
    )
    .unwrap();
    std::fs::set_permissions(&lake, std::fs::Permissions::from_mode(0o755)).unwrap();
    for stage in ["build", "runtime", "proof", "audit"] {
        let marker = root.join(stage);
        let output = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
            .env("FIXSEAL_LEAN_BIN", &root)
            .env("FIXSEAL_LEAN_TIMEOUT_SECS", "1")
            .env("FIXSEAL_TEST_STAGE", stage)
            .env("FIXSEAL_TEST_MARKER", &marker)
            .env("FIXSEAL_TEST_FIXTURES", &root)
            .args(["--lean", "../lean", "after_honest"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .unwrap();
        assert!(
            marker.exists(),
            "stage not reached: {stage}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(output.status.code(), Some(1), "{stage}");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["verdict"], "reject");
        assert_eq!(report["proof"]["verified"], false);
        let reasons = report["reason_codes"].as_array().unwrap();
        assert!(
            reasons.contains(&serde_json::json!("PROOF_TIMEOUT")),
            "{stage}: {report}"
        );
        assert!(
            !reasons.contains(&serde_json::json!("PROOF_REJECTED")),
            "{stage}: {report}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
