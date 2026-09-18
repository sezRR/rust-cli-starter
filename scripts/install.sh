#!/usr/bin/env bash
set -euo pipefail

# --- template configuration --------------------------------------------------
# bin_name must match the package name in Cargo.toml.
# repository must match its `repository` field, as OWNER/REPO.
# Both can be overridden through the environment, using the uppercased binary
# name as the prefix: GREET_REPOSITORY, GREET_VERSION, GREET_RELEASE_URL,
# GREET_SHELL.
bin_name="greet"
repository="OWNER/REPO"
# -----------------------------------------------------------------------------

env_prefix="$(printf '%s' "$bin_name" | tr '[:lower:]-' '[:upper:]_')"

read_env() {
    local name="${env_prefix}_$1"
    printf '%s\n' "${!name:-}"
}

block_begin="# >>> $bin_name completions >>>"
block_end="# <<< $bin_name completions <<<"

install_dir="$HOME/.local/bin"
termux_prefix=""
if [[ -n "${PREFIX:-}" &&
    ( -n "${TERMUX_VERSION:-}" ||
    "${PREFIX:-}" == */com.termux/files/usr ||
    ( -n "${ANDROID_ROOT:-}" && -n "${ANDROID_DATA:-}" ) ) ]]; then
    termux_prefix="${PREFIX:-}"
    install_dir="$termux_prefix/bin"
fi
home_zsh_completion="$HOME/.local/share/zsh/site-functions/_$bin_name"
home_bash_completion="$HOME/.local/share/bash-completion/completions/$bin_name"
home_fish_completion="$HOME/.config/fish/completions/$bin_name.fish"
zsh_completion="$home_zsh_completion"
bash_completion="$home_bash_completion"
fish_completion="$home_fish_completion"
if [[ -n "$termux_prefix" ]]; then
    zsh_completion="$termux_prefix/share/zsh/site-functions/_$bin_name"
    bash_completion="$termux_prefix/share/bash-completion/completions/$bin_name.bash"
    fish_completion="$termux_prefix/share/fish/vendor_completions.d/$bin_name.fish"
fi

detect_target() {
    local os architecture libc libc_details
    os="$(uname -s)"
    architecture="$(uname -m)"

    case "$architecture" in
        x86_64 | amd64) architecture="x86_64" ;;
        i386 | i486 | i586 | i686) architecture="i686" ;;
        arm64 | aarch64) architecture="aarch64" ;;
        arm | armv7 | armv7l | armv8l) architecture="armv7" ;;
        *)
            printf 'Unsupported architecture: %s\n' "$architecture" >&2
            return 1
            ;;
    esac

    case "$os" in
        Darwin)
            if [[ "$architecture" != x86_64 && "$architecture" != aarch64 ]]; then
                printf 'Unsupported macOS architecture: %s\n' "$architecture" >&2
                return 1
            fi
            printf '%s-apple-darwin\n' "$architecture"
            ;;
        Linux)
            if [[ -n "${TERMUX_VERSION:-}" ||
                "${PREFIX:-}" == */com.termux/files/usr ||
                ( -n "${ANDROID_ROOT:-}" && -n "${ANDROID_DATA:-}" ) ||
                "$(uname -o 2>/dev/null || true)" == Android ]]; then
                if [[ "$architecture" == armv7 ]]; then
                    printf 'armv7-linux-androideabi\n'
                else
                    printf '%s-linux-android\n' "$architecture"
                fi
                return
            fi

            if [[ "$architecture" == i686 ]]; then
                printf 'Unsupported Linux architecture: %s\n' "$architecture" >&2
                return 1
            fi

            if [[ -f /etc/alpine-release ]] || compgen -G '/lib/ld-musl-*.so.1' >/dev/null; then
                libc="musl"
            elif command -v getconf >/dev/null 2>&1 && getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
                libc="gnu"
            else
                libc_details="$(ldd --version 2>&1 || true)"
                case "$libc_details" in
                    *musl*) libc="musl" ;;
                    *GNU* | *GLIBC* | *glibc*) libc="gnu" ;;
                    *)
                        printf 'Unable to detect the Linux C library.\n' >&2
                        return 1
                        ;;
                esac
            fi

            if [[ "$architecture" == armv7 ]]; then
                printf 'armv7-unknown-linux-%seabihf\n' "$libc"
            else
                printf '%s-unknown-linux-%s\n' "$architecture" "$libc"
            fi
            ;;
        *)
            printf 'Unsupported operating system: %s\n' "$os" >&2
            return 1
            ;;
    esac
}

