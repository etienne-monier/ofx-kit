use std::process::Command;

#[test]
fn help_is_available() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .arg("--help")
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: ofx"));
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
