use std::path::Path;
use std::process::Command;

#[test]
fn summary_preserves_json_and_exit_status_without_color() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for (version, status, verdict) in [("after_honest", 0, "ACCEPT"), ("before", 1, "REJECT")] {
        let output = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
            .args(["--summary", "target", version])
            .current_dir(&root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(status));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["verdict"].as_str().unwrap().to_uppercase(), verdict);
        let summary = String::from_utf8(output.stderr).unwrap();
        assert!(summary.starts_with(verdict));
        assert!(summary.contains("  attacks "));
        assert!(summary.contains("  regressions "));
        assert!(!summary.contains('\u{1b}'));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_fixseal-judge"))
        .args(["--summary", "bundle", "target", "after_honest"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let bundle: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(bundle["report"]["verdict"], "accept");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("ACCEPT")
    );
}
