# C Codebase Directory Restructuring Design

**Status:** Approved  
**Date:** 2026-09-19  
**Authors:** Super Star Trek Maintainers  
**Target:** `c/`, `tests/`, `CMakeLists.txt`, `CMakePresets.json`, `.gitattributes`, `README.md`

---

## 1. Executive Summary

This design restructures the repository to cleanly isolate the classic 1978 C implementation into its own dedicated directory tree (`c/`), simplifying and decluttering the repository root. All C translation units, headers, manuals, legacy build artifacts, C test suites, and classic golden fixtures move into `c/`. 

The repository root is preserved for the primary modern Go implementation (`cmd/`, `pkg/`, `web/`), project documentation, and CI workflows. A lightweight root `CMakeLists.txt` delegates to `c/`, ensuring developer workflows (`cmake --preset <name>`, `ctest --preset <name>`) and GitHub Actions matrix jobs continue working without modification.

---

## 2. Directory Layout & Architecture

### 2.1 Repository Structure

```
super-star-trek/
├── c/
│   ├── ai.c, battle.c, cmdtab.c, events.c, finish.c, moving.c, osx.c
│   ├── planets.c, reports.c, rules.c, setup.c, sst.c, tui.c, tuifmt.c
│   ├── cmdtab.h, finish.h, osx.h, rules.h, sst.h, tui.h
│   ├── sst.doc                     # Original game manual read by sst at runtime
│   ├── makefile                    # Standalone legacy Makefile
│   ├── CMakeLists.txt              # C engine targets, tests & packaging
│   └── tests/
│       ├── test_cmdtab.c, test_rules.c, test_tuifmt.c
│       ├── analyze.sh, eof.sh, golden.sh, help.sh, journey.sh,
│       │   lineendings.sh, tournament.sh, tui.sh, workflow.sh
│       └── golden/
│           ├── combat.txt, docking.txt, ending.txt, freeze.txt,
│           │   reports.txt, thaw.txt, torpedoes.txt, won.txt
├── cmd/                            # Go CLI and WASM entrypoints
├── pkg/                            # Go packages (audio, classic, engine, tui)
├── web/                            # Browser WASM & retro terminal web app
├── tests/                          # Root test suite for Go
│   ├── golden_test.go
│   ├── tui_golden_test.go
│   └── golden/
│       └── tui/                    # 24 Go TUI golden snapshot files
├── CMakeLists.txt                  # Root wrapper delegating to c/
├── CMakePresets.json               # Root presets for CI & IDEs
└── README.md, go.mod, mise.toml
```

### 2.2 Relocation Manifest
All files must be relocated using `git mv` to preserve commit history:

1. **Root to `c/`**:
   - `ai.c`, `battle.c`, `cmdtab.c`, `events.c`, `finish.c`, `moving.c`, `osx.c`, `planets.c`, `reports.c`, `rules.c`, `setup.c`, `sst.c`, `tui.c`, `tuifmt.c`
   - `cmdtab.h`, `finish.h`, `osx.h`, `rules.h`, `sst.h`, `tui.h`
   - `sst.doc`, `makefile`
2. **`tests/` to `c/tests/`**:
   - Unit tests: `test_cmdtab.c`, `test_rules.c`, `test_tuifmt.c`
   - Shell scripts: `analyze.sh`, `eof.sh`, `golden.sh`, `help.sh`, `journey.sh`, `lineendings.sh`, `tournament.sh`, `tui.sh`, `workflow.sh`
   - Golden text files: `tests/golden/*.txt` (moved to `c/tests/golden/`)
3. **Retained in `tests/`**:
   - `golden_test.go`
   - `tui_golden_test.go`
   - `tests/golden/tui/*.golden` (24 golden files for TUI)

---

## 3. Build System Specification

### 3.1 Root `CMakeLists.txt` Wrapper
A clean wrapper at the top level:
```cmake
cmake_minimum_required(VERSION 3.21)
project(super-star-trek
        LANGUAGES C
        VERSION 1.0)

enable_testing()
add_subdirectory(c)
```

### 3.2 `c/CMakeLists.txt` Target Definitions
The existing build configuration moves into `c/CMakeLists.txt` with path updates:
* Target `sst` sources: `sst.c ai.c battle.c cmdtab.c events.c finish.c moving.c osx.c planets.c reports.c rules.c setup.c tui.c tuifmt.c`.
* Include directories: `${CMAKE_CURRENT_SOURCE_DIR}` (`c/`).
* Test targets:
  * `add_executable(test_cmdtab tests/test_cmdtab.c cmdtab.c)`
  * `add_executable(test_tuifmt tests/test_tuifmt.c tuifmt.c)`
  * `add_executable(test_rules tests/test_rules.c rules.c)`
