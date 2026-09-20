#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

VERSION="${1:-${VERSION:-2.0.0}}"
BUILD_DIR="${REPO_ROOT}/build/wasm"
DIST_DIR="${2:-${DIST_DIR:-${REPO_ROOT}/build/dist}}"

echo "=== Packaging WebAssembly Release Bundle (${VERSION}) ==="

rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}" "${DIST_DIR}"

# 1. Compile WASM binary
echo "Compiling cmd/wasm to ${BUILD_DIR}/sst.wasm..."
GOOS=js GOARCH=wasm go build -trimpath -ldflags="-s -w" -o "${BUILD_DIR}/sst.wasm" ./cmd/wasm

# 2. Copy Go WebAssembly JavaScript runtime support
GOROOT="$(go env GOROOT)"
if [ -f "${GOROOT}/lib/wasm/wasm_exec.js" ]; then
	cp "${GOROOT}/lib/wasm/wasm_exec.js" "${BUILD_DIR}/"
elif [ -f "${GOROOT}/misc/wasm/wasm_exec.js" ]; then
	cp "${GOROOT}/misc/wasm/wasm_exec.js" "${BUILD_DIR}/"
else
	echo "::error::wasm_exec.js not found in GOROOT" >&2
	exit 1
fi

# 3. Copy static frontend assets
cp "${REPO_ROOT}/web/index.html" "${BUILD_DIR}/"
cp "${REPO_ROOT}/web/app.js" "${BUILD_DIR}/"
cp "${REPO_ROOT}/web/audio.js" "${BUILD_DIR}/"
cp "${REPO_ROOT}/web/style.css" "${BUILD_DIR}/"

# 4. Generate distribution archives
ARCHIVE_BASE="super-star-trek_${VERSION}_wasm"
TAR_FILE="${DIST_DIR}/${ARCHIVE_BASE}.tar.gz"
ZIP_FILE="${DIST_DIR}/${ARCHIVE_BASE}.zip"

echo "Creating ${TAR_FILE}..."
tar -czf "${TAR_FILE}" -C "${BUILD_DIR}" .

echo "Creating ${ZIP_FILE}..."
rm -f "${ZIP_FILE}"
(cd "${BUILD_DIR}" && zip -q -r "${ZIP_FILE}" .)

echo "=== WebAssembly Bundle Created Successfully ==="
ls -lh "${TAR_FILE}" "${ZIP_FILE}"
