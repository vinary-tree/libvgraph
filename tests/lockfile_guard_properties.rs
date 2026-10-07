//! Concrete properties refining the lockfile certification state-machine invariants.
//! Fake Git and grep commands inject producer and reader failures without editing
//! either tracked lockfile or relying on filesystem permission differences.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use proptest::prelude::*;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/lockfile-guard-property-fixtures");
        fs::create_dir_all(&root).expect("fixture root must be writable");
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!("{}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("fixture directory must be unique");
        Self(path)
    }

    fn install_command(&self, name: &str, source: &str) {
        let path = self.0.join(name);
        fs::write(&path, source).expect("fake command must be written");
        let mut permissions = fs::metadata(&path)
            .expect("fake command must exist")
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).expect("fake command must be executable");
    }

    fn command_path(&self) -> OsString {
        let mut paths = vec![self.0.clone()];
        paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
        env::join_paths(paths).expect("fixture path must join with PATH")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("could not remove lockfile property fixture: {error}");
        }
    }
}

fn check_with_faults(git_status: u8, emit_lockfile: bool, grep_status: u8) -> Output {
    let fixture = Fixture::new();
    fixture.install_command(
        "git",
        "#!/bin/sh\nif [ \"$LIBVGRAPH_TEST_EMIT\" = 1 ]; then printf 'Cargo.lock\\000'; fi\nexit \"$LIBVGRAPH_TEST_GIT_STATUS\"\n",
    );
    fixture.install_command(
        "grep",
        "#!/bin/sh\nfor arg do\n  if [ \"$arg\" = \"$LIBVGRAPH_TEST_LOCKFILE\" ]; then\n    exit \"$LIBVGRAPH_TEST_GREP_STATUS\"\n  fi\ndone\nexec /usr/bin/grep \"$@\"\n",
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    Command::new("bash")
        .arg(root.join("scripts/check-lockfile-portability.sh"))
        .current_dir(root)
        .env("PATH", fixture.command_path())
        .env("LIBVGRAPH_TEST_GIT_STATUS", git_status.to_string())
        .env("LIBVGRAPH_TEST_EMIT", if emit_lockfile { "1" } else { "0" })
        .env("LIBVGRAPH_TEST_GREP_STATUS", grep_status.to_string())
        .env("LIBVGRAPH_TEST_LOCKFILE", root.join("Cargo.lock"))
        .output()
        .expect("lockfile guard must execute")
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn certification_requires_complete_successful_enumeration_and_readable_patch_free_input(
        git_status in 0_u8..4,
        emit_lockfile in any::<bool>(),
        grep_status in 0_u8..4,
    ) {
        let output = check_with_faults(git_status, emit_lockfile, grep_status);
        let expected_certificate = git_status == 0 && emit_lockfile && grep_status == 1;
        prop_assert_eq!(
            output.status.success(),
            expected_certificate,
            "Git exit {}, emitted={}, grep exit {}: {}",
            git_status,
            emit_lockfile,
            grep_status,
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

#[test]
fn every_bounded_failure_combination_obeys_certification_invariant() {
    for git_status in 0_u8..4 {
        for emit_lockfile in [false, true] {
            for grep_status in 0_u8..4 {
                let output = check_with_faults(git_status, emit_lockfile, grep_status);
                let expected = git_status == 0 && emit_lockfile && grep_status == 1;
                assert_eq!(
                    output.status.success(),
                    expected,
                    "Git exit {git_status}, emitted={emit_lockfile}, grep exit {grep_status}: {}",
                    String::from_utf8_lossy(&output.stderr),
                );
            }
        }
    }
}

#[test]
fn nonzero_git_status_after_a_nonempty_prefix_cannot_certify() {
    let output = check_with_faults(7, true, 1);
    assert!(
        !output.status.success(),
        "partial Git output must not certify"
    );
}

#[test]
fn grep_read_error_cannot_certify() {
    let output = check_with_faults(0, true, 2);
    assert!(!output.status.success(), "grep read error must not certify");
}
