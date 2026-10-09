#!/bin/sh
# ==============================================================================
# Specter CLI - Modern Universal Web Installer (macOS & Linux)
# ==============================================================================
# Usage (Clean/Blank Machine):
#   curl -fsSL https://raw.githubusercontent.com/tuquet/cli/main/install.sh | sh
#   or: wget -qO- https://raw.githubusercontent.com/tuquet/cli/main/install.sh | sh
#
# Requirements: macOS (arm64/x64) or Linux (x64/arm64)
# ==============================================================================

set -e

# ANSI Color formatting
BOLD='\033[1m'
CYAN='\033[38;2;56;189;248m'
GREEN='\033[38;2;74;222;128m'
YELLOW='\033[38;2;251;191;36m'
RED='\033[38;2;248;113;113m'
RESET='\033[0m'
GRAY='\033[38;2;148;163;184m'

write_step() {
    printf "  ${CYAN}[+]${RESET} %s\n" "$1"
}

write_success() {
    printf "  ${GREEN}[OK]${RESET} %s\n" "$1"
}

write_notice() {
    printf "  ${YELLOW}[!]${RESET} %s\n" "$1"
}

write_error() {
    printf "  ${RED}[ERROR]${RESET} %s\n" "$1" >&2
}

printf "\n"
printf "${CYAN}   _____                 __            ${RESET}\n"
printf "${CYAN}  / ___/____  ___  _____/ /____  _____ ${RESET}\n"
printf "${CYAN}  \__ \/ __ \/ _ \/ ___/ __/ _ \/ ___/ ${RESET}\n"
printf "${CYAN} ___/ / /_/ /  __/ /__/ /_/  __/ /     ${BOLD}v1.0.0${RESET}\n"
printf "${CYAN}/____/ .___/\___/\___/\__/\___/_/      ${RESET}\n"
printf "${CYAN}    /_/                                ${RESET}\n"
printf "\n"
printf "  ${BOLD}Specter Unified Zero-Dependency Installer (Unix)${RESET}\n"
printf "  ${GRAY}Autonomous Browser Automation & Distributed Mesh Runtime${RESET}\n\n"

# 1. Detect Operating System and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        PLATFORM="macos"
        ;;
    Linux)
        PLATFORM="linux"
        ;;
    *)
        write_error "Unsupported operating system: $OS. Tuquet supports macOS, Linux, and Windows."
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x64"
        ;;
    arm64|aarch64)
        TARGET_ARCH="arm64"
        ;;
    *)
        write_error "Unsupported CPU architecture: $ARCH. Specter supports x86_64 and arm64."
        exit 1
        ;;
esac

RELEASE_NAME="specter-${PLATFORM}-${TARGET_ARCH}"
write_step "Detected environment: ${BOLD}${PLATFORM} (${TARGET_ARCH})${RESET}"

# 2. Determine Canonical SSOT Paths
SPECTER_DIR="${SPECTER_HOME:-${SPECTER_DIR:-${TUQUET_HOME:-${TUQUET_DIR:-${HOME}/.specter}}}}"
SPECTER_BIN_DIR="${SPECTER_DIR}/bin"
TARGET_EXE="${SPECTER_BIN_DIR}/specter"

# Auto-migration if ~/.tuquet exists and ~/.specter does not
if [ ! -e "${SPECTER_DIR}" ] && [ -d "${HOME}/.tuquet" ]; then
    ln -s "${HOME}/.tuquet" "${SPECTER_DIR}" 2>/dev/null || true
fi

write_step "Preparing canonical SSOT directory: ${SPECTER_BIN_DIR}"
mkdir -p "${SPECTER_BIN_DIR}"

# 3. Check for local build binary (if running from repo) or download from GitHub
LOCAL_BIN=""
if [ -f "./cli/target/x86_64-unknown-linux-musl/release/specter" ]; then
    LOCAL_BIN="./cli/target/x86_64-unknown-linux-musl/release/specter"
elif [ -f "./cli/target/release/specter" ]; then
    LOCAL_BIN="./cli/target/release/specter"
elif [ -f "./target/release/specter" ]; then
    LOCAL_BIN="./target/release/specter"
fi

if [ -n "$LOCAL_BIN" ]; then
    write_step "Installing local compiled binary from: ${LOCAL_BIN}"
    cp -f "${LOCAL_BIN}" "${TARGET_EXE}"
    chmod +x "${TARGET_EXE}"
    write_success "Installed specter to ${TARGET_EXE}"
