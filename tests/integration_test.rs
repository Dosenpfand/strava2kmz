use assert_cmd::Command;
use assert_fs::prelude::{PathAssert, PathChild};
use assert_fs::TempDir;

#[test]
fn test_bin() {
    let output_file_names = [
        "1996690532.kmz",
        "2033837592.kmz",
        "3022137832.kmz",
        "3717243816.kmz",
    ];
    let tmp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.current_dir("tests")
        .arg("export.zip")
        .arg(format!("{}", tmp_dir.to_string_lossy()))
        .assert()
        .success();
    for output_file_name in output_file_names {
        let output_file = tmp_dir.child(output_file_name);
        output_file.assert(predicates::path::exists());
    }
}
