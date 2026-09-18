use std::process::{Command, Stdio};

use usage::{Args, Run};

/// The binary name, the release asset name, and the completion file name all
/// follow the package name, so renaming the package in `Cargo.toml` is enough.
const BIN: &str = env!("CARGO_PKG_NAME");

/// Prefix for this CLI's environment variables, for example `GREET_SHELL`.
fn env_prefix() -> String {
    BIN.to_uppercase().replace('-', "_")
}

fn env_var(suffix: &str) -> Option<String> {
    std::env::var(format!("{}_{suffix}", env_prefix())).ok()
}

/// `OWNER/REPO`, taken from the `repository` field in `Cargo.toml`, or from
/// `GREET_REPOSITORY` when it is set.
fn repository() -> String {
    if let Some(repository) = env_var("REPOSITORY").filter(|value| !value.is_empty()) {
        return repository;
    }

    let configured = env!("CARGO_PKG_REPOSITORY");
    let slug = configured
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit_once("github.com")
        .map(|(_, path)| path.trim_start_matches([':', '/']))
        .unwrap_or("");

    if slug.is_empty() || slug == "OWNER/REPO" {
        eprintln!(
            "set `repository` in Cargo.toml (or {}_REPOSITORY) to your GitHub repository before using `{BIN} upgrade`",
            env_prefix()
        );
        std::process::exit(1);
    }
    slug.to_owned()
}

fn print_completion_hint() {
    let shell = env_var("SHELL").or_else(|| std::env::var("SHELL").ok());
    let shell = shell
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str());
    let termux_prefix = std::env::var("PREFIX").ok().filter(|prefix| {
        !prefix.is_empty()
            && (std::env::var_os("TERMUX_VERSION").is_some()
                || (std::env::var_os("ANDROID_ROOT").is_some()
                    && std::env::var_os("ANDROID_DATA").is_some())
                || prefix.ends_with("/com.termux/files/usr"))
    });

    match shell {
        Some("zsh") => {
            let path = termux_prefix
                .map(|prefix| format!("{prefix}/share/zsh/site-functions/_{BIN}"))
                .unwrap_or_else(|| format!("$HOME/.local/share/zsh/site-functions/_{BIN}"));
            println!("Reload Zsh completion with: source \"{path}\"");
        }
        Some("bash") => {
            let path = termux_prefix
                .map(|prefix| format!("{prefix}/share/bash-completion/completions/{BIN}.bash"))
                .unwrap_or_else(|| format!("$HOME/.local/share/bash-completion/completions/{BIN}"));
            println!("Reload Bash completion with: source \"{path}\"");
        }
        Some("fish") => {
            let path = termux_prefix
                .map(|prefix| format!("{prefix}/share/fish/vendor_completions.d/{BIN}.fish"))
                .unwrap_or_else(|| format!("$HOME/.config/fish/completions/{BIN}.fish"));
            println!("Reload Fish completion with: source \"{path}\"");
        }
        _ => println!("Restart your shell to load its completion."),
    }
}

/// Upgrade to the latest release
#[derive(Args)]
pub(crate) struct Upgrade {}

impl Run for Upgrade {
    type Output = ();

    fn run(self) {
        let current_version = env!("CARGO_PKG_VERSION");
        let repository = repository();
        let latest = Command::new("gh")
            .args([
                "release",
                "view",
                "--repo",
                &repository,
                "--json",
                "tagName",
                "--jq",
                ".tagName",
            ])
            .output()
            .unwrap_or_else(|error| {
                eprintln!("failed to start GitHub CLI: {error}");
                std::process::exit(1);
            });

        if !latest.status.success() {
            eprint!("{}", String::from_utf8_lossy(&latest.stderr));
            eprintln!("failed to check the latest release; run `gh auth login` and try again");
            std::process::exit(latest.status.code().unwrap_or(1));
        }

        let latest_tag = String::from_utf8_lossy(&latest.stdout).trim().to_owned();
        let latest_version = latest_tag.strip_prefix('v').unwrap_or(&latest_tag);
        if latest_version.is_empty() {
            eprintln!("latest release did not return a version tag");
            std::process::exit(1);
        }

        println!("Current version: {current_version}");
        println!("Latest version:  {latest_version}");
        if current_version == latest_version {
            println!("{BIN} is already up to date");
            print_completion_hint();
            return;
        }
        println!("Upgrading {BIN} {current_version} -> {latest_version}...");

        let mut fetch = Command::new("gh")
            .args([
                "api",
                "-H",
                "Accept: application/vnd.github.raw+json",
                &format!("repos/{repository}/contents/scripts/install.sh"),
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| {
                eprintln!("failed to start GitHub CLI: {error}");
                std::process::exit(1);
            });

        let installer = fetch.stdout.take().expect("GitHub CLI stdout was piped");
        let install_status = Command::new("bash")
            .stdin(Stdio::from(installer))
            .env(format!("{}_VERSION", env_prefix()), &latest_tag)
            .env_remove(format!("{}_RELEASE_URL", env_prefix()))
            .status()
            .unwrap_or_else(|error| {
                let _ = fetch.kill();
                let _ = fetch.wait();
                eprintln!("failed to start installer: {error}");
                std::process::exit(1);
            });
        let fetch_status = fetch.wait().unwrap_or_else(|error| {
            eprintln!("failed to wait for GitHub CLI: {error}");
            std::process::exit(1);
        });

        if !fetch_status.success() {
            eprintln!("failed to download installer; run `gh auth login` and try again");
            std::process::exit(fetch_status.code().unwrap_or(1));
        }
        if !install_status.success() {
            eprintln!("{BIN} upgrade failed");
            std::process::exit(install_status.code().unwrap_or(1));
        }

        println!("{BIN} was upgraded to {latest_version}");
        print_completion_hint();
    }
}
