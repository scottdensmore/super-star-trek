# C Codebase Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the repository by isolating all classic 1978 C code, headers, documentation, tests, and build definitions into a dedicated `c/` directory tree, keeping the root directory clean and focused on the Go implementation while preserving seamless CMake and CI workflows.

**Architecture:** Relocate 14 C sources, 6 headers, `sst.doc`, and `makefile` to `c/` via `git mv`. Move C test harnesses, shell scripts, and classic golden text files to `c/tests/`. Establish `c/CMakeLists.txt` for the C build/test definitions and replace root `CMakeLists.txt` with a lightweight wrapper calling `add_subdirectory(c)`. Update line-endings enforcement and `.gitattributes` to maintain CRLF/LF file guarantees across the new paths.

**Tech Stack:** C17, CMake 3.21+, CTest, POSIX shell, Git, Go 1.26+

**Spec:** `docs/superpowers/specs/2026-09-19-c-codebase-restructure-design.md`

## Global Constraints

- Use `git mv` for all relocations to preserve full git history.
- Preserve vintage file line endings exactly: 11 CRLF files (`ai.c`, `battle.c`, `events.c`, `finish.c`, `moving.c`, `planets.c`, `reports.c`, `setup.c`, `sst.c`, `sst.doc`, `sst.h`) and all LF files.
- Zero modifications to C source code (`#include` directives remain unchanged).
- `cmake --preset <name>`, `cmake --build --preset <name>`, and `ctest --preset <name>` must continue working without changes from the repository root.
- Pure Go test suite (`go test ./...`) and WASM build (`bash scripts/build-wasm.sh`) must remain green at all stages.

---

### Task 1: Relocate C Sources, Headers, Documentation, and Makefile to `c/`

**Files:**
- Create directory: `c/`
- Move: `ai.c`, `battle.c`, `cmdtab.c`, `events.c`, `finish.c`, `moving.c`, `osx.c`, `planets.c`, `reports.c`, `rules.c`, `setup.c`, `sst.c`, `tui.c`, `tuifmt.c` -> `c/`
- Move: `cmdtab.h`, `finish.h`, `osx.h`, `rules.h`, `sst.h`, `tui.h` -> `c/`
- Move: `sst.doc`, `makefile` -> `c/`
- Modify: `c/makefile`

**Interfaces:**
- Consumes: Vintage and modern C translation units in repository root.
- Produces: `c/` directory containing all 20 C source/header files, `c/sst.doc`, and an updated `c/makefile`.

- [ ] **Step 1: Create `c/` directory and relocate C sources, headers, and docs via `git mv`**

```bash
mkdir -p c
git mv ai.c battle.c cmdtab.c events.c finish.c moving.c osx.c planets.c reports.c rules.c setup.c sst.c tui.c tuifmt.c c/
git mv cmdtab.h finish.h osx.h rules.h sst.h tui.h c/
git mv sst.doc makefile c/
```

- [ ] **Step 2: Update `c/makefile` with all current C objects**

Update `c/makefile` so standalone `make` inside `c/` compiles `sst` cleanly:

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

- [ ] **Step 3: Test standalone build with `make` in `c/`**

Run: `make -C c clean && make -C c && test -x c/sst && make -C c clean`
Expected: Exits 0, building `c/sst` and cleaning up objects.

- [ ] **Step 4: Commit C source relocations**

```bash
git add c/
git commit -m "refactor(c): relocate C sources, headers, docs, and makefile to c/"
```

---

### Task 2: Relocate C Test Harnesses, Scripts, and Golden Fixtures to `c/tests/`

**Files:**
- Create: `c/tests/`, `c/tests/golden/`
- Move: `tests/test_cmdtab.c`, `tests/test_rules.c`, `tests/test_tuifmt.c` -> `c/tests/`
- Move: `tests/analyze.sh`, `tests/eof.sh`, `tests/golden.sh`, `tests/help.sh`, `tests/journey.sh`, `tests/lineendings.sh`, `tests/tournament.sh`, `tests/tui.sh`, `tests/workflow.sh` -> `c/tests/`
- Move: `tests/golden/*.txt` -> `c/tests/golden/`
- Retain in `tests/`: `golden_test.go`, `tui_golden_test.go`, `golden/tui/*.golden`

**Interfaces:**
- Consumes: C tests previously residing in `tests/`.
- Produces: Fully self-contained `c/tests/` test tree; root `tests/` contains only Go tests.

- [ ] **Step 1: Create `c/tests/golden` and move test sources, scripts, and fixtures via `git mv`**

```bash
mkdir -p c/tests/golden
git mv tests/test_cmdtab.c tests/test_rules.c tests/test_tuifmt.c c/tests/
git mv tests/analyze.sh tests/eof.sh tests/golden.sh tests/help.sh tests/journey.sh tests/lineendings.sh tests/tournament.sh tests/tui.sh tests/workflow.sh c/tests/
git mv tests/golden/*.txt c/tests/golden/
```