* Test script execution:
  * All test commands updated to `${CMAKE_CURRENT_SOURCE_DIR}/tests/<script>.sh`.
  * `WORKING_DIRECTORY` set to `${CMAKE_CURRENT_SOURCE_DIR}` for `journey`, `journey-tui`, and `tui`, guaranteeing `sst.doc` co-location.
  * `workflow.sh` receives `${CMAKE_SOURCE_DIR}` (root of repository) to validate `.github/workflows/`.
  * `lineendings.sh` receives `${CMAKE_SOURCE_DIR}` (root of repository) to validate the work tree.

### 3.3 CMake Presets
`CMakePresets.json` remains at the repository root. The binary directory `${sourceDir}/build/${presetName}` remains unchanged, outputting the `sst` executable to `build/${presetName}/c/sst`.

### 3.4 Standalone `c/makefile`
The standalone Makefile in `c/` is updated to include `cmdtab.o`, `rules.o`, `tui.o`, and `tuifmt.o` alongside existing objects:
```make
CFLAGS=     -O -DSCORE -DCAPTURE -DCLOAKING

.c.o:
	$(CC) $(CFLAGS) -c $<

OFILES=     sst.o finish.o reports.o setup.o osx.o moving.o battle.o events.o ai.o planets.o cmdtab.o rules.o tui.o tuifmt.o
HFILES=     sst.h cmdtab.h finish.h osx.h rules.h tui.h

sst:  $(OFILES)
	$(CC) -o sst $(OFILES) -lncurses -lm

clean:
	rm -f $(OFILES) sst

$(OFILES):  $(HFILES)
```

---

## 4. Test Harness & Environment Adaptation

### 4.1 Script Working Directory & Paths
All shell test scripts locate their assets relative to their parent directory:
```sh
srcdir=$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)
```
When running from `c/tests/`, `$srcdir` resolves directly to `c/`:
* `golden.sh` locates `c/tests/golden/*.txt`.
* `help.sh` verifies `c/sst.doc`.
* `analyze.sh` receives the source list for `sst` and resolves non-absolute paths against `c/`.

### 4.2 Line Endings Enforcement (`c/tests/lineendings.sh`)
Update file lists in `c/tests/lineendings.sh` to match the relocated paths:
```sh
crlf_files='c/ai.c
c/battle.c
c/events.c
c/finish.c
c/moving.c
c/planets.c
c/reports.c
c/setup.c
c/sst.c
c/sst.doc
c/sst.h'

required_seen='c/sst.c
c/sst.h
c/sst.doc
c/tui.c
c/tests/lineendings.sh'
```

### 4.3 Git Attributes (`.gitattributes`)
Update `.gitattributes` to explicitly freeze vintage CRLF and LF files in their new locations:
```gitattributes
# C test golden fixtures
c/tests/golden/*.txt -text

# Vintage C files (CRLF) and newer C files (LF)
c/*.c -text
c/*.h -text
c/tests/*.sh -text
c/sst.doc -text

# Other shell scripts
*.sh -text
```

---

## 5. Continuous Integration & Documentation

### 5.1 CI Workflow (`.github/workflows/ci.yml`)
Because `CMakePresets.json` and the root `CMakeLists.txt` remain at repository root, all CI jobs (`go-test`, `golangci-lint`, `wasm-verify`, `c-matrix`) continue without configuration changes.

### 5.2 Documentation (`README.md`)
Update the `## Classic C Edition` section:
* Note that C source code and documentation reside in `c/`.
* Update executable path to `./build/debug/c/sst` or `./build/release/c/sst`.
* Update test command: `bash c/tests/golden.sh ./build/debug/c/sst`.

---

## 6. Verification Plan

1. **Git State & History**: Verify with `git status` that files are staged as renames/moves (`R`).
2. **Line Endings**: Run `c/tests/lineendings.sh` to ensure zero line ending regressions across both CRLF and LF files.
3. **C Test Suite**:
   - `cmake --preset ci-debug && cmake --build --preset ci-debug && ctest --preset ci-debug`
   - `cmake --preset ci-release && cmake --build --preset ci-release && ctest --preset ci-release`
4. **Go Test Suite**:
   - `go test -v -race ./...`
   - Verify `tests/golden_test.go` and `tests/tui_golden_test.go` pass without issues.
5. **WebAssembly Build**:
   - Run `bash scripts/build-wasm.sh`.
