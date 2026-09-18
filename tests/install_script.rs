#![cfg(unix)]

mod common;

use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::Path,
    process::{Command, Output},
};

use common::TempDir;

const BIN: &str = env!("CARGO_PKG_NAME");
const INSTALLER: &str = include_str!("../scripts/install.sh");
const UNINSTALLER: &str = include_str!("../scripts/uninstall.sh");

fn block_begin() -> String {
    format!("# >>> {BIN} completions >>>")
}

fn block_end() -> String {
    format!("# <<< {BIN} completions <<<")
}

fn source_line() -> String {
    format!(
        "[ -s \"$HOME/.local/share/zsh/site-functions/_{BIN}\" ] && . \"$HOME/.local/share/zsh/site-functions/_{BIN}\""
    )
}

/// The part of a script before its main body, so a test can call one function.
fn definitions<'a>(script: &'a str, main_body_starts_with: &str) -> &'a str {
    script
        .split_once(main_body_starts_with)
        .map(|(definitions, _)| definitions)
        .expect("the script defines its functions before its main body")
}

fn run(definitions: &str, call: &str, profile: &Path, home: &Path) -> Output {
    let script = format!("{definitions}\n{call}\n");

    Command::new("bash")
        .args(["-c", &script, "profile-test"])
        .arg(profile)
        .env("HOME", home)
        .output()
        .unwrap()
}

fn configure(profile: &Path, home: &Path) -> Output {
    let call = format!("configure_startup_block \"$1\" '{}'", source_line());
    run(
        definitions(INSTALLER, "configure_completion_startup() {"),
        &call,
        profile,
        home,
    )
}

fn clean(profile: &Path, home: &Path) -> Output {
    run(
        definitions(UNINSTALLER, "\nremove_installed_binary \"$install_dir"),
        "clean_profile \"$1\"",
        profile,
        home,
    )
}

fn user_profile() -> String {
    "# user-owned setup\nexport EDITOR=vi\n".to_owned()
}

fn configured_profile() -> String {
    format!(
        "{}\n{}\n{}\n{}\n",
        user_profile(),
        block_begin(),
        source_line(),
        block_end()
    )
}

#[test]
fn configuring_writes_one_block_and_is_idempotent() {
    let temp = TempDir::new("profile-configure");
    let profile = temp.path().join(".zshrc");
    fs::write(&profile, user_profile()).unwrap();

    let first = configure(&profile, temp.path());
    assert!(
        first.status.success(),
        "configure failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(fs::read_to_string(&profile).unwrap(), configured_profile());
    assert!(String::from_utf8_lossy(&first.stdout).contains("Configured completion startup"));

    let second = configure(&profile, temp.path());
    assert!(second.status.success());
    assert_eq!(fs::read_to_string(&profile).unwrap(), configured_profile());
    assert!(String::from_utf8_lossy(&second.stdout).contains("already configured"));
}

#[test]
fn configuring_replaces_a_stale_block() {
    let temp = TempDir::new("profile-stale");
    let profile = temp.path().join(".zshrc");
    let stale = format!(
        "{}\n{}\nsource /old/path\n{}\n",
        user_profile(),
        block_begin(),
        block_end()
    );
    fs::write(&profile, stale).unwrap();

    let output = configure(&profile, temp.path());
    assert!(output.status.success());
    let updated = fs::read_to_string(&profile).unwrap();
    assert_eq!(updated, configured_profile());
    assert_eq!(updated.matches(&block_begin()).count(), 1, "{updated}");
    assert!(!updated.contains("source /old/path"), "{updated}");
}

#[test]
fn uninstalling_restores_the_original_profile() {
    let temp = TempDir::new("profile-clean");
    let profile = temp.path().join(".zshrc");
    fs::write(&profile, user_profile()).unwrap();

    assert!(configure(&profile, temp.path()).status.success());
    let output = clean(&profile, temp.path());
    assert!(
        output.status.success(),
        "clean failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(&profile).unwrap(), user_profile());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Removed"));

    // A profile without a block is left untouched.
    let second = clean(&profile, temp.path());
    assert!(second.status.success());
    assert!(second.stdout.is_empty());
    assert_eq!(fs::read_to_string(&profile).unwrap(), user_profile());
}

#[test]
fn profile_edits_preserve_mode_and_symlinks() {
    let temp = TempDir::new("profile-symlink");
    let target = temp.path().join("profile");
    let zshrc = temp.path().join(".zshrc");
    fs::write(&target, user_profile()).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
    symlink("profile", &zshrc).unwrap();

    for output in [configure(&zshrc, temp.path()), clean(&zshrc, temp.path())] {
        assert!(
            output.status.success(),
            "profile edit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert!(
        fs::symlink_metadata(&zshrc)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::metadata(&target).unwrap().mode() & 0o777, 0o640);
    assert_eq!(fs::read_to_string(&target).unwrap(), user_profile());
}

#[test]
fn profile_edits_refuse_to_break_hard_links() {
    let temp = TempDir::new("profile-hard-link");
    let profile = temp.path().join(".zshrc");
    let linked = temp.path().join("linked-zshrc");
    fs::write(&profile, configured_profile()).unwrap();
    fs::hard_link(&profile, &linked).unwrap();

    for output in [
        configure(&profile, temp.path()),
        clean(&profile, temp.path()),
    ] {
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Skipped hard-linked profile"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert_eq!(
        fs::metadata(&profile).unwrap().ino(),
        fs::metadata(&linked).unwrap().ino()
    );
    assert_eq!(fs::read_to_string(&profile).unwrap(), configured_profile());
}
