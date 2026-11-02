use std::process::Command;

#[test]
fn help_is_available() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .arg("--help")
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("Usage: ofx"));
    assert!(help.contains("inspect"));
    assert!(help.contains("validate"));
    assert!(help.contains("decode"));
    assert!(help.contains("convert"));
}

#[test]
fn inspect_help_describes_filters() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args(["inspect", "--help"])
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--account"));
    assert!(help.contains("--from"));
    assert!(help.contains("--limit"));
}

#[test]
fn version_is_available() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .arg("--version")
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn inspect_json_is_available() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args([
            "inspect",
            "../ofx-kit/tests/fixtures/simple_bank_statement.ofx",
            "--json",
        ])
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["statements"][0]["account"]["account_id"], "987654321");
}

#[test]
fn validate_json_reports_invalid_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args(["validate", "missing.ofx", "--json"])
        .output()
        .expect("failed to run ofx");

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(json["valid"], false);
    assert_eq!(json["error"]["kind"], "io");
}