detect_shell() {
    local candidate parent_command parent_pid
    local -a candidates
    candidates=("$(read_env SHELL)")

    parent_pid="$PPID"
    if command -v ps >/dev/null 2>&1; then
        for _ in 1 2; do
            parent_command="$(ps -p "$parent_pid" -o comm= 2>/dev/null || true)"
            candidates+=("$parent_command")
            parent_pid="$(ps -p "$parent_pid" -o ppid= 2>/dev/null || true)"
            parent_pid="${parent_pid//[[:space:]]/}"
            [[ "$parent_pid" =~ ^[0-9]+$ ]] || break
        done
    fi
    candidates+=("${SHELL:-}")

    for candidate in "${candidates[@]}"; do
        candidate="${candidate//[[:space:]]/}"
        candidate="${candidate##*/}"
        candidate="${candidate#-}"
        case "$candidate" in
            bash | zsh | fish)
                printf '%s\n' "$candidate"
                return
                ;;
        esac
    done

    return 1
}

is_generated_completion() {
    local path="$1"

    [[ -f "$path" ]] &&
        grep -Fq "@generated by usage-argv for \`$bin_name __complete_word__" "$path"
}

install_completion() {
    local shell_name="$1"
    local completion_file="$2"

    if [[ -e "$completion_file" || -L "$completion_file" ]] &&
        ! is_generated_completion "$completion_file"; then
        printf 'Refusing to overwrite unrelated completion file %s\n' "$completion_file" >&2
        return 1
    fi

    mkdir -p "$(dirname -- "$completion_file")"
    temporary_completion="$completion_file.tmp.$$"
    "$installed_bin" completion --shell "$shell_name" > "$temporary_completion"
    chmod 0644 "$temporary_completion"
    mv -f "$temporary_completion" "$completion_file"
    temporary_completion=""
    printf 'Installed %s completion to %s\n' "$shell_name" "$completion_file"
}