else
    TARBALL_URL="https://github.com/tuquet/cli/releases/latest/download/${RELEASE_NAME}.tar.gz"
    FALLBACK_BIN_URL="https://github.com/tuquet/cli/releases/latest/download/${RELEASE_NAME}"

    write_step "Downloading ${RELEASE_NAME} from GitHub Releases..."

    DOWNLOADED=0
    TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'specter')"

    if command -v curl >/dev/null 2>&1; then
        if curl -fsSL "${TARBALL_URL}" -o "${TMP_DIR}/specter.tar.gz" 2>/dev/null; then
            tar -xzf "${TMP_DIR}/specter.tar.gz" -C "${TMP_DIR}"
            if [ -f "${TMP_DIR}/specter" ]; then
                mv -f "${TMP_DIR}/specter" "${TARGET_EXE}"
                DOWNLOADED=1
            fi
        elif curl -fsSL "${FALLBACK_BIN_URL}" -o "${TARGET_EXE}" 2>/dev/null; then
            DOWNLOADED=1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if wget -q "${TARBALL_URL}" -O "${TMP_DIR}/specter.tar.gz" 2>/dev/null; then
            tar -xzf "${TMP_DIR}/specter.tar.gz" -C "${TMP_DIR}"
            if [ -f "${TMP_DIR}/specter" ]; then
                mv -f "${TMP_DIR}/specter" "${TARGET_EXE}"
                DOWNLOADED=1
            fi
        elif wget -q "${FALLBACK_BIN_URL}" -O "${TARGET_EXE}" 2>/dev/null; then
            DOWNLOADED=1
        fi
    else
        write_error "Neither curl nor wget was found. Please install curl or wget to continue."
        rm -rf "${TMP_DIR}"
        exit 1
    fi

    rm -rf "${TMP_DIR}"

    if [ "$DOWNLOADED" -eq 1 ]; then
        chmod +x "${TARGET_EXE}"
        write_success "Downloaded and installed specter binary to ${TARGET_EXE}"
    else
        write_notice "Release asset ${RELEASE_NAME} not found on latest release. Checking existing local installation..."
        if [ ! -x "${TARGET_EXE}" ]; then
            write_error "Failed to acquire ${RELEASE_NAME}. Please build from source via 'cargo build --release'."
            exit 1
        fi
    fi
fi

# 4. Configure PATH in Shell Profile
write_step "Configuring PATH in shell profiles..."
PATH_LINE="export PATH=\"\$HOME/.specter/bin:\$PATH\""

SHELL_PROFILES=""
if [ -n "$BASH_VERSION" ] || [ -f "${HOME}/.bashrc" ]; then
    SHELL_PROFILES="${SHELL_PROFILES} ${HOME}/.bashrc"
fi
if [ -n "$ZSH_VERSION" ] || [ -f "${HOME}/.zshrc" ]; then
    SHELL_PROFILES="${SHELL_PROFILES} ${HOME}/.zshrc"
fi
if [ -f "${HOME}/.profile" ]; then
    SHELL_PROFILES="${SHELL_PROFILES} ${HOME}/.profile"
fi

if [ -z "$SHELL_PROFILES" ]; then
    SHELL_PROFILES="${HOME}/.profile"
fi

CONFIGURED_ANY=0
for PROFILE in $SHELL_PROFILES; do
    if [ -f "$PROFILE" ]; then
        if ! grep -q "\.specter/bin" "$PROFILE" 2>/dev/null && ! grep -q "\.tuquet/bin" "$PROFILE" 2>/dev/null; then
            printf "\n# Specter CLI\n%s\n" "${PATH_LINE}" >> "$PROFILE"
            write_success "Added Specter to PATH in ${PROFILE}"
            CONFIGURED_ANY=1
        fi
    fi
done

if [ "$CONFIGURED_ANY" -eq 0 ]; then
    write_success "PATH already configured in shell profile(s)"
fi

# 5. Run One-Command Bootstrap
printf "\n"
write_step "Triggering automatic ecosystem bootstrap (SSOT, DB, MCP, Chromium LTS)..."
printf "\n"

export PATH="${SPECTER_BIN_DIR}:${PATH}"
"${TARGET_EXE}" bootstrap || write_notice "Bootstrap completed with notices."

printf "\n"
printf "${GREEN}==============================================================================${RESET}\n"
printf "${GREEN}  SPECTER CLI INSTALLATION COMPLETED SUCCESSFULLY${RESET}\n"
printf "${GREEN}==============================================================================${RESET}\n\n"
printf "  To get started, reload your shell or run immediately:\n"
printf "    ${CYAN}specter browser launch${RESET}  Launch Antidetect Chromium browser sandbox\n"
printf "    ${CYAN}specter doctor${RESET}          Check environment and system shims\n"
printf "    ${CYAN}specter status${RESET}          Inspect unified services dashboard\n"
printf "    ${CYAN}specter bridge start${RESET}    Start secure SOCKS5 mesh tunnel\n\n"
printf "  ${GRAY}Single Source of Truth Root: ${SPECTER_DIR}${RESET}\n\n"