- [ ] **Step 2: Verify root `tests/` contains only Go test files and TUI goldens**

Run: `ls -la tests`
Expected: Contains only `golden/` (with `tui/` subdirectory), `golden_test.go`, and `tui_golden_test.go`.

- [ ] **Step 3: Verify Go test suite passes**

Run: `go test -v ./tests`
Expected: `TestGoldenParityBasics` and all TUI golden tests PASS.

- [ ] **Step 4: Commit C test relocation**

```bash
git add c/tests/ tests/
git commit -m "refactor(c): relocate C tests, scripts, and golden fixtures to c/tests/"
```

---

### Task 3: Configure CMake Subsystem and Root Delegation Wrapper

**Files:**
- Create: `c/CMakeLists.txt`
- Modify: `CMakeLists.txt` (root wrapper)

**Interfaces:**
- Consumes: Relocated `c/` source files and `c/tests/` test harnesses.
- Produces: Root `CMakeLists.txt` that invokes `add_subdirectory(c)` and `c/CMakeLists.txt` that configures `sst` and CTest targets.

- [ ] **Step 1: Create `c/CMakeLists.txt`**

Write `c/CMakeLists.txt` specifying sources and tests relative to `c/`:

```cmake
cmake_minimum_required(VERSION 3.21)
include(CMakePrintHelpers)

project(sst
        LANGUAGES C
        VERSION 1.0)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DSCORE -DCAPTURE -DCLOAKING")

option(SST_WERROR "Treat compiler warnings as errors (CI does)" OFF)
if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra -Wmissing-prototypes")
    if(SST_WERROR)
        set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Werror")
    endif()
endif()

if(${CMAKE_BUILD_TYPE} STREQUAL "Debug")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DDEBUG")
    cmake_print_variables(CMAKE_C_FLAGS)
endif()

add_executable(sst sst.c ai.c battle.c cmdtab.c events.c finish.c moving.c osx.c planets.c reports.c rules.c setup.c tui.c tuifmt.c)

set(CURSES_NEED_NCURSES TRUE)
find_package(Curses REQUIRED)
target_include_directories(sst PRIVATE ${CURSES_INCLUDE_DIRS} ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(sst PRIVATE ${CURSES_LIBRARIES} m)

find_library(TINFO_LIBRARY tinfo)
if(TINFO_LIBRARY)
    target_link_libraries(sst PRIVATE ${TINFO_LIBRARY})
endif()

add_executable(test_cmdtab tests/test_cmdtab.c cmdtab.c)
target_include_directories(test_cmdtab PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(test_cmdtab PRIVATE m)
add_test(NAME cmdtab COMMAND test_cmdtab)

add_executable(test_tuifmt tests/test_tuifmt.c tuifmt.c)
target_include_directories(test_tuifmt PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(test_tuifmt PRIVATE m)
add_test(NAME tuifmt COMMAND test_tuifmt)

add_executable(test_rules tests/test_rules.c rules.c)
target_include_directories(test_rules PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(test_rules PRIVATE m)
add_test(NAME rules COMMAND test_rules)

add_test(NAME journey
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/journey.sh $<TARGET_FILE:sst> $<CONFIG>)
set_tests_properties(journey PROPERTIES
                     WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
                     TIMEOUT 120)

add_test(NAME journey-tui
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/journey.sh $<TARGET_FILE:sst> $<CONFIG> -t)
set_tests_properties(journey-tui PROPERTIES
                     WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
                     ENVIRONMENT "TERM=dumb"
                     TIMEOUT 120)

add_test(NAME eof COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/eof.sh $<TARGET_FILE:sst>)
set_tests_properties(eof PROPERTIES TIMEOUT 150)

add_test(NAME help COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/help.sh $<TARGET_FILE:sst>)
set_tests_properties(help PROPERTIES TIMEOUT 120)

add_test(NAME tui
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/tui.sh $<TARGET_FILE:sst> $<CONFIG>)
set_tests_properties(tui PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 300)

add_test(NAME tournament
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/tournament.sh $<TARGET_FILE:sst> $<CONFIG>)
set_tests_properties(tournament PROPERTIES TIMEOUT 300)

add_test(NAME golden
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/golden.sh $<TARGET_FILE:sst>)
set_tests_properties(golden PROPERTIES TIMEOUT 120)

set(analyze_flags "-std=gnu${CMAKE_C_STANDARD} ${CMAKE_C_FLAGS}")
foreach(dir ${CURSES_INCLUDE_DIRS})
    set(analyze_flags "${analyze_flags} -I${dir}")
endforeach()
add_test(NAME analyze
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/analyze.sh
                 "${analyze_flags}" "${CMAKE_C_COMPILER}"
                 "$<TARGET_PROPERTY:sst,SOURCES>")
set_tests_properties(analyze PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 300)

add_test(NAME workflow
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/workflow.sh
                 ${CMAKE_SOURCE_DIR})
set_tests_properties(workflow PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 60)

add_test(NAME lineendings
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/lineendings.sh
                 ${CMAKE_SOURCE_DIR})
set_tests_properties(lineendings PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 60)

install(TARGETS sst DESTINATION bin)
install(FILES sst.doc DESTINATION share/doc/super-star-trek)

set(CPACK_PACKAGE_NAME "super-star-trek")
set(CPACK_PACKAGE_VENDOR "Super Star Trek Authors")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Classic terminal space-strategy game written in C17")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_GENERATOR "TGZ;DEB")
set(CPACK_DEBIAN_PACKAGE_MAINTAINER "Super Star Trek Authors")
set(CPACK_DEBIAN_PACKAGE_SECTION "games")
include(CPack)
```

