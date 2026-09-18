# Rust CLI starter

A starter template for Rust command-line tools built on
[`usage-rs`](https://usage.jdx.dev/rust/quickstart): one example command, shell
completions for Bash, Zsh, and Fish, a self-upgrading installer, and a GitHub
Actions release pipeline that publishes verified binaries for macOS, Linux, and
Android.

The template ships as a working CLI called `greet`, versioned from `v0.1.0`.

## What you get

- `greet hello [NAME]` — the example command to replace with your own
- `greet completion --shell <bash|zsh|fish> [--install]` — generated completions
- `greet upgrade` — installs the latest release and refreshes the completion
- `scripts/install.sh` / `scripts/uninstall.sh` — installers that verify SHA-256
  checksums and manage a single marked block in your shell profile
- `.github/workflows/release.yml` — tag `vX.Y.Z`, get a draft release, 12
  target binaries, and an automatic publish
- `Makefile` — `run`, `test`, `lint`, `verify`, `install`, `release`

## Make it yours

1. Create your repository from this template and clone it.
2. Set `repository` in `Cargo.toml` to `https://github.com/OWNER/REPO`. Until you
   do, `greet upgrade` and `scripts/install.sh` stop with a clear error.
3. Rename the package, if you want a different binary name. Everything else
   follows the package name in `Cargo.toml` — release assets, completion files,
   environment variables, and the `Makefile` — except four string literals the
   compiler needs spelled out:

   | File | Literal |
   | --- | --- |
   | `Cargo.toml` | `name = "greet"` |
   | `src/cli/mod.rs` | `#[usage(bin = "greet", ...)]` |
   | `src/cli/hello.rs` | `#[usage(env = "GREET_NAME", ...)]` |
   | `scripts/install.sh`, `scripts/uninstall.sh` | `bin_name="greet"` |

   Rust's `env!("CARGO_BIN_EXE_greet")` in `tests/` follows the package name too,
   so rename it there as well.
4. Run `make sync-lock && make verify`.
5. Replace `src/cli/hello.rs` with your own command and register it in
   `src/cli/mod.rs`.

Environment variables use the uppercased binary name as their prefix, so a
`greet` binary reads `GREET_VERSION`, `GREET_SHELL`, `GREET_REPOSITORY`, and
`GREET_RELEASE_URL`.

## Requirements

- Bash to run the installer
- Bash, Zsh, or Fish for shell completions
- An authenticated [GitHub CLI](https://cli.github.com/) with access to the
  repository, or `GREET_RELEASE_URL` pointing at public release assets
- `tar`, `install`, and `sha256sum` or `shasum`

## Install

```sh
gh auth login
gh api \
  -H "Accept: application/vnd.github.raw+json" \
  repos/OWNER/REPO/contents/scripts/install.sh | bash
```

The installer uses your authenticated GitHub session to download and verify the
latest release. It installs persistent completion only for the invoking Bash,
Zsh, or Fish shell. For Bash and Zsh it writes one marked block to the shell's
startup files; Fish discovers its completion automatically. Set `GREET_SHELL` to
`bash`, `zsh`, or `fish` to override shell detection.

Installed files:

- Binary: `~/.local/bin/greet` (`$PREFIX/bin/greet` on Termux)
- Detected Zsh: `~/.local/share/zsh/site-functions/_greet`
- Detected Bash: `~/.local/share/bash-completion/completions/greet`
- Detected Fish: `~/.config/fish/completions/greet.fish`

On Termux, completions go under `$PREFIX/share/bash-completion/completions`,
`$PREFIX/share/zsh/site-functions`, or `$PREFIX/share/fish/vendor_completions.d`.
On desktop systems, make sure `~/.local/bin` is on your `PATH`.

To install a specific release:

```sh
gh api \
  -H "Accept: application/vnd.github.raw+json" \
  repos/OWNER/REPO/contents/scripts/install.sh | GREET_VERSION=v0.1.0 bash
```

From a checkout, `make install` and `make install-version VERSION=0.1.0` do the
same thing.

### Termux on Android

```sh
pkg update
pkg install bash coreutils gh tar
gh auth login
gh api \
  -H "Accept: application/vnd.github.raw+json" \
  repos/OWNER/REPO/contents/scripts/install.sh | bash
```

The installer detects Termux, downloads a Bionic-linked Android binary, and
installs it to `$PREFIX/bin/greet`, which is already on the Termux `PATH`.

## Uninstall

```sh
gh api \
  -H "Accept: application/vnd.github.raw+json" \
  repos/OWNER/REPO/contents/scripts/uninstall.sh | bash
```

From a checkout, run `make uninstall`. The uninstaller removes the binary, the
generated completion files, and the marked block the installer added to your
shell profiles. It leaves unrelated files and completions intact.

## Use

```sh
greet hello
# hello, world

greet hello Alice
# hello, Alice

GREET_NAME=Jeff greet hello
# hello, Jeff

greet --help

greet upgrade
```

`greet upgrade` prints the current and latest versions first. When an update is
available, it uses your GitHub CLI session to install that release and refresh
the completion for the detected shell, then prints the reload command — a child
process cannot reload its parent shell.

To generate or install completions manually:

```sh
greet completion --shell zsh
greet completion --shell zsh --install
```

Replace `zsh` with `bash` or `fish` as needed.

## Development

Development requires Rust and Cargo. `make lint` also needs `jq`, `shellcheck`,
and `actionlint`.

```sh
make run
make test
make verify
```

## Release

Bump the version in `Cargo.toml`, then synchronize the lockfile and commit:

```sh
make sync-lock
make verify
```

Tag and push from a clean worktree:

```sh
make release
```

The workflow checks that the tag matches the Cargo package version, creates a
draft release, uploads checksummed binaries for Intel and ARM64 macOS; x86_64,
ARM64, and ARMv7 Linux (glibc and musl); and ARM64, ARMv7, x86_64, and i686
Android, then publishes the release.
