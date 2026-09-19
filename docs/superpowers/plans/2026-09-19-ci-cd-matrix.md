# GitHub Actions CI/CD Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a comprehensive GitHub Actions CI/CD workflow matrix (.github/workflows/ci.yml) validating Go tests, race detection, static analysis, WebAssembly compilation/Node execution, legacy C engine builds with -Werror, and CTest integration with a unified status gate.

**Architecture:** A decoupled, high-concurrency pipeline in `.github/workflows/ci.yml` composed of 4 parallel validation jobs (`go-test`, `golangci-lint`, `wasm-verify`, `c-matrix`) feeding into an atomic branch protection gate job (`ci-success`). The workflow adheres to least privilege permissions (`contents: read`), dynamic concurrency cancellation, and strict linting via `actionlint` and `shellcheck`.

**Tech Stack:** GitHub Actions, Go 1.26.x, Node.js 22.x, CMake 3.21+, CTest, GCC/Clang (C17), ncurses, tmux, shellcheck, actionlint.

**Spec:** [docs/superpowers/specs/2026-09-19-ci-cd-matrix-design.md](file:///home/scottdensmore/Developer/scottdensmore/super-star-trek/docs/superpowers/specs/2026-09-19-ci-cd-matrix-design.md)

## Global Constraints

- Target branch: `scottdensmore/feat/ci-actions-matrix`
- Target Go version: `1.26.x`
- C build standard: `C17` with `-Werror` enforced via `SST_WERROR=ON`
- Single workflow file: `.github/workflows/ci.yml`
- Branch protection gate: `ci-success` job aggregating all matrix jobs
- Workflow linting: Must pass `actionlint` and `shellcheck` with zero warnings (`tests/workflow.sh`)
- Zero compiled binaries or artifacts committed to git tracking

---

### Task 1: Author GitHub Actions CI Workflow (`.github/workflows/ci.yml`)

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: GitHub Actions workflow `CI` with jobs `go-test`, `golangci-lint`, `wasm-verify`, `c-matrix`, and `ci-success`.

- [ ] **Step 1: Create workflow directory**

Create directory `.github/workflows`:
```bash
mkdir -p .github/workflows
```

- [ ] **Step 2: Write complete `.github/workflows/ci.yml`**

Write `.github/workflows/ci.yml` with exact syntax:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  go-test:
    name: go-test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    timeout-minutes: 10
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version: '1.26.x'
          cache: true

      - name: Run go vet
        run: go vet ./...

      - name: Run Go test suite with race detector
        run: go test -v -race ./...

  golangci-lint:
    name: golangci-lint
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version: '1.26.x'
          cache: true

      - name: Run golangci-lint
        uses: golangci/golangci-lint-action@v6
        with:
          version: v1.64.6

  wasm-verify:
    name: wasm-verify
    runs-on: ubuntu-latest
    timeout-minutes: 5
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version: '1.26.x'
          cache: true

      - name: Set up Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '22'

      - name: Verify WebAssembly compilation
        run: GOOS=js GOARCH=wasm go build -o /dev/null ./cmd/wasm

      - name: Verify build-wasm.sh script
        run: bash scripts/build-wasm.sh

      - name: Run WebAssembly tests via Node runner
        run: |
          goroot="$(go env GOROOT)"
          node_exec=""
          if [ -f "$goroot/lib/wasm/wasm_exec_node.js" ]; then
            node_exec="$goroot/lib/wasm/wasm_exec_node.js"
          elif [ -f "$goroot/misc/wasm/wasm_exec_node.js" ]; then
            node_exec="$goroot/misc/wasm/wasm_exec_node.js"
          fi
          if [ -z "$node_exec" ]; then
            echo "::error::wasm_exec_node.js not found in GOROOT"
            exit 1
          fi
          GOOS=js GOARCH=wasm go test -v -exec="node $node_exec" ./cmd/wasm

  c-matrix:
    name: c-matrix (${{ matrix.os }}, ${{ matrix.preset }})
    runs-on: ${{ matrix.os }}
    timeout-minutes: 15
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
        preset: [ci-debug, ci-release]
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Linux build dependencies, tmux and shellcheck
        if: runner.os == 'Linux'
        timeout-minutes: 7
        run: |
          for attempt in 1 2 3; do
            rc=0
            sudo timeout -k 10 60 apt-get update || rc=$?
            if [ "$rc" -eq 0 ]; then
              what="apt-get install"
              sudo timeout -k 10 120 apt-get install -y \
                --no-install-recommends libncurses-dev tmux \
                shellcheck || rc=$?
              if [ "$rc" -eq 0 ]; then
                exit 0
              fi
            else
              what="apt-get update"
            fi
            case $rc in
              124) why="timed out at its bound" ;;
              137) why="ignored the bound and was killed" ;;
              *)   why="failed with status $rc" ;;
            esac
            echo "::warning::attempt $attempt: $what $why" >&2
            sleep 5
          done
          echo "::error::could not install libncurses-dev, tmux and shellcheck" >&2
          exit 1

      - name: Install macOS build dependencies
        if: runner.os == 'macOS'
        timeout-minutes: 7
        env:
          HOMEBREW_NO_AUTO_UPDATE: 1
        run: |
          brew list tmux >/dev/null 2>&1 && exit 0
          for attempt in 1 2 3; do
            if brew install tmux; then
              exit 0
            fi
            echo "::warning::brew attempt $attempt failed" >&2
            sleep 5
          done
          echo "::error::could not install tmux" >&2
          exit 1

      - name: Install actionlint
        if: runner.os == 'Linux'
        timeout-minutes: 2
        env:
          ACTIONLINT_VERSION: '1.7.12'
          ACTIONLINT_SHA256: 8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8
        run: |
          cd "$RUNNER_TEMP"
          url=https://github.com/rhysd/actionlint/releases/download
          tgz=actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz
          for attempt in 1 2 3; do
            rc=0
            timeout -k 5 20 curl -sSLf -o "$tgz" \
              "$url/v${ACTIONLINT_VERSION}/$tgz" || rc=$?
            if [ "$rc" -eq 0 ]; then
              break
            fi
            case $rc in
              124) why="timed out at its bound" ;;
              137) why="ignored the bound and was killed" ;;
              *)   why="failed with status $rc" ;;
            esac
            echo "::warning::attempt $attempt: actionlint download $why" >&2
            sleep 3
          done
          if [ "$rc" -ne 0 ]; then
            echo "::error::could not download actionlint" >&2
            exit 1
          fi
          echo "${ACTIONLINT_SHA256}  $tgz" | sha256sum -c -
          tar xzf "$tgz" actionlint
          sudo install -m 0755 actionlint /usr/local/bin/actionlint
          actionlint --version

      - name: Configure CMake
        run: cmake --preset ${{ matrix.preset }}

      - name: Build C engine with -Werror
        run: cmake --build --preset ${{ matrix.preset }}

      - name: Run CTest test suite
        run: ctest --preset ${{ matrix.preset }}

  ci-success:
    name: ci-success
    runs-on: ubuntu-latest
    if: always()
    needs:
      - go-test
      - golangci-lint
      - wasm-verify
      - c-matrix
    steps:
      - name: Verify all matrix jobs succeeded
        run: |
          success=true
          for result in \
            "${{ needs.go-test.result }}" \
            "${{ needs.golangci-lint.result }}" \
            "${{ needs.wasm-verify.result }}" \
            "${{ needs.c-matrix.result }}"; do
            if [ "$result" != "success" ]; then
              echo "::error::Job dependency ended with status '$result'"
              success=false
            fi
          done
          if [ "$success" != "true" ]; then
            exit 1
          fi
          echo "All CI jobs passed successfully."