- [ ] **Step 2: Replace root `CMakeLists.txt` with lightweight wrapper**

Write root `CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.21)

project(super-star-trek
        LANGUAGES C
        VERSION 1.0)

enable_testing()
add_subdirectory(c)
```

- [ ] **Step 3: Test CMake configuration from repository root**

Run: `rm -rf build/ci-debug && cmake --preset ci-debug && cmake --build --preset ci-debug`
Expected: Configures, builds `build/ci-debug/c/sst`, `test_cmdtab`, `test_tuifmt`, `test_rules` with exit 0.

- [ ] **Step 4: Commit CMake configuration**

```bash
git add CMakeLists.txt c/CMakeLists.txt
git commit -m "build(cmake): delegate C build targets and tests to c/ subdirectory"
```

---

### Task 4: Update Line Endings Guard and Git Attributes

**Files:**
- Modify: `c/tests/lineendings.sh`
- Modify: `.gitattributes`

**Interfaces:**
- Consumes: Relocated repository layout.
- Produces: Updated `.gitattributes` and `c/tests/lineendings.sh` reflecting paths under `c/`.

- [ ] **Step 1: Update `.gitattributes` with relocated C paths**

Update `.gitattributes`:

```gitattributes
# The golden fixtures are compared byte for byte, so a clone that
# rewrote their line endings would fail every journey.
c/tests/golden/*.txt -text

# Vintage C files (CRLF) and newer C files (LF)
c/*.c -text
c/*.h -text
c/tests/*.sh -text
c/sst.doc -text

# Other shell scripts
*.sh -text
```

- [ ] **Step 2: Update `c/tests/lineendings.sh` file lists**

Update `crlf_files` and `required_seen` in `c/tests/lineendings.sh`:

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
```

and

```sh
required_seen='c/sst.c
c/sst.h
c/sst.doc
c/tui.c
c/tests/lineendings.sh'
```

- [ ] **Step 3: Run `c/tests/lineendings.sh`**

Run: `bash c/tests/lineendings.sh .`
Expected: Output `line endings: 34 files, all as recorded` with exit 0.

- [ ] **Step 4: Commit line ending protections**

```bash
git add .gitattributes c/tests/lineendings.sh
git commit -m "chore(git): update .gitattributes and line endings check for c/ directory structure"
```

---

### Task 5: Update Documentation and End-to-End Verification

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: Working builds in both C and Go.
- Produces: Updated documentation referencing `c/` and verified test suites.

- [ ] **Step 1: Update `README.md` Classic C Edition section**

Update lines 111-135 in `README.md`:

```markdown
## Classic C Edition

The original C implementation and classic curses full-screen interface are preserved in the `c/` directory.

### Requirements
- C17 compiler (`gcc` or `clang`)
- CMake 3.21+
- `libncurses-dev` (Linux) or ncurses (macOS)

### Building
```bash
cmake --preset debug           # or release
cmake --build --preset debug
./build/debug/c/sst
```

### Running C Tests
```bash
ctest --preset debug
bash c/tests/golden.sh ./build/debug/c/sst
```

### C Full-Screen Mode (`sst -t`)
Run `sst -t` for the curses two-panel display. It requires a terminal of at least 72×24 columns and can be combined with `-f` (`sst -f -t`). Without `-t`, the classic scrolling display is used.
```

- [ ] **Step 2: Run C test suite via CTest**

Run: `ctest --preset ci-debug`
Expected: 100% tests pass (or skips for analyze/workflow where tools absent).

- [ ] **Step 3: Run full Go test suite**

Run: `go test -v -race ./...`
Expected: All tests in all packages pass.

- [ ] **Step 4: Run WASM verification**

Run: `bash scripts/build-wasm.sh`
Expected: Builds `web/sst.wasm` cleanly with exit 0.

- [ ] **Step 5: Commit documentation updates**

```bash
git add README.md
git commit -m "docs: update Classic C Edition build and test paths in README"
```
