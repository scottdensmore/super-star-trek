# Official v2.0.0 Release Packaging & Multi-Platform Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an automated multi-platform distribution and release packaging pipeline for the official v2.0.0 milestone of Super Star Trek, producing cross-compiled native binaries, universal macOS binaries, Linux `.deb`/`.rpm` packages, Homebrew formula, WebAssembly bundles, and GitHub Actions release automation.

**Architecture:** 
- `cmd/sst`: Top-level version provenance variables (`version`, `commit`, `date`) and `--version`/`-v` flag support.
- `scripts/package-wasm.sh`: Standalone bundling script producing ready-to-deploy WebAssembly distribution archives (`.tar.gz` and `.zip`).
- `.goreleaser.yaml`: Declarative GoReleaser v2 specification defining cross-compilation builds, universal binaries, archives, NFPM Linux packages, and Homebrew tap formula.
- `.github/workflows/release.yml`: Automated CI/CD release workflow triggered on semantic version tags, publishing GitHub releases and deploying WebAssembly to GitHub Pages.
- `README.md`: Official installation documentation across Homebrew, apt/deb, rpm, precompiled binaries, and Go toolchain.

**Tech Stack:** Go 1.26.x, GoReleaser v2, NFPM, Homebrew, WebAssembly, GitHub Actions, Bubbletea.

**Spec:** [docs/superpowers/specs/2026-09-19-v2-release-packaging-design.md](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-19-v2-release-packaging-design.md)

## Global Constraints

- Target branch: `scottdensmore/feat/v2-release-packaging`
- Target Go version: `1.26.x`
- 100% standard library Go for core binary builds (`CGO_ENABLED=0`)
- Multi-platform targets: Linux (`amd64`, `arm64`), macOS (`amd64`, `arm64`, Universal), Windows (`amd64`)
- Packages: `.deb`, `.rpm`, `.tar.gz`, `.zip`, Homebrew tap formula
- WebAssembly: Bundled static distribution archives (`sst.wasm`, `wasm_exec.js`, `index.html`, `app.js`, `audio.js`, `style.css`)
- Zero test failures across Go race detector (`go test -v -race ./...`), CTest, golden tests, and WASM test runner
- Pass `golangci-lint run ./...` with zero issues

---

### Task 1: CLI Version Provenance & Flag Support (`cmd/sst`)

**Files:**
- Modify: `cmd/sst/main.go:1-60`
- Test: `cmd/sst/main_test.go`

**Interfaces:**
- Produces:
  - Top-level variables: `version string = "2.0.0-dev"`, `commit string = "none"`, `date string = "unknown"`
  - CLI flags: `--version` and `-v`
  - Output: `sst version 2.0.0 (commit: <commit>, built at: <date>)` exiting with status 0.

- [ ] **Step 1: Write failing tests in `cmd/sst/main_test.go`**

In `cmd/sst/main_test.go`, add `TestMain_VersionFlag`:
```go
func TestMain_VersionFlag(t *testing.T) {
	cases := []struct {
		name string
		flag string
	}{
		{"LongFlag", "--version"},
		{"ShortFlag", "-v"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			var out, errOut bytes.Buffer
			code := run([]string{tc.flag}, strings.NewReader(""), &out, &errOut)
			if code != 0 {
				t.Fatalf("expected exit code 0 for %s, got %d. stderr: %s", tc.flag, code, errOut.String())
			}
			stdout := out.String()
			if !strings.Contains(stdout, "sst version") {
				t.Errorf("expected 'sst version' in output, got: %q", stdout)
			}
			if !strings.Contains(stdout, "commit:") {
				t.Errorf("expected 'commit:' in output, got: %q", stdout)
			}
			if !strings.Contains(stdout, "built at:") {
				t.Errorf("expected 'built at:' in output, got: %q", stdout)
			}
		})
	}
}
```

- [ ] **Step 2: Run test to confirm failure (RED)**

Run: `go test -v ./cmd/sst -run TestMain_VersionFlag`  
Expected: FAIL (`flag provided but not defined: -version` or exit code != 0).

- [ ] **Step 3: Implement version variables and `--version`/`-v` in `cmd/sst/main.go`**

