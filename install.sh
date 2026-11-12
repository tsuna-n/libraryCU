#!/usr/bin/env bash
#
# install.sh - Installer script for libraryCube (lbc)
#
# Usage:
#   ./install.sh [OPTIONS]
#
# Options:
#   --system          Install system-wide to /usr/local/bin (may prompt for sudo)
#   --prefix <DIR>    Install binary to custom directory (default: ~/.local/bin)
#   --no-build        Skip source build (use bundled lbc or target/release/lbc)
#   --archive <FILE>  Install a local release archive, verified with --sha256
#   --sha256 <HEX>    Independently trusted SHA-256 digest of --archive
#   --uninstall       Remove installed lbc binary from target directory
#   -h, --help        Show this help message and exit
#

set -euo pipefail

# Colors and formatting
if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]]; then
  BOLD="\033[1m"
  GREEN="\033[32m"
  BLUE="\033[34m"
  YELLOW="\033[33m"
  RED="\033[31m"
  RESET="\033[0m"
else
  BOLD=""
  GREEN=""
  BLUE=""
  YELLOW=""
  RED=""
  RESET=""
fi

print_info() {
  printf "${BLUE}${BOLD}==>${RESET} %s\n" "$*"
}

print_success() {
  printf "${GREEN}${BOLD}==>${RESET} ${GREEN}%s${RESET}\n" "$*"
}

print_warn() {
  printf "${YELLOW}${BOLD}WARNING:${RESET} %s\n" "$*"
}

print_error() {
  printf "${RED}${BOLD}ERROR:${RESET} %s\n" "$*" >&2
}

show_help() {
  cat << 'EOF'
libraryCube (lbc) Installer

Usage:
  ./install.sh [OPTIONS]

Options:
  --system           Install system-wide to /usr/local/bin (may prompt for sudo)
  --prefix <DIR>     Install binary to custom directory (default: ~/.local/bin)
  --no-build         Skip source build (use bundled lbc or target/release/lbc)
  --archive <FILE>   Install a local release archive, verified with --sha256
  --sha256 <HEX>     Independently trusted SHA-256 digest (64 hex characters)
  --uninstall        Remove installed lbc binary from target directory
  -h, --help         Show this help message and exit

Examples:
  ./install.sh                   # Install to ~/.local/bin
  ./install.sh --system          # Install system-wide to /usr/local/bin
  ./install.sh --prefix ~/bin    # Install to ~/bin
  ./install.sh --uninstall       # Remove lbc
EOF
}

# Resolve project root directory; archive paths remain relative to the caller.
CALLER_DIR="$PWD"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

INSTALL_DIR="${HOME}/.local/bin"
SYSTEM_INSTALL=0
DO_BUILD=1
DO_UNINSTALL=0
ARCHIVE=""
EXPECTED_SHA256=""
ARCHIVE_TEMP=""
trap 'if [[ -n "$ARCHIVE_TEMP" ]]; then rm -rf -- "$ARCHIVE_TEMP"; fi' EXIT

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --system)
      INSTALL_DIR="/usr/local/bin"
      SYSTEM_INSTALL=1
      shift
      ;;
    --prefix)
      if [[ -z "${2:-}" ]]; then
        print_error "--prefix requires a directory path"
        exit 1
      fi
      INSTALL_DIR="$2"
      shift 2
      ;;
    --no-build)
      DO_BUILD=0
      shift
      ;;
    --archive|--sha256)
      if [[ -z "${2:-}" ]]; then
        print_error "$1 requires a value"
        exit 1
      fi
      if [[ "$1" == --archive ]]; then ARCHIVE="$2"; else EXPECTED_SHA256="$2"; fi
      shift 2
      ;;
    --uninstall)
      DO_UNINSTALL=1
      shift
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    *)
      print_error "Unknown option: $1"
      echo "Run './install.sh --help' for available options."
      exit 1
      ;;
  esac
done

if [[ -n "$ARCHIVE" || -n "$EXPECTED_SHA256" ]]; then
  if [[ -z "$ARCHIVE" || ! "$EXPECTED_SHA256" =~ ^[[:xdigit:]]{64}$ || "$DO_UNINSTALL" -eq 1 ]]; then
    print_error "--archive and --sha256 require a file and an independently trusted 64-character hex digest; do not combine them with --uninstall."
    exit 1
  fi
