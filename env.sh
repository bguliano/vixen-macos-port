# Source this before running cargo by hand:  source ./env.sh
#
# A normal rustup install (https://rustup.rs) is all you need: rust-toolchain.toml pins the Rust version and
# rustup installs it automatically the first time cargo runs here. If this checkout has its own project-local
# toolchain in .tools/ (an older development setup), that one is used instead.
_VIXEN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
if [ -d "$_VIXEN_ROOT/.tools/rustup" ]; then
    export RUSTUP_HOME="$_VIXEN_ROOT/.tools/rustup"
    export CARGO_HOME="$_VIXEN_ROOT/.tools/cargo"
    export PATH="$CARGO_HOME/bin:$PATH"
elif [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust isn't installed. Install it from https://rustup.rs, then open a new terminal and try again." >&2
fi
