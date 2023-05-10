use assert_cmd::Command;

#[test]
fn test_bin() {
    let mut cmd = Command::cargo_bin("strava2kmz").unwrap();
    cmd.arg("tests/export.zip").assert().success();
}
