use proptest::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    directory: PathBuf,
    lockfile: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target =
            env::var_os("CARGO_TARGET_DIR").map_or_else(|| manifest.join("target"), PathBuf::from);
        let target = if target.is_absolute() {
            target
        } else {
            manifest.join(target)
        };
        let parent = target.join("lockfile-portability-tests");
        fs::create_dir_all(&parent).expect("create disk-backed test directory");
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let directory = parent.join(format!("{}-{sequence}", std::process::id()));
        fs::create_dir(&directory).expect("create unique test directory");
        let lockfile = directory.join("Cargo.lock");
        Self {
            directory,
            lockfile,
        }
    }

    fn write(&self, contents: &str) {
        fs::write(&self.lockfile, contents).expect("write generated lockfile");
    }

    fn portable(&self) -> bool {
        portable_path(&self.lockfile)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lockfile);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn portable_path(lockfile: &Path) -> bool {
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/lockfile-portability-lib.sh");
    Command::new("bash")
        .args([
            "-c",
            "source \"$1\"; lockfile_has_only_repository_resolution \"$2\"",
            "_",
        ])
        .arg(library)
        .arg(lockfile)
        .output()
        .expect("run production lockfile predicate")
        .status
        .success()
}

fn clean_lockfile(package: &str) -> String {
    format!("version = 3\n\n[[package]]\nname = \"{package}\"\nversion = \"0.1.0\"\n")
}

fn ambient_header(style: u8, indent: u8) -> String {
    let (first, second) = match style % 3 {
        0 => ("patch", "unused"),
        1 => ("\"patch\"", "\"unused\""),
        _ => ("'patch'", "'unused'"),
    };
    format!("{}[[ {first} . {second} ]]\n", " ".repeat(indent as usize))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn generated_clean_lockfiles_are_accepted(package in "[a-z]{1,12}") {
        let fixture = Fixture::new();
        fixture.write(&format!("# [[patch.unused]] is only a comment\n{}", clean_lockfile(&package)));
        prop_assert!(fixture.portable());
    }

    #[test]
    fn generated_ambient_headers_are_rejected(
        package in "[a-z]{1,12}", style in 0u8..3, indent in 0u8..4
    ) {
        let fixture = Fixture::new();
        fixture.write(&format!("{}{}name = \"ambient\"\n",
            clean_lockfile(&package), ambient_header(style, indent)));
        prop_assert!(!fixture.portable());
    }

    #[test]
    fn changing_a_clean_lockfile_invalidates_its_certificate(
        package in "[a-z]{1,12}", style in 0u8..3
    ) {
        let fixture = Fixture::new();
        let clean = clean_lockfile(&package);
        fixture.write(&clean);
        prop_assert!(fixture.portable());
        fixture.write(&format!("{clean}{}name = \"ambient\"\n", ambient_header(style, 0)));
        prop_assert!(!fixture.portable());
        fixture.write(&clean);
        prop_assert!(fixture.portable());
    }

    #[test]
    fn generated_transition_traces_preserve_all_model_invariants(
        initial in 0u8..4, actions in proptest::collection::vec(0u8..6, 0..24)
    ) {
        let mut ambient = initial;
        let mut certified = false;
        let mut published = false;
        let root = Fixture::new();
        let fuzz = Fixture::new();
        for action in std::iter::once(None).chain(actions.into_iter().map(Some)) {
            if let Some(action) = action {
                if !published {
                    match action {
                        0 => { ambient |= 1; certified = false; }
                        1 => { ambient |= 2; certified = false; }
                        2 => { ambient &= !1; certified = false; }
                        3 => { ambient &= !2; certified = false; }
                        4 if ambient == 0 => certified = true,
                        5 if certified && ambient == 0 => published = true,
                        _ => {}
                    }
                }
            }
            prop_assert!(ambient <= 3);
            prop_assert!(!certified || ambient == 0);
            prop_assert!(!published || ambient == 0);
            prop_assert!(!published || certified);
            root.write(&format!("{}{}", clean_lockfile("root"),
                if ambient & 1 == 0 { String::new() } else { ambient_header(0, 0) }));
            fuzz.write(&format!("{}{}", clean_lockfile("fuzz"),
                if ambient & 2 == 0 { String::new() } else { ambient_header(0, 0) }));
            prop_assert_eq!(root.portable() && fuzz.portable(), ambient == 0);
        }
    }
}

#[test]
fn tracked_repository_lockfiles_are_portable() {
    let status = Command::new("bash")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check-lockfile-portability.sh"))
        .status()
        .expect("run repository lockfile gate");
    assert!(
        status.success(),
        "tracked Cargo.lock must not contain ambient patches"
    );
}

#[cfg(unix)]
#[test]
fn missing_and_symlink_lockfiles_are_rejected() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    assert!(!fixture.portable());
    fixture.write(&clean_lockfile("root"));
    let link = fixture.directory.join("linked.lock");
    symlink(&fixture.lockfile, &link).expect("create test symlink");
    assert!(!portable_path(&link));
    fs::remove_file(link).expect("remove test symlink");
}
