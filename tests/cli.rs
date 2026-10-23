use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "dyncursor-cli-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    fn executable(&self) -> PathBuf {
        let executable = self.0.join("DynCursor.exe");
        fs::copy(env!("CARGO_BIN_EXE_DynCursor"), &executable).unwrap();
        let cursors = self.0.join("resources/cur");
        fs::create_dir_all(&cursors).unwrap();
        for name in ["custom-arrow.cur", "custom-beam.cur", "custom-link.cur"] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("resources/cur")
                    .join(name),
                cursors.join(name),
            )
            .unwrap();
        }
        executable
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Restrict recursive cleanup to the exact temporary fixture we created.
        let directory = fs::canonicalize(&self.0).unwrap();
        let temporary = fs::canonicalize(std::env::temp_dir()).unwrap();
        assert_eq!(directory.parent(), Some(temporary.as_path()));
        assert!(directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("dyncursor-cli-test-"));
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn version_matches_package_metadata() {
    let output = Command::new(env!("CARGO_BIN_EXE_DynCursor"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("DynCursor {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn default_config_works_from_an_unrelated_working_directory() {
    let fixture = Fixture::new();
    let executable = fixture.executable();
    fs::create_dir(fixture.0.join("config")).unwrap();
    fs::write(
        fixture.0.join("config/config.yaml"),
        include_str!("../config/config.yaml"),
    )
    .unwrap();
    let output = Command::new(executable)
        .arg("--check-config")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn missing_default_config_fails_cleanly_without_a_dialog() {
    let fixture = Fixture::new();
    let output = Command::new(fixture.executable())
        .arg("--check-config")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("config.yaml"));
    assert!(!error.contains("panicked"));
}

#[test]
fn malformed_config_reports_an_error_and_valid_relative_path_is_supported() {
    let fixture = Fixture::new();
    let executable = fixture.executable();
    let config_path = fixture.0.join("custom.yaml");
    fs::write(&config_path, "custom-layout: 0804\ncursor-mapping: 32512").unwrap();
    let invalid = Command::new(&executable)
        .args(["--check-config", "custom.yaml"])
        .current_dir(&fixture.0)
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("key:value"));
    fs::write(&config_path, include_str!("../config/config.yaml")).unwrap();
    let valid = Command::new(executable)
        .args(["--check-config", "custom.yaml"])
        .current_dir(&fixture.0)
        .output()
        .unwrap();
    assert!(valid.status.success());
}
