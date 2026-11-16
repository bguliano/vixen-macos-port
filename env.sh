# Source this to use the project-local Rust toolchain:  source ./env.sh
_VIXEN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
export RUSTUP_HOME="$_VIXEN_ROOT/.tools/rustup"
export CARGO_HOME="$_VIXEN_ROOT/.tools/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
# Several workers build at once on this laptop, and each cargo would otherwise run a job per core.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}"
