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
  --uninstall        Remove installed lbc binary from target directory
  -h, --help         Show this help message and exit

Examples:
  ./install.sh                   # Install to ~/.local/bin
  ./install.sh --system          # Install system-wide to /usr/local/bin
  ./install.sh --prefix ~/bin    # Install to ~/bin
  ./install.sh --uninstall       # Remove lbc
EOF
}

# Resolve project root directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

INSTALL_DIR="${HOME}/.local/bin"
SYSTEM_INSTALL=0
DO_BUILD=1
DO_UNINSTALL=0

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

SOURCE_TREE=0
if [[ -f "Cargo.toml" ]] && grep -q 'name = "librarycube"' Cargo.toml 2>/dev/null; then
  SOURCE_TREE=1
fi
BUNDLED_BIN="${SCRIPT_DIR}/lbc"

if [[ "$DO_UNINSTALL" -eq 0 ]] && [[ "$SOURCE_TREE" -eq 0 ]] && [[ ! -f "$BUNDLED_BIN" ]]; then
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

if [[ "$SOURCE_TREE" -eq 1 ]] && [[ -f "${SCRIPT_DIR}/target/release/lbc" ]]; then
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
if [[ -x "${INSTALL_DIR}/lbc" ]]; then
  INSTALLED_VERSION="$("${INSTALL_DIR}/lbc" --version 2>/dev/null || echo "lbc")"
  print_success "Successfully installed ${INSTALLED_VERSION} to ${INSTALL_DIR}/lbc"
else
  print_error "Installation verification failed: ${INSTALL_DIR}/lbc is not executable."
  exit 1
fi

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
  case "$CURRENT_SHELL" in
    zsh)
      printf "  ${BOLD}echo 'export PATH=\"%s:\$PATH\"' >> ~/.zshrc && source ~/.zshrc${RESET}\n" "$INSTALL_DIR"
      ;;
    fish)
      printf "  ${BOLD}fish_add_path %s${RESET}\n" "$INSTALL_DIR"
      ;;
    *)
      printf "  ${BOLD}echo 'export PATH=\"%s:\$PATH\"' >> ~/.bashrc && source ~/.bashrc${RESET}\n" "$INSTALL_DIR"
      ;;
  esac
  echo ""
fi

echo "Quick start:"
echo "  lbc --help"
echo "  lbc add --title \"My First Note\" --stdin"
echo "  lbc ask \"How do I use libraryCube?\""
echo ""
