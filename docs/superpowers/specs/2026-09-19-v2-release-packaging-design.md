# Official v2.0.0 Release Packaging & Multi-Platform Distribution Design

## 1. Overview & Goals

This specification defines the multi-platform packaging, distribution, and release automation pipeline for the official **v2.0.0** milestone of Super Star Trek (Issue #244).

With the complete modernization of Super Star Trek into a modern Go Bubbletea TUI, WebAssembly browser game, retro procedural audio synthesizer, and validated 1978 C parity, this pipeline automates the generation and publication of production release artifacts across Linux, macOS, Windows, WebAssembly, and package managers.

### Key Objectives
1. **Multi-Platform Cross-Compilation:** Native binaries for Linux (`amd64`, `arm64`), macOS (`amd64`, `arm64`, Universal), and Windows (`amd64`).
2. **Standard Package Formats:** Compressed tarballs (`.tar.gz`), zip archives (`.zip`), Linux packages (`.deb`, `.rpm`), and Homebrew tap formula.
3. **WebAssembly Distribution:** Standalone downloadable static web bundle and live GitHub Pages deployment.
4. **Build Provenance & Version Inspection:** Linker flag injection (`version`, `commit`, `date`) and `--version` / `-v` CLI flag support in `cmd/sst`.
5. **Declarative Automation:** GoReleaser v2 configuration (`.goreleaser.yaml`) orchestrated via GitHub Actions (`.github/workflows/release.yml`).

---

## 2. CLI Version Provenance (`cmd/sst`)

### 2.1 Build-Time Variables
In `cmd/sst/main.go`, define package variables injected at link time:
```go
var (
    version = "2.0.0-dev"
    commit  = "none"
    date    = "unknown"
)
```

### 2.2 CLI Flags
- Flags: `--version` and `-v`.
- Format when invoked:
  ```text
  sst version 2.0.0 (commit: abc1234, built at: 2026-09-19T20:00:00Z)
  ```
- Exits with status code `0`.
- Documented in `sst --help` usage output.

---

## 3. GoReleaser Pipeline (`.goreleaser.yaml`)

Configuration adhering to GoReleaser v2 schema:

### 3.1 Binary Compilation (`builds`)
- **ID:** `sst`
- **Main:** `./cmd/sst`
- **Binary:** `sst` (appends `.exe` automatically for Windows)
- **Environment:** `CGO_ENABLED=0`
- **GoOS:** `[linux, darwin, windows]`
- **GoArch:** `[amd64, arm64]`
- **Ignore:** `{goos: windows, goarch: arm64}`
- **Flags:** `-trimpath`
- **Ldflags:**
  `-s -w -X main.version={{.Version}} -X main.commit={{.Commit}} -X main.date={{.Date}}`

### 3.2 macOS Universal Binaries (`universal_binaries`)
- **ID:** `sst`
- **Name Template:** `sst`
- **Replace:** `false` (creates combined universal binary for Apple Silicon and Intel Macs).

### 3.3 Archives (`archives`)
- **Name Template:** `super-star-trek_{{ .Version }}_{{ .Os }}_{{ .Arch }}`
- **Formats:**
  - `tar.gz` for Linux and macOS.
  - `zip` for Windows.
- **Included Files:**
  - `README.md`
  - `LICENSE`
  - `c/sst.doc` (classic 1978 instructions)

### 3.4 Linux Packages (`nfpms`)
- **ID:** `packages`
- **Package Name:** `super-star-trek`
- **Vendor:** `Scott Densmore`
- **Homepage:** `https://github.com/scottdensmore/super-star-trek`
- **Maintainer:** `Scott Densmore <scottdensmore@users.noreply.github.com>`
- **Description:** `Authentic modernized Super Star Trek with Bubbletea TUI and retro audio`
- **License:** `MIT`
- **Formats:** `[deb, rpm]`
- **Destination:** Binary installed to `/usr/bin/sst`.
- **Documentation:** Installed to `/usr/share/doc/super-star-trek/`.

### 3.5 Homebrew Tap (`brews`)
- **Target Repository:** `scottdensmore/homebrew-tap`
- **Formula Name:** `super-star-trek.rb`
- **Homepage:** `https://github.com/scottdensmore/super-star-trek`
- **Description:** `Authentic modernized Super Star Trek with Bubbletea TUI and retro audio`
- **License:** `MIT`
- **Test:** `system "#{bin}/sst", "--version"`
- **Local Template:** Checked-in copy at `Formula/super-star-trek.rb`.

### 3.6 Extra Release Files & Checksums
- **Extra Files:** `dist/super-star-trek_*.tar.gz` and `.zip` for WebAssembly.
- **Checksum:** Canonical `checksums.txt` generated using SHA-256.

---

## 4. WebAssembly Packaging & GitHub Pages

### 4.1 Packaging Script (`scripts/package-wasm.sh`)
An idempotent bash script that:
1. Compiles WebAssembly binary:
   ```bash
   GOOS=js GOARCH=wasm go build -trimpath -ldflags="-s -w" -o build/wasm/sst.wasm ./cmd/wasm
   ```
2. Copies Go WASM runtime support:
   ```bash
   cp "$(go env GOROOT)/lib/wasm/wasm_exec.js" build/wasm/
   ```
3. Copies static web assets from `web/`:
   ```bash
   cp web/index.html web/app.js web/audio.js web/style.css build/wasm/
   ```
4. Creates standalone distribution archives in `dist/`:
   - `super-star-trek_${VERSION}_wasm.tar.gz`
   - `super-star-trek_${VERSION}_wasm.zip`

### 4.2 GitHub Pages Deployment
In `.github/workflows/release.yml`, upload the `build/wasm` directory as a Pages artifact and deploy via `actions/deploy-pages@v4` on tag releases.

---

## 5. GitHub Actions Release Workflow (`.github/workflows/release.yml`)

- **Triggers:**
  - `push: tags: ['v*']`
  - `workflow_dispatch` (with inputs for testing dry-runs/snapshots).
- **Permissions:**
  - `contents: write` (release creation and tag push)
  - `pages: write`
  - `id-token: write`
- **Job Sequence:**
  1. Checkout with `fetch-depth: 0`.
  2. Setup Go `1.26.x`.
  3. Run tests and linter before release (`go test -race ./...`, `golangci-lint run ./...`).
  4. Run `scripts/package-wasm.sh` to produce WASM bundles.
  5. Run GoReleaser via `goreleaser/goreleaser-action@v6`.
  6. Deploy WASM directory to GitHub Pages.

---

## 6. Documentation & Quick Start Updates

Update `README.md` with official installation instructions across all target environments:
- **Homebrew:**
  ```bash
  brew install scottdensmore/tap/super-star-trek
  ```
- **Debian / Ubuntu:**
  ```bash
  sudo dpkg -i super-star-trek_2.0.0_linux_amd64.deb
  ```
- **Fedora / RHEL:**
  ```bash
  sudo rpm -i super-star-trek_2.0.0_linux_amd64.rpm
  ```
- **Precompiled Tarballs / Zip:**
  Download from GitHub Releases for Linux, macOS Universal, or Windows.
- **Go Toolchain:**
  ```bash
  go install github.com/scottdensmore/super-star-trek/cmd/sst@v2.0.0
  ```
- **WebAssembly Browser Edition:**
  Live playable link hosted on GitHub Pages or self-hosted via `super-star-trek_2.0.0_wasm.tar.gz`.

---

## 7. Testing & Verification Plan

1. **Unit Tests:**
   - Verify `cmd/sst/main_test.go` tests `--version` and `-v` outputs and exit code 0.
2. **GoReleaser Dry-Run / Snapshot:**
   - Validate `.goreleaser.yaml` using `goreleaser check`.
   - Run snapshot build: `goreleaser release --snapshot --clean --skip=publish`.
   - Verify all binaries compile, universal macOS binary is generated, `.deb` and `.rpm` are created, and checksums are computed.
3. **WebAssembly Packaging Script:**
   - Run `scripts/package-wasm.sh` and verify archive contents and uncompressed integrity.
4. **CI Workflow Linting:**
   - Verify `.github/workflows/release.yml` syntax.
