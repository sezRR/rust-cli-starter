#![cfg(unix)]

mod common;

use std::{env, fs, os::unix::fs::PermissionsExt, process::Command};

use common::TempDir;

const BIN: &str = env!("CARGO_PKG_NAME");

/// Prefix of this CLI's environment variables, matching `src/cli/upgrade.rs`.
fn env_prefix() -> String {
    BIN.to_uppercase().replace('-', "_")
}

fn env_var(suffix: &str) -> String {
    format!("{}_{suffix}", env_prefix())
}

fn mock_path(temp: &TempDir, script: &str) -> std::ffi::OsString {
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).unwrap();

    let gh = bin.join("gh");
    fs::write(&gh, script).unwrap();
    let mut permissions = fs::metadata(&gh).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&gh, permissions).unwrap();

    let mut paths = vec![bin];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap()));
    env::join_paths(paths).unwrap()
}

#[test]
fn current_version_skips_install() {
    let temp = TempDir::new("upgrade-current");
    let install_marker = temp.path().join("installer-ran");
    let script = format!(
        r#"#!/usr/bin/env bash
if [[ "$1" == release ]]; then
    printf 'v{}\n'
else
    touch "$INSTALL_MARKER"
fi
"#,
        env!("CARGO_PKG_VERSION")
    );
    let path = mock_path(&temp, &script);

    let output = Command::new(env!("CARGO_BIN_EXE_greet"))
        .arg("upgrade")
        .env("PATH", path)
        .env(env_var("REPOSITORY"), "owner/repo")
        .env("SHELL", "/bin/zsh")
        .env_remove(env_var("SHELL"))
        .env_remove("PREFIX")
        .env_remove("TERMUX_VERSION")
        .env_remove("ANDROID_ROOT")
        .env_remove("ANDROID_DATA")
        .env("INSTALL_MARKER", &install_marker)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(!install_marker.exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("Current version: {}", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains(&format!("{BIN} is already up to date")));
    assert!(stdout.contains(&format!(
        "Reload Zsh completion with: source \"$HOME/.local/share/zsh/site-functions/_{BIN}\""
    )));
    assert!(!stdout.contains("autoload -Uz compinit"));
    assert!(!stdout.contains("Bash completion"));
    assert!(!stdout.contains("Fish completion"));
}

#[test]
fn current_version_uses_termux_completion_path() {
    let temp = TempDir::new("upgrade-current-termux");
    let script = format!(
        "#!/usr/bin/env bash\nprintf 'v{}\\n'\n",
        env!("CARGO_PKG_VERSION")
    );
    let path = mock_path(&temp, &script);

    let output = Command::new(env!("CARGO_BIN_EXE_greet"))
        .arg("upgrade")
        .env("PATH", path)
        .env(env_var("REPOSITORY"), "owner/repo")
        .env("SHELL", "/data/data/com.termux/files/usr/bin/bash")
        .env("PREFIX", "/data/data/com.termux/files/usr")
        .env("TERMUX_VERSION", "0.118.3")
        .env_remove(env_var("SHELL"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!(
        "source \"/data/data/com.termux/files/usr/share/bash-completion/completions/{BIN}.bash\""
    )));
    assert!(!stdout.contains("$HOME/.local/share/bash-completion"));
}

#[test]
fn upgrade_pins_queried_release() {
    let temp = TempDir::new("upgrade-newer");
    let install_marker = temp.path().join("installed-version");
    let script = r#"#!/usr/bin/env bash
if [[ "$1" == release ]]; then
    printf 'v9.9.9\n'
else
    cat <<'INSTALLER'
#!/usr/bin/env bash
test -z "${__PREFIX___RELEASE_URL:-}" || exit 92
printf '%s\n' "$__PREFIX___VERSION" > "$INSTALL_MARKER"
INSTALLER
fi
"#
    .replace("__PREFIX__", &env_prefix());
    let path = mock_path(&temp, &script);

    let output = Command::new(env!("CARGO_BIN_EXE_greet"))
        .arg("upgrade")
        .env("PATH", path)
        .env(env_var("REPOSITORY"), "owner/repo")
        .env(env_var("VERSION"), "v0.0.1")
        .env(env_var("RELEASE_URL"), "https://example.invalid")
        .env("INSTALL_MARKER", &install_marker)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(install_marker).unwrap(), "v9.9.9\n");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Latest version:  9.9.9"));
    assert!(stdout.contains(&format!("{BIN} was upgraded to 9.9.9")));
}

#[test]
fn release_lookup_failure_is_propagated() {
    let temp = TempDir::new("upgrade-failure");
    let path = mock_path(&temp, "#!/usr/bin/env bash\nexit 43\n");

    let output = Command::new(env!("CARGO_BIN_EXE_greet"))
        .arg("upgrade")
        .env("PATH", path)
        .env(env_var("REPOSITORY"), "owner/repo")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(43));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to check the latest release"));
}

#[test]
fn unconfigured_repository_is_reported() {
    if !env!("CARGO_PKG_REPOSITORY").ends_with("OWNER/REPO") {
        println!("the repository is configured; skipping");
        return;
    }

    let temp = TempDir::new("upgrade-unconfigured");
    let path = mock_path(&temp, "#!/usr/bin/env bash\nprintf 'v9.9.9\\n'\n");

    let output = Command::new(env!("CARGO_BIN_EXE_greet"))
        .arg("upgrade")
        .env("PATH", path)
        .env_remove(env_var("REPOSITORY"))
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("set `repository` in Cargo.toml"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
