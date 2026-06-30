#!/usr/bin/env bash
set -euo pipefail

REPO="harunjeylan/ocean"
APP="ocean"
INSTALL_DIR="${HOME}/.${APP}/bin"

if [ "${1:-}" = "latest" ] || [ -z "${1:-}" ]; then
    VERSION="latest"
    RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
else
    VERSION="$1"
    RELEASE_URL="https://api.github.com/repos/${REPO}/releases/tags/${VERSION}"
fi

echo "Installing ${APP}..."

# Detect OS and architecture
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "${OS}" in
    linux)
        case "${ARCH}" in
            x86_64|amd64)  TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64|arm64)  echo "ARM64 Linux is not yet supported via pre-built binaries."; exit 1 ;;
            *)              echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
        esac
        ;;
    darwin)
        case "${ARCH}" in
            arm64|aarch64)  TARGET="aarch64-apple-darwin" ;;
            x86_64)         echo "x86_64 macOS is not yet supported via pre-built binaries."; exit 1 ;;
            *)              echo "Unsupported architecture: ${ARCH}"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: ${OS}"
        exit 1
        ;;
esac

echo "  OS:      ${OS}"
echo "  Arch:    ${ARCH}"
echo "  Target:  ${TARGET}"
echo "  Version: ${VERSION}"

# Fetch release info
echo ""
echo "Fetching release info..."
if ! command -v curl &>/dev/null; then
    echo "curl is required but not found."
    exit 1
fi

RELEASE_JSON="$(curl -sfL "${RELEASE_URL}")"
TAG="$(echo "${RELEASE_JSON}" | grep -m1 '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
ASSET_NAME="ocean-${TAG}-${TARGET}.tar.gz"

if [ -z "${TAG}" ]; then
    echo "Failed to determine latest release tag."
    exit 1
fi

DOWNLOAD_URL="$(echo "${RELEASE_JSON}" | grep -oP '"browser_download_url": *"\K[^"]*'"${ASSET_NAME}"'"' | sed 's/"$//')"

if [ -z "${DOWNLOAD_URL}" ]; then
    echo "No pre-built binary found for '${ASSET_NAME}'."
    echo "Available assets:"
    echo "${RELEASE_JSON}" | grep -oP '"name": *"\K[^"]*' || true
    exit 1
fi

echo "  Asset:   ${ASSET_NAME}"
echo "  URL:     ${DOWNLOAD_URL}"

# Download
echo ""
echo "Downloading..."
TMP_DIR="$(mktemp -d)"
TAR_PATH="${TMP_DIR}/${ASSET_NAME}"
curl -sfL "${DOWNLOAD_URL}" -o "${TAR_PATH}"

# Install
mkdir -p "${INSTALL_DIR}"
echo ""
echo "Extracting to ${INSTALL_DIR}..."
tar -xzf "${TAR_PATH}" -C "${INSTALL_DIR}"
rm -rf "${TMP_DIR}"

BINARY="${INSTALL_DIR}/${APP}"
if [ ! -f "${BINARY}" ]; then
    echo "Binary not found at '${BINARY}' after extraction."
    exit 1
fi
chmod +x "${BINARY}"

# Add to PATH via shell rc
SHELL_RC=""
if [ -n "${BASH_VERSION:-}" ] && [ -f "${HOME}/.bashrc" ]; then
    SHELL_RC="${HOME}/.bashrc"
elif [ -n "${ZSH_VERSION:-}" ] && [ -f "${HOME}/.zshrc" ]; then
    SHELL_RC="${HOME}/.zshrc"
elif [ -f "${HOME}/.bashrc" ]; then
    SHELL_RC="${HOME}/.bashrc"
elif [ -f "${HOME}/.zshrc" ]; then
    SHELL_RC="${HOME}/.zshrc"
elif [ -f "${HOME}/.profile" ]; then
    SHELL_RC="${HOME}/.profile"
fi

PATH_LINE="export PATH=\"\${HOME}/.${APP}/bin:\${PATH}\""
if [ -n "${SHELL_RC}" ]; then
    if grep -qF "${HOME}/.${APP}/bin" "${SHELL_RC}" 2>/dev/null; then
        echo "${APP} bin directory is already in PATH (${SHELL_RC})."
    else
        echo "" >> "${SHELL_RC}"
        echo "# ${APP}" >> "${SHELL_RC}"
        echo "${PATH_LINE}" >> "${SHELL_RC}"
        echo "Added '~/.${APP}/bin' to PATH in ${SHELL_RC}."
    fi
else
    echo ""
    echo "Could not detect shell rc file. Add this to your shell profile:"
    echo "  ${PATH_LINE}"
fi

echo ""
echo "${APP} ${TAG} installed successfully!"
echo "  Binary: ${BINARY}"
echo ""
echo "You may need to restart your terminal or run:"
echo "  export PATH=\"\${HOME}/.${APP}/bin:\${PATH}\""
echo ""
echo "Next steps:"
echo "  1. Run: ${APP} --help"
echo "  2. cd to a project directory and run: ${APP} init"