```

- [ ] **Step 3: Commit workflow file**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions CI workflow matrix and unified success gate"
```

---

### Task 2: Validate Workflow Locally via `tests/workflow.sh` and CTest

**Files:**
- Test: `.github/workflows/ci.yml`
- Test: `tests/workflow.sh`

**Interfaces:**
- Consumes: `.github/workflows/ci.yml`
- Validates: `actionlint`, `shellcheck`, CTest test #12 (`workflow`)

- [ ] **Step 1: Run workflow.sh lint harness**

Run:
```bash
PATH="$HOME/.local/share/mise/installs/go/1.26.6/bin:$PATH" tests/workflow.sh .
```
Expected: `workflow OK (1 file(s))` and exit status `0`.

- [ ] **Step 2: Run CTest debug suite**

Run:
```bash
PATH="$HOME/.local/share/mise/installs/go/1.26.6/bin:$PATH" ctest --preset debug -R workflow
```
Expected: Test #12 (`workflow`) status `Passed` (not `Skipped`).

- [ ] **Step 3: Run full local test battery**

Run:
```bash
go test -race ./...
GOOS=js GOARCH=wasm go test -v -exec="node $(go env GOROOT)/lib/wasm/wasm_exec_node.js" ./cmd/wasm
tests/golden.sh build/debug/sst
```
Expected: All suites report 100% PASS.

---

### Task 3: Push, Open PR, and Verify GitHub Actions Execution

**Files:**
- GitHub Pull Request targeting `main`

- [ ] **Step 1: Push feature branch to origin**

```bash
git push -u origin scottdensmore/feat/ci-actions-matrix
```

- [ ] **Step 2: Create Pull Request**

```bash
gh pr create --base main --head scottdensmore/feat/ci-actions-matrix \
  --title "ci: configure GitHub Actions test and cross-compilation matrix" \
  --body "Closes #245 by configuring a comprehensive GitHub Actions CI/CD matrix..."
```

- [ ] **Step 3: Monitor GitHub Actions run**

Run:
```bash
gh pr checks
```
Verify that `go-test`, `golangci-lint`, `wasm-verify`, `c-matrix`, and `ci-success` trigger, execute, and pass green.