Define package variables:
```go
var (
	version = "2.0.0-dev"
	commit  = "none"
	date    = "unknown"
)
```

In `run(args []string, in io.Reader, out, errOut io.Writer) int`:
Add `--version` and `-v` flags to `fs`:
```go
	versionFlag := fs.Bool("version", false, "print version information and exit")
	fs.BoolVar(versionFlag, "v", false, "shorthand for --version")
```

Check right after `fs.Parse(args)`:
```go
	if *versionFlag {
		_, _ = fmt.Fprintf(out, "sst version %s (commit: %s, built at: %s)\n", version, commit, date)
		return 0
	}
```

- [ ] **Step 4: Run tests to confirm passing (GREEN)**

Run: `go test -v ./cmd/sst`  
Expected: PASS with 100% passing tests.

- [ ] **Step 5: Commit**

```bash
git add cmd/sst/main.go cmd/sst/main_test.go
git commit -m "feat(cli): add version provenance variables and --version flag"
```

---

### Task 2: WebAssembly Packaging Script & Standalone Bundles (`scripts/package-wasm.sh`)

**Files:**
- Create: `scripts/package-wasm.sh`
- Test: `scripts/package-wasm.sh` execution and verification test

**Interfaces:**
- Produces:
  - `dist/super-star-trek_${VERSION}_wasm.tar.gz`
  - `dist/super-star-trek_${VERSION}_wasm.zip`
  - Contains: `sst.wasm`, `wasm_exec.js`, `index.html`, `app.js`, `audio.js`, `style.css`

- [ ] **Step 1: Write `scripts/package-wasm.sh`**

Create `scripts/package-wasm.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-${VERSION:-2.0.0}}"
BUILD_DIR="${REPO_ROOT}/build/wasm"
DIST_DIR="${REPO_ROOT}/dist"

echo "=== Packaging WebAssembly Release Bundle (${VERSION}) ==="

rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}" "${DIST_DIR}"

# 1. Compile WASM binary
echo "Compiling cmd/wasm to ${BUILD_DIR}/sst.wasm..."
GOOS=js GOARCH=wasm go build -trimpath -ldflags="-s -w" -o "${BUILD_DIR}/sst.wasm" ./cmd/wasm

# 2. Copy Go WebAssembly JavaScript runtime support
GOROOT="$(go env GOROOT)"
cp "${GOROOT}/lib/wasm/wasm_exec.js" "${BUILD_DIR}/"

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
(cd "${BUILD_DIR}" && zip -q -r "${ZIP_FILE}" .)

echo "=== WebAssembly Bundle Created Successfully ==="
ls -lh "${TAR_FILE}" "${ZIP_FILE}"
```

Make executable:
```bash
chmod +x scripts/package-wasm.sh
```

- [ ] **Step 2: Test script execution and archive integrity**

Execute:
```bash
./scripts/package-wasm.sh 2.0.0
```
Verify:
```bash
tar -tzf dist/super-star-trek_2.0.0_wasm.tar.gz | grep sst.wasm
tar -tzf dist/super-star-trek_2.0.0_wasm.tar.gz | grep audio.js
unzip -l dist/super-star-trek_2.0.0_wasm.zip | grep sst.wasm
```
Expected: All files present and uncorrupted.

- [ ] **Step 3: Commit**

```bash
git add scripts/package-wasm.sh
git commit -m "feat(wasm): add automated WebAssembly standalone packaging script"
```

---

### Task 3: GoReleaser v2 Configuration & Local Snapshot Validation (`.goreleaser.yaml`, `Formula/super-star-trek.rb`)

**Files:**
- Create: `.goreleaser.yaml`
- Create: `Formula/super-star-trek.rb`

**Interfaces:**
- Produces:
  - Cross-compiled binaries: Linux (amd64, arm64), macOS (amd64, arm64, Universal), Windows (amd64)
  - NFPM packages: `.deb` and `.rpm`
  - Homebrew tap formula definition
  - Canonical `checksums.txt`

- [ ] **Step 1: Write `.goreleaser.yaml`**

