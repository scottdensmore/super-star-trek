# Design Specification: GitHub Actions CI/CD Matrix & Quality Gates

**Issue:** [#245: ci: configure GitHub Actions test and cross-compilation matrix](https://github.com/scottdensmore/super-star-trek/issues/245)  
**Date:** 2026-09-19  
**Status:** Approved by User  
**Target Branch:** `scottdensmore/feat/ci-actions-matrix`  

---

## 1. Overview & Goals

Super Star Trek has evolved from its 1978 BASIC origins into an ANSI C legacy engine with strict CTest/Golden test suites, alongside a modern Go Bubbletea TUI, WebAssembly browser deployment, and curated tactical scenarios.

To guarantee repository integrity, prevent regressions across both the classic C engine and modern Go/WASM codebases, and enforce strict automated merge criteria on all Pull Requests and merges to `main`, this design introduces a comprehensive GitHub Actions CI/CD workflow matrix.

### Objectives
1. **Parallel Modular Quality Gates:** Run Go test & race detection, static analysis/linting, WebAssembly compilation & Node.js execution, and legacy C CMake/CTest matrices concurrently.
2. **Multi-Platform Verification:** Test across Linux (`ubuntu-latest`) and macOS (`macos-latest`).
3. **Strict Warning & Linter Policies:** Enforce `SST_WERROR=ON` on C builds, `go vet` and `golangci-lint` on Go code, and `actionlint` + `shellcheck` on all workflow shell steps.
4. **Unified Branch Protection Gate:** Expose a single atomic status check (`ci-success`) so GitHub branch protection rules remain robust against future matrix adjustments.
5. **Self-Testing CI Validation:** Integrate seamlessly with the existing `tests/workflow.sh` CTest target (`workflow`), turning it from skipped to an actively passing check.

---

## 2. Architecture & Pipeline Topology

The pipeline is defined in a single GitHub Actions workflow file: `.github/workflows/ci.yml`.

```
                                  ┌────────────────────────┐
                                  │ Push to main / PR      │
                                  └───────────┬────────────┘
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    ▼                         ▼                         ▼
      ┌───────────────────────────┐    ┌─────────────┐    ┌───────────────────────────┐
      │          go-test          │    │golangci-lint│    │        wasm-verify        │
      │ ┌───────────────────────┐ │    │  (Linux)    │    │  (Linux: Go + Node.js)    │
      │ │ ubuntu-latest, Go1.26 │ │    └─────────────┘    └─────────────┬─────────────┘
      │ ├───────────────────────┤ │                                     │
      │ │ macos-latest, Go1.26  │ │                                     │
      │ └───────────────────────┘ │                                     │
      └─────────────┬─────────────┘                                     │
                    │                                                   │
                    └─────────────────────────┬─────────────────────────┘
                                              │
                                              ▼
                    ┌───────────────────────────────────────────────────┐
                    │                     c-matrix                      │
                    │ ┌───────────────────────┬───────────────────────┐ │
                    │ │ ubuntu / ci-debug     │ ubuntu / ci-release   │ │
                    │ ├───────────────────────┼───────────────────────┤ │
                    │ │ macos / ci-debug      │ macos / ci-release    │ │
                    │ └───────────────────────┴───────────────────────┘ │
                    └─────────────────────────┬─────────────────────────┘
                                              │
                                              ▼
                                    ┌───────────────────┐
                                    │    ci-success     │
                                    │ (Required Status) │
                                    └───────────────────┘
```

---

## 3. Workflow Specifications

### 3.1 Triggers, Concurrency, and Permissions
* **Name:** `CI`
* **Triggers:**
  ```yaml
  on:
    push:
      branches: [main]
    pull_request:
    workflow_dispatch:
  ```
* **Permissions:** `contents: read` (principle of least privilege).
* **Concurrency:**
  ```yaml
  concurrency:
    group: ci-${{ github.ref }}
    cancel-in-progress: true
  ```
  Cancels stale in-progress runs when commits are rapidly pushed to the same pull request branch.

---

### 3.2 Job 1: `go-test` (Go Test Suite & Race Detector)
* **Strategy Matrix:** `os: [ubuntu-latest, macos-latest]`
* **Timeout:** 10 minutes
* **Environment:** Go 1.26.x via `actions/setup-go@v5` with Go module caching enabled (`cache: true`).
* **Steps:**
  1. `actions/checkout@v4`
  2. Setup Go 1.26.x
  3. Static check: `go vet ./...`
  4. Test suite: `go test -v -race ./...` (enforces zero test failures and zero data races across all packages).

---

### 3.3 Job 2: `golangci-lint` (Go Static Analysis)
* **Runner:** `ubuntu-latest`
* **Timeout:** 5 minutes
* **Steps:**
  1. `actions/checkout@v4`
  2. Setup Go 1.26.x
  3. Run `golangci/golangci-lint-action@v6` with standard Go linters (`govet`, `errcheck`, `staticcheck`, `unused`, `gosimple`, `ineffassign`).

---

### 3.4 Job 3: `wasm-verify` (WebAssembly Compilation & Virtual Execution)
* **Runner:** `ubuntu-latest`
* **Timeout:** 5 minutes
* **Environment:** Go 1.26.x + Node.js 22.x (`actions/setup-node@v4`).
* **Steps:**
  1. `actions/checkout@v4`
  2. Setup Go and Node.js.
  3. Direct compilation check:
     ```bash
     GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm
     ```
  4. Script validation:
     Execute `scripts/build-wasm.sh` to ensure `web/sst.wasm` compiles with `-trimpath -ldflags="-s -w"` and that `wasm_exec.js` is copied from `$GOROOT`.
  5. WASM Virtual Machine Execution:
     Run `cmd/wasm` unit tests in the Node.js WebAssembly environment:
     ```bash
     GOOS=js GOARCH=wasm go test -v -exec="node $(go env GOROOT)/lib/wasm/wasm_exec_node.js" ./cmd/wasm
     ```

---

### 3.5 Job 4: `c-matrix` (Classic C Engine, Warnings-as-Errors, & Golden Master)
* **Strategy Matrix:**
  * `os: [ubuntu-latest, macos-latest]`
  * `preset: [ci-debug, ci-release]` (4 matrix legs)
  * `fail-fast: false`
* **Timeout:** 15 minutes
* **System Dependencies (with bounded retries and timeouts):**
  * **Linux (`ubuntu-latest`):**
    * System packages: `libncurses-dev`, `tmux`, `shellcheck` via `apt-get` with 3 bounded retries.
    * `actionlint`: Pinned v1.7.12 binary downloaded to `$RUNNER_TEMP` with SHA-256 integrity verification (`8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`) and installed to `/usr/local/bin`.
  * **macOS (`macos-latest`):**
    * System packages: `tmux` via Homebrew with 3 bounded retries (`HOMEBREW_NO_AUTO_UPDATE: 1`).
* **Build & Test Steps:**
  1. Configure: `cmake --preset ${{ matrix.preset }}`
  2. Build: `cmake --build --preset ${{ matrix.preset }}` (treats compiler warnings as errors via `SST_WERROR=ON`).
  3. CTest: `ctest --preset ${{ matrix.preset }}` (executes all 13 tests: `cmdtab`, `tuifmt`, `rules`, `journey`, `journey-tui`, `eof`, `help`, `tui` under tmux, `tournament`, `golden`, `analyze` via `gcc -fanalyzer`, `workflow` via `actionlint`+`shellcheck`, and `lineendings`).

---

### 3.6 Job 5: `ci-success` (Unified Branch Protection Gate)
* **Runner:** `ubuntu-latest`
* **Trigger condition:** `if: always()`
* **Dependencies:** `needs: [go-test, golangci-lint, wasm-verify, c-matrix]`
* **Behavior:**
  Inspects the result string of all dependencies. If any dependent job has a status other than `success`, the step logs `::error::` diagnostics and exits with non-zero status code `1`.
* **Benefit:** Allows configuring a single invariant required status check on GitHub (`ci-success`), shielding branch protection settings from matrix permutations.

---

## 4. Verification & Testing Strategy

1. **Syntax & Workflow Linting:**
   * Run `actionlint -no-color .github/workflows/ci.yml` locally with `shellcheck` on PATH.
   * Run `tests/workflow.sh .` and confirm `workflow OK (1 file(s))` output.
2. **Local CTest Target:**
   * Re-run `ctest --preset debug` locally and confirm that test #12 (`workflow`) runs and passes instead of skipping.
3. **Go & WASM Verification:**
   * Run `go test -v -race ./...`
   * Run `GOOS=js GOARCH=wasm go test -v -exec="node $(go env GOROOT)/lib/wasm/wasm_exec_node.js" ./cmd/wasm`
4. **Git & PR Verification:**
   * Commit workflow, push feature branch, open PR referencing [#245](https://github.com/scottdensmore/super-star-trek/issues/245), and verify that GitHub Actions triggers all 5 jobs and passes green.
