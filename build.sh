#!/usr/bin/env bash
set -euo pipefail
trap 'printf "Build failed at line %s. Fix the error above and run bash build.sh again.\n" "$LINENO" >&2' ERR

if [[ $# -ne 0 ]]; then
    printf 'Usage: bash build.sh\n' >&2
    exit 1
fi
if [[ $(uname -s) != Linux ]]; then
    printf 'This project requires Linux. On Windows, run this script inside WSL2.\n' >&2
    exit 1
fi
if [[ -n ${SUDO_USER:-} && $SUDO_USER != root ]]; then
    printf 'Run bash build.sh without sudo. The script uses sudo only for system packages.\n' >&2
    exit 1
fi

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
[[ -f Cargo.toml && -f Cargo.lock ]] || { printf 'Cargo.toml and Cargo.lock must be beside build.sh.\n' >&2; exit 1; }
[[ -r /etc/os-release ]] || { printf 'Cannot detect Linux distribution: /etc/os-release is missing.\n' >&2; exit 1; }
. /etc/os-release
printf 'System: %s (%s)\n' "${PRETTY_NAME:-${ID:-Linux}}" "$(uname -m)"

build_tmp=$(mktemp -d)
trap 'rm -rf -- "$build_tmp"' EXIT
if ! command -v curl >/dev/null || ! printf 'int main(void) { return 0; }\n' | cc -x c - -o "$build_tmp/link-check" 2>/dev/null; then
    admin=()
    if [[ $(id -u) != 0 ]]; then
        command -v sudo >/dev/null || { printf 'Missing build tools. Ask an administrator to install a C compiler/linker, curl, and CA certificates.\n' >&2; exit 1; }
        admin=(sudo)
    fi
    printf 'Installing system build tools (sudo may request your login password)...\n'
    case " ${ID:-} ${ID_LIKE:-} " in
        *' debian '*|*' ubuntu '*)
            "${admin[@]}" apt-get update
            "${admin[@]}" apt-get install -y build-essential curl ca-certificates
            ;;
        *' fedora '*|*' rhel '*|*' centos '*|*' rocky '*|*' almalinux '*)
            package_manager=dnf
            command -v dnf >/dev/null || package_manager=yum
            "${admin[@]}" "$package_manager" install -y gcc glibc-devel curl ca-certificates
            ;;
        *' suse '*|*' opensuse '*|*' opensuse-leap '*|*' opensuse-tumbleweed '*)
            "${admin[@]}" zypper --non-interactive install gcc glibc-devel curl ca-certificates
            ;;
        *' arch '*|*' manjaro '*)
            "${admin[@]}" pacman -S --needed --noconfirm base-devel curl ca-certificates
            ;;
        *)
            printf 'Automatic package installation is unavailable for %s. Install a C compiler/linker, curl, and CA certificates, then rerun.\n' "${ID:-this distribution}" >&2
            exit 1
            ;;
    esac
fi
# Check the linker too: a compiler command alone does not guarantee libc is installed.
printf 'int main(void) { return 0; }\n' | cc -x c - -o "$build_tmp/link-check"

required_rust=$(sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml)
[[ $required_rust =~ ^([0-9]+)\.([0-9]+)(\.([0-9]+))?$ ]] || { printf 'Cannot read rust-version from Cargo.toml.\n' >&2; exit 1; }
required_major=${BASH_REMATCH[1]}
required_minor=${BASH_REMATCH[2]}
required_patch=${BASH_REMATCH[4]:-0}
rust_is_ready() {
    local version
    version=$("$@" --version 2>/dev/null) || return 1
    [[ $version =~ ^rustc\ ([0-9]+)\.([0-9]+)\.([0-9]+) ]] || return 1
    (( BASH_REMATCH[1] > required_major ||
       (BASH_REMATCH[1] == required_major && BASH_REMATCH[2] > required_minor) ||
       (BASH_REMATCH[1] == required_major && BASH_REMATCH[2] == required_minor && BASH_REMATCH[3] >= required_patch) ))
}

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
cargo_command=(cargo)
if ! rust_is_ready rustc || ! cargo --version >/dev/null 2>&1; then
    if ! command -v rustup >/dev/null; then
        printf 'Downloading the official Rust installer...\n'
        curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$build_tmp/rustup-init.sh"
        sh "$build_tmp/rustup-init.sh" -y --profile minimal --default-toolchain stable --no-modify-path
    else
        printf 'Installing/updating the stable Rust toolchain...\n'
        rustup toolchain install stable --profile minimal
    fi
    rust_is_ready rustup run stable rustc || { printf 'Stable Rust does not meet the required version %s.\n' "$required_rust" >&2; exit 1; }
    cargo_command=(rustup run stable cargo)
fi

printf 'Building nfv9gen...\n'
# Keep the executable in the documented location even if CARGO_TARGET_DIR is set.
"${cargo_command[@]}" build --release --locked --target-dir "$PWD/target"
./target/release/nfv9gen --version
./target/release/nfv9gen validate examples/basic.json
./target/release/nfv9gen preview examples/basic.json --kind data --hex
printf '\nBuild complete: %s/target/release/nfv9gen\n' "$PWD"
printf 'Next: start your collector, then use the send command in README.md.\n'