Create `.goreleaser.yaml`:
```yaml
# yaml-language-server: $schema=https://goreleaser.com/static/schema.json
version: 2

project_name: super-star-trek

before:
  hooks:
    - go mod tidy

builds:
  - id: sst
    main: ./cmd/sst
    binary: sst
    env:
      - CGO_ENABLED=0
    goos:
      - linux
      - darwin
      - windows
    goarch:
      - amd64
      - arm64
    ignore:
      - goos: windows
        goarch: arm64
    flags:
      - -trimpath
    ldflags:
      - -s -w
      - -X main.version={{.Version}}
      - -X main.commit={{.Commit}}
      - -X main.date={{.Date}}

universal_binaries:
  - id: sst
    replace: false
    name_template: sst

archives:
  - id: default
    name_template: "{{ .ProjectName }}_{{ .Version }}_{{ .Os }}_{{ .Arch }}"
    format: tar.gz
    format_overrides:
      - goos: windows
        format: zip
    files:
      - README.md
      - LICENSE
      - c/sst.doc

nfpms:
  - id: packages
    package_name: super-star-trek
    file_name_template: "{{ .ProjectName }}_{{ .Version }}_{{ .Os }}_{{ .Arch }}"
    vendor: Scott Densmore
    homepage: https://github.com/scottdensmore/super-star-trek
    maintainer: Scott Densmore <scottdensmore@users.noreply.github.com>
    description: Authentic modernized Super Star Trek with Bubbletea TUI and retro audio
    license: MIT
    formats:
      - deb
      - rpm
    bindir: /usr/bin
    contents:
      - src: README.md
        dst: /usr/share/doc/super-star-trek/README.md
      - src: c/sst.doc
        dst: /usr/share/doc/super-star-trek/sst.doc

brews:
  - repository:
      owner: scottdensmore
      name: homebrew-tap
    directory: Formula
    homepage: https://github.com/scottdensmore/super-star-trek
    description: Authentic modernized Super Star Trek with Bubbletea TUI and retro audio
    license: MIT
    test: |
      system "#{bin}/sst", "--version"

checksum:
  name_template: "checksums.txt"
  algorithm: sha256

snapshot:
  version_template: "{{ incpatch .Version }}-snapshot"

changelog:
  sort: asc
  filters:
    exclude:
      - "^docs:"
      - "^test:"
      - "^chore:"
      - Merge pull request
      - Merge branch
```

- [ ] **Step 2: Create reference Homebrew formula template `Formula/super-star-trek.rb`**

Create `Formula/super-star-trek.rb`:
```ruby
# typed: false
# frozen_string_literal: true

class SuperStarTrek < Formula
  desc "Authentic modernized Super Star Trek with Bubbletea TUI and retro audio"
  homepage "https://github.com/scottdensmore/super-star-trek"
  version "2.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_darwin_amd64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    if Hardware::CPU.arm?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_darwin_arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_linux_amd64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/scottdensmore/super-star-trek/releases/download/v#{version}/super-star-trek_#{version}_linux_arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "sst"
    doc.install "README.md", "c/sst.doc" if File.exist?("c/sst.doc")
  end

  test do
    assert_match "sst version", shell_output("#{bin}/sst --version")
  end
end
```

- [ ] **Step 3: Run GoReleaser validation and snapshot build**

Install or use goreleaser:
```bash
mise use -g goreleaser@latest || go install github.com/goreleaser/goreleaser/v2@latest
goreleaser check
goreleaser release --snapshot --clean --skip=publish
```
Verify generated packages in `dist/`:
- `super-star-trek_*_linux_amd64.tar.gz`
- `super-star-trek_*_linux_arm64.tar.gz`
- `super-star-trek_*_darwin_amd64.tar.gz`
- `super-star-trek_*_darwin_arm64.tar.gz`
- `super-star-trek_*_windows_amd64.zip`
- `super-star-trek_*_linux_amd64.deb`
- `super-star-trek_*_linux_amd64.rpm`
- `checksums.txt`
- Check `dist/sst_linux_amd64_v1/sst --version` returns version output.

- [ ] **Step 4: Clean up snapshot artifacts and commit**

```bash
rm -rf dist/
git add .goreleaser.yaml Formula/super-star-trek.rb
git commit -m "feat(release): configure GoReleaser v2 multi-platform packaging and Homebrew formula"
```

