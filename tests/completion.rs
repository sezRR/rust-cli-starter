#![cfg(unix)]

mod common;

use std::{
    fs,
    process::{Command, Output},
};

use common::TempDir;

const BIN: &str = env!("CARGO_PKG_NAME");

const ZSH_COMPLETION_INIT: &str =
    "(( $+functions[compdef] )) || { autoload -Uz compinit && compinit; }";

fn generate_completion(shell: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_greet"))
        .args(["completion", "--shell", shell])
        .output()
        .unwrap()
}

#[test]
fn only_zsh_completion_initializes_its_completion_system() {
    let output = generate_completion("zsh");
    assert!(output.status.success());
    let zsh = String::from_utf8(output.stdout).unwrap();
    assert!(zsh.starts_with(&format!("#compdef {BIN}\n")), "{zsh}");
    assert_eq!(zsh.matches(ZSH_COMPLETION_INIT).count(), 1, "{zsh}");

    for shell in ["bash", "fish"] {
        let output = generate_completion(shell);
        assert!(output.status.success(), "{shell}");
        let script = String::from_utf8(output.stdout).unwrap();
        assert!(!script.contains(ZSH_COMPLETION_INIT), "{shell}: {script}");
    }
}

#[test]
fn sourced_zsh_completion_registers_in_a_clean_shell() {
    if Command::new("zsh").arg("--version").output().is_err() {
        println!("zsh is not installed; skipping");
        return;
    }

    let generated = generate_completion("zsh");
    assert!(generated.status.success());
    let temp = TempDir::new("zsh-completion");
    fs::write(temp.path().join(format!("_{BIN}")), generated.stdout).unwrap();

    let output = Command::new("zsh")
        .args([
            "-f",
            "-c",
            &format!("ZDOTDIR=$PWD; source ./_{BIN}; print -r -- ${{_comps[{BIN}]-missing}}"),
        ])
        .current_dir(temp.path())
        .env("HOME", temp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "zsh failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("_{BIN}")
    );
}

#[test]
fn direct_zsh_install_writes_the_self_initializing_script() {
    let temp = TempDir::new("zsh-install");
    let data_home = temp.path().join("data");
    let home = temp.path().join("home");
    let output = Command::new(env!("CARGO_BIN_EXE_greet"))
        .args(["completion", "--shell", "zsh", "--install"])
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &data_home)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let installed = data_home.join(format!("zsh/site-functions/_{BIN}"));
    let script = fs::read_to_string(&installed).unwrap();
    assert!(script.starts_with(&format!("#compdef {BIN}\n")), "{script}");
    assert_eq!(script.matches(ZSH_COMPLETION_INIT).count(), 1, "{script}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("source '{}'", installed.display())));
    assert!(!stdout.contains("autoload -Uz compinit"), "{stdout}");
}
