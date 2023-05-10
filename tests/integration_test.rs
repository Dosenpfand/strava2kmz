use assert_cmd::Command;

#[test]
fn test_bin() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.current_dir("tests")
        .arg("export.zip")
        .assert()
        .success();
}