resolve_symlink() {
    local path="$1"
    local link_target
    local depth=0

    while [[ -L "$path" ]]; do
        link_target="$(readlink "$path")"
        if [[ "$link_target" == /* ]]; then
            path="$link_target"
        else
            path="$(dirname -- "$path")/$link_target"
        fi
        depth=$((depth + 1))
        if (( depth > 20 )); then
            return 1
        fi
    done

    printf '%s\n' "$path"
}

hard_link_count() {
    local path="$1"

    if [[ "$(uname -s)" == Darwin ]]; then
        stat -f '%l' "$path"
    else
        stat -c '%h' "$path"
    fi
}

file_mode() {
    local path="$1"

    if [[ "$(uname -s)" == Darwin ]]; then
        stat -f '%Lp' "$path"
    else
        stat -c '%a' "$path"
    fi
}

# Print the profile without its managed block, dropping the blank line that
# separates the block from whatever the user wrote above it.
strip_block() {
    awk -v begin="$block_begin" -v end="$block_end" '
        $0 == "" { held++; next }
        $0 == begin {
            for (index_ = 1; index_ < held; index_++) print ""
            held = 0
            inside = 1
            next
        }
        inside && $0 == end { inside = 0; next }
        inside { next }
        { while (held > 0) { print ""; held-- } print }
        END { while (held > 0) { print ""; held-- } }
    ' "$1"
}

# Replace a profile with new contents, preserving its mode and its symlink.
replace_profile() {
    local profile_file="$1" contents_file="$2"
    local replacement_file original_mode

    if ! replacement_file="$(mktemp "${profile_file}.$bin_name-replacement.XXXXXX")"; then
        return 1
    fi
    if ! original_mode="$(file_mode "$profile_file")" ||
        ! cp -p "$profile_file" "$replacement_file" ||
        ! chmod u+w "$replacement_file" ||
        ! cat "$contents_file" > "$replacement_file" ||
        ! chmod "$original_mode" "$replacement_file" ||
        ! mv -f "$replacement_file" "$profile_file"; then
        rm -f -- "$replacement_file"
        return 1
    fi
}

configure_startup_block() {
    local config_file="$1" source_line="$2"
    local profile_file updated_file link_count

    mkdir -p "$(dirname -- "$config_file")"
    [[ -e "$config_file" ]] || touch "$config_file"

    profile_file="$(resolve_symlink "$config_file")" || {
        printf 'Skipped unresolved profile symlink %s\n' "$config_file" >&2
        return 0
    }
    link_count="$(hard_link_count "$profile_file")"
    if (( link_count > 1 )); then
        printf 'Skipped hard-linked profile %s\n' "$config_file" >&2
        return 0
    fi

    updated_file="$(mktemp "${profile_file}.$bin_name-update.XXXXXX")"
    {
        strip_block "$profile_file"
        printf '\n%s\n%s\n%s\n' "$block_begin" "$source_line" "$block_end"
    } > "$updated_file"

    if cmp -s "$profile_file" "$updated_file"; then
        rm -f -- "$updated_file"
        printf 'Completion startup is already configured in %s\n' "$config_file"
        return 0
    fi

    if ! replace_profile "$profile_file" "$updated_file"; then
        rm -f -- "$updated_file"
        return 1
    fi
    rm -f -- "$updated_file"
    printf 'Configured completion startup in %s\n' "$config_file"
}

configure_completion_startup() {
    local shell_name="$1"
    local login_config_file source_line zsh_config_dir

    case "$shell_name" in
        zsh)
            zsh_config_dir="${ZDOTDIR:-$HOME}"
            [[ "$zsh_config_dir" == /* ]] || zsh_config_dir="$HOME"
            if [[ -n "$termux_prefix" ]]; then
                source_line="[ -s \"\$PREFIX/share/zsh/site-functions/_$bin_name\" ] && . \"\$PREFIX/share/zsh/site-functions/_$bin_name\""
            else
                source_line="[ -s \"\$HOME/.local/share/zsh/site-functions/_$bin_name\" ] && . \"\$HOME/.local/share/zsh/site-functions/_$bin_name\""
            fi
            configure_startup_block "$zsh_config_dir/.zshrc" "$source_line"
            ;;
        bash)
            if [[ -n "$termux_prefix" ]]; then
                source_line="if [ -n \"\${BASH_VERSION:-}\" ] && [ -s \"\$PREFIX/share/bash-completion/completions/$bin_name.bash\" ]; then . \"\$PREFIX/share/bash-completion/completions/$bin_name.bash\"; fi"
            else
                source_line="if [ -n \"\${BASH_VERSION:-}\" ] && [ -s \"\$HOME/.local/share/bash-completion/completions/$bin_name\" ]; then . \"\$HOME/.local/share/bash-completion/completions/$bin_name\"; fi"
            fi
            configure_startup_block "$HOME/.bashrc" "$source_line"

            if [[ -f "$HOME/.bash_profile" ]]; then
                login_config_file="$HOME/.bash_profile"
            elif [[ -f "$HOME/.bash_login" ]]; then
                login_config_file="$HOME/.bash_login"
            elif [[ -f "$HOME/.profile" ]]; then
                login_config_file="$HOME/.profile"
            else
                login_config_file="$HOME/.bash_profile"
            fi
            configure_startup_block "$login_config_file" "$source_line"
            ;;
    esac
}

remove_inactive_completion() {
    local path="$1"

    if is_generated_completion "$path"; then
        rm -f -- "$path"
        printf 'Removed inactive shell completion from %s\n' "$path"
    fi
}

target="$(detect_target)"
active_shell="$(detect_shell || true)"
completion_file=""
case "$active_shell" in
    bash) completion_file="$bash_completion" ;;
    zsh) completion_file="$zsh_completion" ;;
    fish) completion_file="$fish_completion" ;;
esac
archive_name="$bin_name-$target.tar.gz"
checksum_name="$bin_name-$target.sha256"

version="$(read_env VERSION)"
[[ -n "$version" ]] || version="latest"
release_url="$(read_env RELEASE_URL)"
repository_override="$(read_env REPOSITORY)"
[[ -z "$repository_override" ]] || repository="$repository_override"

if [[ "$version" != latest ]]; then
    [[ "$version" == v* ]] || version="v$version"
fi

if [[ -z "$release_url" && "$repository" == "OWNER/REPO" ]]; then
    printf 'Set repository in this installer (or %s_REPOSITORY) to your OWNER/REPO first.\n' \
        "$env_prefix" >&2
    exit 1
fi

for required_command in tar install; do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf '%s is required to install %s.\n' "$required_command" "$bin_name" >&2
        exit 1
    fi
done

if [[ -n "$release_url" ]]; then
    required_downloader="curl"
else
    required_downloader="gh"
fi

if ! command -v "$required_downloader" >/dev/null 2>&1; then
    printf '%s is required to download %s.\n' "$required_downloader" "$bin_name" >&2
    exit 1
fi

temporary_dir="$(mktemp -d)"
temporary_completion=""
trap 'rm -rf "$temporary_dir"; [[ -z "$temporary_completion" ]] || rm -f "$temporary_completion"' EXIT

printf 'Downloading %s %s for %s...\n' "$bin_name" "$version" "$target"
if [[ -n "$release_url" ]]; then
    release_url="${release_url%/}"
    curl --fail --location --silent --show-error \
        "$release_url/$archive_name" \
        --output "$temporary_dir/$archive_name"
    curl --fail --location --silent --show-error \
        "$release_url/$checksum_name" \
        --output "$temporary_dir/$checksum_name"
else
    download_args=(
        --repo "$repository"
        --dir "$temporary_dir"
        --pattern "$archive_name"
        --pattern "$checksum_name"
    )
    if [[ "$version" == latest ]]; then
        gh release download "${download_args[@]}"
    else
        gh release download "$version" "${download_args[@]}"
    fi
fi

(
    cd -- "$temporary_dir"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum --check "$checksum_name"
    elif command -v shasum >/dev/null 2>&1; then
        shasum --algorithm 256 --check "$checksum_name"
    else
        printf 'sha256sum or shasum is required to verify %s.\n' "$bin_name" >&2
        exit 1
    fi
)

if [[ -n "$completion_file" && ( -e "$completion_file" || -L "$completion_file" ) ]] &&
    ! is_generated_completion "$completion_file"; then
    printf 'Refusing to overwrite unrelated completion file %s\n' "$completion_file" >&2
    exit 1
fi

tar -xzf "$temporary_dir/$archive_name" -C "$temporary_dir" "$bin_name"
mkdir -p "$install_dir"
install -m 0755 "$temporary_dir/$bin_name" "$install_dir/$bin_name"
installed_bin="$install_dir/$bin_name"

printf 'Installed %s to %s\n' "$bin_name" "$installed_bin"

if [[ -n "$completion_file" ]]; then
    for candidate_completion in \
        "$zsh_completion" \
        "$bash_completion" \
        "$fish_completion" \
        "$home_zsh_completion" \
        "$home_bash_completion" \
        "$home_fish_completion"; do
        if [[ "$candidate_completion" != "$completion_file" ]]; then
            remove_inactive_completion "$candidate_completion"
        fi
    done
    install_completion "$active_shell" "$completion_file"
    configure_completion_startup "$active_shell"
    printf 'Installed persistent completion for the detected %s shell.\n' "$active_shell"
    printf 'Enable it in this shell with: source "%s"\n' "$completion_file"
else
    printf 'Could not detect Bash, Zsh, or Fish; skipped completion installation.\n' >&2
fi

rm -rf "$temporary_dir"
trap - EXIT
