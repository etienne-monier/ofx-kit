use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_output_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!("ofx-kit-convert-{nonce}.xml"))
}

#[test]
fn convert_writes_valid_xml_to_stdout() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args([
            "convert",
            "../ofx-kit/tests/fixtures/simple_bank_statement.ofx",
        ])
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    let xml = String::from_utf8_lossy(&output.stdout);
    assert!(xml.starts_with("<?OFX OFXHEADER=\"200\""));
    assert!(xml.contains("<OFX>"));
    assert!(xml.contains("</OFX>"));
}

#[test]
fn convert_check_does_not_write_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args([
            "convert",
            "../ofx-kit/tests/fixtures/simple_bank_statement.ofx",
            "--check",
        ])
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn convert_updates_only_the_header_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args([
            "convert",
            "../ofx-kit/tests/fixtures/simple_bank_statement.ofx",
            "--to",
            "220",
        ])
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    let xml = String::from_utf8_lossy(&output.stdout);
    assert!(xml.starts_with("<?OFX OFXHEADER=\"200\" VERSION=\"220\""));
    assert!(xml.contains("<ACCTID>987654321</ACCTID>"));
}

#[test]
fn convert_writes_to_output_file() {
    let path = temporary_output_path();
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args([
            "convert",
            "../ofx-kit/tests/fixtures/simple_bank_statement.ofx",
            "--output",
            path.to_str().expect("temporary path should be UTF-8"),
        ])
        .output()
        .expect("failed to run ofx");

    assert!(output.status.success());
    let xml = std::fs::read_to_string(&path).expect("output file should exist");
    assert!(xml.contains("<OFX>"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn convert_reports_missing_input_as_data_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_ofx"))
        .args(["convert", "missing.ofx"])
        .output()
        .expect("failed to run ofx");

    assert_eq!(output.status.code(), Some(1));
}