fi
if [[ "$INSTALL_DIR" != /* ]]; then INSTALL_DIR="${CALLER_DIR}/${INSTALL_DIR}"; fi

# Verify an immutable private copy before interpreting any archive data.
if [[ -n "$ARCHIVE" ]]; then
  if [[ "$ARCHIVE" != /* ]]; then ARCHIVE="${CALLER_DIR}/${ARCHIVE}"; fi
  if [[ ! -f "$ARCHIVE" || -L "$ARCHIVE" ]]; then
    print_error "Archive must be a regular file: $ARCHIVE"
    exit 1
  fi
  ARCHIVE_TEMP="$(mktemp -d "${TMPDIR:-/tmp}/lbc-install.XXXXXXXX")"
  cp -- "$ARCHIVE" "${ARCHIVE_TEMP}/release.tar.gz"
  ARCHIVE="${ARCHIVE_TEMP}/release.tar.gz"
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_SHA256="$(sha256sum "$ARCHIVE")"
  elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_SHA256="$(shasum -a 256 "$ARCHIVE")"
  else
    print_error "SHA-256 verification requires sha256sum or shasum."
    exit 1
  fi
  ACTUAL_SHA256="${ACTUAL_SHA256%% *}"
  if [[ "$(printf '%s' "$EXPECTED_SHA256" | tr 'A-F' 'a-f')" != "$ACTUAL_SHA256" ]]; then
    print_error "SHA-256 mismatch. Installation was not changed."
    exit 1
  fi
  case "$(uname -s):$(uname -m)" in
    Linux:x86_64) ARCHIVE_TARGET="x86_64-unknown-linux-gnu" ;;
    Darwin:x86_64|Darwin:arm64) ARCHIVE_TARGET="universal-apple-darwin" ;;
    *) print_error "No supported release archive for $(uname -s) $(uname -m)."; exit 1 ;;
  esac
  if ! LC_ALL=C tar -tzf "$ARCHIVE" > "${ARCHIVE_TEMP}/names" ||
     ! LC_ALL=C tar --numeric-owner -tvzf "$ARCHIVE" > "${ARCHIVE_TEMP}/metadata"; then
    print_error "Invalid or corrupt release archive."
    exit 1
  fi
  if ! IFS= read -r FIRST_MEMBER < "${ARCHIVE_TEMP}/names"; then
    print_error "Empty release archive."
    exit 1
  fi
  PACKAGE="${FIRST_MEMBER%%/*}"
  if [[ ! "$PACKAGE" =~ ^lbc-([0-9]+\.[0-9]+\.[0-9]+)-${ARCHIVE_TARGET}$ ]]; then
    print_error "Archive package does not match this platform ($ARCHIVE_TARGET)."
    exit 1
  fi
  PACKAGE_VERSION="${BASH_REMATCH[1]}"
  # Both GNU tar and bsdtar expose final names/types, including PAX overrides.
  if ! LC_ALL=C awk -v root="$PACKAGE" '
    FNR == NR {
      name = $0; sub(/\/$/, "", name)
      if (seen[name]++ || ($0 != root "/" && $0 != root &&
          $0 != root "/lbc" && $0 != root "/install.sh" &&
          $0 != root "/README.md" && $0 != root "/CHANGELOG.md" && $0 != root "/LICENSE")) exit 1
      names[++count] = $0; next
    }
    {
      name = names[++rows]; type = substr($1, 1, 1)
      size = ($2 ~ /^[0-9]+\/[0-9]+$/) ? $3 : $5
      if (size !~ /^[0-9]+$/ || $NF != name) exit 1
      if (name == root || name == root "/") {
        if (type != "d" || size != 0) exit 1
      } else if (type != "-" || size <= 0 || size > 134217728) exit 1
    }
    END {
      if (rows != count || !seen[root "/lbc"] || !seen[root "/install.sh"] ||
          !seen[root "/README.md"] || !seen[root "/CHANGELOG.md"] || !seen[root "/LICENSE"]) exit 1
    }
  ' "${ARCHIVE_TEMP}/names" "${ARCHIVE_TEMP}/metadata"; then
    print_error "Unsafe, unexpected, duplicate, missing, or oversized archive member."
    exit 1
  fi
  # Extract only the binary as data; never run the packaged installer.
  if ! tar -xOzf "$ARCHIVE" "$PACKAGE/lbc" > "${ARCHIVE_TEMP}/lbc"; then
    print_error "Could not read the verified archive binary."
    exit 1
  fi
  MAGIC="$(od -An -tx1 -N4 "${ARCHIVE_TEMP}/lbc" | tr -d ' \n')"
  case "$ARCHIVE_TARGET:$MAGIC" in
    x86_64-unknown-linux-gnu:7f454c46|universal-apple-darwin:cafebabe|universal-apple-darwin:cafebabf|universal-apple-darwin:cffaedfe|universal-apple-darwin:feedfacf) ;;
    *) print_error "Archive does not contain a native executable for $ARCHIVE_TARGET."; exit 1 ;;
  esac
  chmod 755 "${ARCHIVE_TEMP}/lbc"
  if ! ARCHIVE_VERSION="$("${ARCHIVE_TEMP}/lbc" --version 2>/dev/null)" || [[ "$ARCHIVE_VERSION" != "lbc $PACKAGE_VERSION" ]]; then
    print_error "Archive binary version verification failed. Installation was not changed."
    exit 1
  fi
  DO_BUILD=0
fi

SOURCE_TREE=0
if [[ -f "Cargo.toml" ]] && grep -q 'name = "librarycube"' Cargo.toml 2>/dev/null; then
  SOURCE_TREE=1
fi
BUNDLED_BIN="${SCRIPT_DIR}/lbc"

if [[ "$DO_UNINSTALL" -eq 0 ]] && [[ "$SOURCE_TREE" -eq 0 ]] && [[ ! -f "$BUNDLED_BIN" ]] && [[ -z "$ARCHIVE" ]]; then
  print_error "No bundled lbc binary or libraryCube source tree was found beside install.sh."
  exit 1
fi

# Handle uninstall
if [[ "$DO_UNINSTALL" -eq 1 ]]; then
  TARGET_BIN="${INSTALL_DIR}/lbc"
  if [[ -f "$TARGET_BIN" ]]; then
    print_info "Removing ${TARGET_BIN}..."
    if [[ -w "$INSTALL_DIR" ]]; then
      rm -f "$TARGET_BIN"
    else
      if command -v sudo >/dev/null 2>&1; then
        sudo rm -f "$TARGET_BIN"
      else
        rm -f "$TARGET_BIN"
      fi
    fi
    print_success "libraryCube (lbc) uninstalled successfully from ${INSTALL_DIR}."
  else
    print_warn "Binary not found at ${TARGET_BIN}. Nothing to uninstall."
  fi
  exit 0
fi

print_info "Preparing to install libraryCube (lbc)..."
print_info "Destination directory: ${INSTALL_DIR}"

# Check Rust toolchain
ensure_rust() {
  if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    return 0
  fi

  print_warn "Rust toolchain (cargo/rustc) was not found on your system."

  if [[ -t 0 ]]; then
    printf "${BOLD}Would you like to install Rust using rustup now? [Y/n]: ${RESET}"
    read -r response
    case "${response}" in
      [yY][eE][sS]|[yY]|"")
        print_info "Installing Rust via rustup..."
        if command -v curl >/dev/null 2>&1; then
          curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        elif command -v wget >/dev/null 2>&1; then
          wget -qO- https://sh.rustup.rs | sh -s -- -y
        else
          print_error "Neither curl nor wget was found. Please install curl or wget first."
          exit 1
        fi

        # Source cargo environment for the current process
        if [[ -f "$HOME/.cargo/env" ]]; then
          # shellcheck source=/dev/null
          source "$HOME/.cargo/env"
        fi
        ;;
      *)
        print_error "Rust is required to build libraryCube. Please install Rust from https://rustup.rs/ and try again."
        exit 1
        ;;
    esac
  else
    print_error "Non-interactive environment and Rust is not installed."
    print_error "Please install Rust from https://rustup.rs/ before running install.sh."
    exit 1
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    print_error "Failed to locate cargo after rustup installation. Please restart your shell or check your PATH."
    exit 1
  fi
}

# Pre-flight check & build. Release archives contain a prebuilt binary and do
# not require Rust; source checkouts retain the existing build-first behavior.
if [[ "$DO_BUILD" -eq 1 ]] && [[ "$SOURCE_TREE" -eq 1 ]]; then
  ensure_rust
  print_info "Building libraryCube in release mode..."
  cargo build --locked --release
fi

if [[ -n "$ARCHIVE" ]]; then
  RELEASE_BIN="${ARCHIVE_TEMP}/lbc"
elif [[ "$SOURCE_TREE" -eq 1 ]] && [[ -f "${SCRIPT_DIR}/target/release/lbc" ]]; then
  RELEASE_BIN="${SCRIPT_DIR}/target/release/lbc"
else
  RELEASE_BIN="$BUNDLED_BIN"
fi
if [[ ! -f "$RELEASE_BIN" ]]; then
  print_error "Binary not found at ${RELEASE_BIN}."
  print_error "Run without '--no-build' in a source checkout, or use an official release archive."
  exit 1
fi

# Ensure destination directory exists
if [[ ! -d "$INSTALL_DIR" ]]; then
  print_info "Creating directory ${INSTALL_DIR}..."
  if mkdir -p "$INSTALL_DIR" 2>/dev/null; then
    :
  else
    if command -v sudo >/dev/null 2>&1; then
      sudo mkdir -p "$INSTALL_DIR"
    else
      print_error "Cannot create directory ${INSTALL_DIR}. Permission denied."
      exit 1
    fi
  fi
fi

# Install binary
print_info "Installing binary to ${INSTALL_DIR}/lbc..."
if [[ -w "$INSTALL_DIR" ]]; then
  if command -v install >/dev/null 2>&1; then
    install -m 755 "$RELEASE_BIN" "${INSTALL_DIR}/lbc"
  else
    cp "$RELEASE_BIN" "${INSTALL_DIR}/lbc"
    chmod 755 "${INSTALL_DIR}/lbc"
  fi
else
  if command -v sudo >/dev/null 2>&1; then
    if command -v install >/dev/null 2>&1; then
      sudo install -m 755 "$RELEASE_BIN" "${INSTALL_DIR}/lbc"
    else
      sudo cp "$RELEASE_BIN" "${INSTALL_DIR}/lbc"
      sudo chmod 755 "${INSTALL_DIR}/lbc"
    fi
  else
    print_error "Cannot write to ${INSTALL_DIR}. Please run with sudo or choose another directory."
    exit 1
  fi
fi

# Verify installation
if [[ ! -x "${INSTALL_DIR}/lbc" ]] ||
  ! INSTALLED_VERSION="$("${INSTALL_DIR}/lbc" --version 2>/dev/null)" ||
  [[ "$INSTALLED_VERSION" != lbc\ * ]]; then
  print_error "Installation verification failed for ${INSTALL_DIR}/lbc."
  exit 1
fi
print_success "Successfully installed ${INSTALLED_VERSION} to ${INSTALL_DIR}/lbc"

# Check PATH
in_path=0
case ":$PATH:" in
  *":${INSTALL_DIR}:"*)
    in_path=1
    ;;
esac

if [[ "$in_path" -eq 0 ]]; then
  echo ""
  print_warn "${INSTALL_DIR} is not currently in your PATH environment variable."
  echo "To use 'lbc' directly from any directory, add it to your shell configuration:"

  CURRENT_SHELL="$(basename "${SHELL:-bash}")"
  # Single quotes prevent spaces, dollars, and command substitutions in prefixes
  # from becoming shell syntax when the user copies the PATH command.
  QUOTED_DIR="'${INSTALL_DIR//\'/\'\\\'\'}'"
  case "$CURRENT_SHELL" in
    fish)
      FISH_DIR="${INSTALL_DIR//\\/\\\\}"
      FISH_DIR="'${FISH_DIR//\'/\\\'}'"
      printf '  fish_add_path -- %s\n' "$FISH_DIR"
      ;;
    *)
      printf '  export PATH=%s:"$PATH"\n' "$QUOTED_DIR"
      echo "For future terminals, add that line to ~/.zshrc (zsh) or ~/.bashrc (Bash), then open a new terminal."
      ;;
  esac
  echo ""
fi

echo "Quick start:"
echo "  lbc setup"
echo "  lbc learn"
echo '  lbc search "your problem"'
echo '  lbc explain build.log'
echo ""
