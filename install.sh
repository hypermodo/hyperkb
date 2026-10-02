#!/usr/bin/env bash
set -euo pipefail

# HyperKB Universal One-Line Installer (Tier 3)
# Usage: curl -fsSL https://raw.githubusercontent.com/hypermodo/hyperkb/main/install.sh | sh

REPO="hypermodo/hyperkb"
VERSION="${HYPERKB_VERSION:-latest}"

echo "==> Detecting host environment..."

OS="$(uname -s)"
case "${OS}" in
  Darwin*)  PLATFORM="apple-darwin" ;;
  Linux*)   PLATFORM="unknown-linux-musl" ;;
  *)        echo "Error: Unsupported operating system: ${OS}"; exit 1 ;;
esac

ARCH="$(uname -m)"
case "${ARCH}" in
  x86_64*|amd64*) ARCH_TARGET="x86_64" ;;
  arm64*|aarch64*) ARCH_TARGET="aarch64" ;;
  *)               echo "Error: Unsupported CPU architecture: ${ARCH}"; exit 1 ;;
esac

TARGET="${ARCH_TARGET}-${PLATFORM}"
echo "==> Detected target: ${TARGET}"

if [ "${VERSION}" = "latest" ]; then
  RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
  echo "==> Fetching latest release information..."
  TAG=$(curl -sSfL "${RELEASE_URL}" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
  if [ -z "${TAG}" ]; then
    echo "Error: Unable to determine latest release tag from GitHub."
    exit 1
  fi
else
  TAG="${VERSION}"
fi

ARCHIVE_NAME="hyperkb-${TAG}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE_NAME}"

INSTALL_DIR="${HYPERKB_INSTALL_DIR:-${HOME}/.local/bin}"
mkdir -p "${INSTALL_DIR}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "==> Downloading HyperKB ${TAG} for ${TARGET}..."
curl -sSfL "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ARCHIVE_NAME}"

echo "==> Unpacking to ${INSTALL_DIR}..."
tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "${TMP_DIR}"
mv "${TMP_DIR}/hyperkb" "${INSTALL_DIR}/hyperkb"
chmod +x "${INSTALL_DIR}/hyperkb"

echo "==> Successfully installed HyperKB ${TAG} to ${INSTALL_DIR}/hyperkb"

# Check if INSTALL_DIR is in PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    echo "Notice: ${INSTALL_DIR} is not in your \$PATH."
    echo "Add the following line to your shell profile (~/.zshrc or ~/.bashrc):"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
esac

echo ""
echo "Verify installation by running:"
echo "  hyperkb --help"
