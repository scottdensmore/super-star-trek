#!/usr/bin/env bash
# Packages a built `sst` binary as a release archive:
#   <out-dir>/super-star-trek-<version>-<os>-<arch>.tar.gz   (macOS, Linux)
#   <out-dir>/super-star-trek-<version>-<os>-<arch>.zip      (Windows)
# plus a matching .sha256 file. The archive holds one directory of the same
# name with the binary, README.md, LICENSE and the original manual (sst.doc).
#
# Usage: scripts/package-release.sh <target-triple> <version> [out-dir]
#   The binary is read from target/<target-triple>/release/ (or target/release/
#   when <target-triple> is "host"); os/arch come from the triple.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

if [ "$#" -lt 2 ]; then
    echo "usage: $0 <target-triple|host> <version> [out-dir]" >&2
    exit 2
fi
TARGET="$1"
VERSION="$2"
OUT_DIR="${3:-${ROOT_DIR}/dist}"
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"

if [ "${TARGET}" = "host" ]; then
    TARGET="$(rustc -vV | awk '/^host: / { print $2 }')"
    BIN_DIR="${TARGET_DIR}/release"
else
    BIN_DIR="${TARGET_DIR}/${TARGET}/release"
fi

case "${TARGET}" in
    x86_64-*) ARCH=x86_64 ;;
    aarch64-*) ARCH=aarch64 ;;
    *) echo "error: unsupported architecture in ${TARGET}" >&2; exit 1 ;;
esac
case "${TARGET}" in
    *-apple-darwin) OS=macos; EXE="" ;;
    *-linux-*) OS=linux; EXE="" ;;
    *-windows-*) OS=windows; EXE=".exe" ;;
    *) echo "error: unsupported OS in ${TARGET}" >&2; exit 1 ;;
esac

BIN="${BIN_DIR}/sst${EXE}"
if [ ! -f "${BIN}" ]; then
    echo "error: ${BIN} not found; build it first with cargo build --release --locked -p super-star-trek" >&2
    exit 1
fi

NAME="super-star-trek-${VERSION}-${OS}-${ARCH}"
STAGE="$(mktemp -d)"
trap 'rm -rf "${STAGE}"' EXIT
mkdir -p "${STAGE}/${NAME}" "${OUT_DIR}"
cp "${BIN}" "${STAGE}/${NAME}/"
cp README.md LICENSE "${STAGE}/${NAME}/"
cp c/sst.doc "${STAGE}/${NAME}/"

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1"
    else
        shasum -a 256 "$1"
    fi
}

if [ "${OS}" = "windows" ]; then
    ARCHIVE="${NAME}.zip"
    rm -f "${OUT_DIR}/${ARCHIVE}"
    if command -v 7z >/dev/null 2>&1; then
        (cd "${STAGE}" && 7z a -tzip -bso0 -bsp0 "${ARCHIVE}" "${NAME}")
    else
        (cd "${STAGE}" && zip -q -r "${ARCHIVE}" "${NAME}")
    fi
else
    ARCHIVE="${NAME}.tar.gz"
    tar -czf "${STAGE}/${ARCHIVE}" -C "${STAGE}" "${NAME}"
fi
mv "${STAGE}/${ARCHIVE}" "${OUT_DIR}/"
(cd "${OUT_DIR}" && sha256 "${ARCHIVE}" > "${ARCHIVE}.sha256")

echo "${OUT_DIR}/${ARCHIVE}"