---

### Task 4: GitHub Actions Release Workflow & Documentation (`.github/workflows/release.yml`, `README.md`)

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `README.md:35-85`

**Interfaces:**
- Produces:
  - Automated release pipeline triggered on `v*` tags
  - Automated GitHub Pages deployment
  - Comprehensive installation instructions in `README.md`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

Create `.github/workflows/release.yml`:
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:

permissions:
  contents: write
  pages: write
  id-token: write

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

jobs:
  release:
    name: Build & Publish Release
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version: '1.26.x'
          cache: true

      - name: Run verification tests
        run: |
          go vet ./...
          go test -race ./pkg/engine/... ./pkg/audio/... ./cmd/sst/...

      - name: Package WebAssembly Standalone Bundle
        run: |
          TAG_NAME="${{ github.ref_name }}"
          VERSION="${TAG_NAME#v}"
          if [ -z "$VERSION" ] || [ "$VERSION" = "main" ]; then
            VERSION="2.0.0"
          fi
          ./scripts/package-wasm.sh "$VERSION"

      - name: Run GoReleaser
        uses: goreleaser/goreleaser-action@v6
        with:
          distribution: goreleaser
          version: '~> v2'
          args: release --clean
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          HOMEBREW_TAP_GITHUB_TOKEN: ${{ secrets.HOMEBREW_TAP_GITHUB_TOKEN || secrets.GITHUB_TOKEN }}

      - name: Setup GitHub Pages
        uses: actions/configure-pages@v5

      - name: Upload Pages Artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: build/wasm

      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 2: Update `README.md` with multi-platform installation guides**

Add the comprehensive **Installation & Quick Start** section in `README.md`:
```markdown
## Installation

### Homebrew (macOS & Linux)
```bash
brew install scottdensmore/tap/super-star-trek
```

### Debian / Ubuntu (`.deb`)
Download the `.deb` package from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases) and install:
```bash
sudo dpkg -i super-star-trek_2.0.0_linux_amd64.deb
```

### Fedora / RHEL (`.rpm`)
Download the `.rpm` package from the [Releases page](https://github.com/scottdensmore/super-star-trek/releases) and install:
```bash
sudo rpm -i super-star-trek_2.0.0_linux_amd64.rpm
```

### Direct Binary Download
Pre-compiled standalone binaries for Linux (`amd64`, `arm64`), macOS (Universal Apple Silicon & Intel), and Windows (`amd64`) are available on the [Releases page](https://github.com/scottdensmore/super-star-trek/releases).

### Go Toolchain
```bash
go install github.com/scottdensmore/super-star-trek/cmd/sst@latest
```

### WebAssembly Browser Edition
Play instantly in your web browser with retro sound synthesis:
- **Live Online:** [https://scottdensmore.github.io/super-star-trek/](https://scottdensmore.github.io/super-star-trek/)
- **Self-Hosted:** Download `super-star-trek_2.0.0_wasm.tar.gz` from Releases and serve with any static web server (`python3 -m http.server 8080`).
```

- [ ] **Step 3: Run repository verification suites**

```bash
golangci-lint run ./...
go test -v -race ./...
```
Expected: PASS with 0 lint issues and 100% passing tests.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml README.md
git commit -m "feat(release): add GitHub Actions release workflow and documentation updates"
```

---

### Task 5: Push Branch, Open PR, and Verify GitHub Actions Matrix

**Files:**
- GitHub Pull Request targeting `main`

- [ ] **Step 1: Push feature branch to origin**

```bash
git checkout -b scottdensmore/feat/v2-release-packaging
git push -u origin scottdensmore/feat/v2-release-packaging
```

- [ ] **Step 2: Create Pull Request**

```bash
gh pr create --base main --head scottdensmore/feat/v2-release-packaging \
  --title "feat: official v2.0.0 release packaging and multi-platform distribution" \
  --body "Closes #244 by introducing the automated release packaging and multi-platform distribution pipeline..."
```

- [ ] **Step 3: Verify GitHub Actions CI Execution**

Run: `gh pr checks`  
Verify all CI matrix jobs pass green.
