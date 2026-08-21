cmake_minimum_required(VERSION 3.21)
include(CMakePrintHelpers)

project(sst
        LANGUAGES C
        VERSION 1.0)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DSCORE -DCAPTURE -DCLOAKING")

# Warnings are on for everyone, so a new one is visible where it is
# introduced. They are only fatal where that can be enforced without
# breaking somebody's build on a compiler version nobody here has
# tried: CI turns SST_WERROR on, a contributor gets the warning and a
# working build.
option(SST_WERROR "Treat compiler warnings as errors (CI does)" OFF)
if(CMAKE_C_COMPILER_ID MATCHES "GNU|Clang")
    # -Wmissing-prototypes is what keeps a definition and its
    # declaration in agreement: it fires on any non-static function
    # defined without a prototype in scope, which is the only way a
    # signature can drift apart from the header the callers read. It
    # is the standing check for osx.c, the one file that cannot
    # include sst.h.
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra -Wmissing-prototypes")
    if(SST_WERROR)
        set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Werror")
    endif()
endif()

if(${CMAKE_BUILD_TYPE} STREQUAL "Debug")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DDEBUG")
    cmake_print_variables(CMAKE_C_FLAGS)
endif()

add_executable(sst sst.c ai.c battle.c events.c finish.c moving.c osx.c planets.c reports.c rules.c setup.c tui.c tuifmt.c)

set(CURSES_NEED_NCURSES TRUE)
find_package(Curses REQUIRED)
target_include_directories(sst PRIVATE ${CURSES_INCLUDE_DIRS})
target_link_libraries(sst PRIVATE ${CURSES_LIBRARIES} m)

# tui.c calls setupterm/tigetstr, which some distributions ship in a
# separate libtinfo. Linking -lncurses happens to pull it in where
# libncurses.so is a linker script, but that is luck, not a contract.
find_library(TINFO_LIBRARY tinfo)
if(TINFO_LIBRARY)
    target_link_libraries(sst PRIVATE ${TINFO_LIBRARY})
endif()

enable_testing()
add_executable(test_tuifmt tests/test_tuifmt.c tuifmt.c)
target_link_libraries(test_tuifmt PRIVATE m)
add_test(NAME tuifmt COMMAND test_tuifmt)

# The game's rules, asked directly. Expectations come from sst.doc --
# see tests/test_rules.c and the note at the top of rules.h.
add_executable(test_rules tests/test_rules.c rules.c)
target_link_libraries(test_rules PRIVATE m)
add_test(NAME rules COMMAND test_rules)

add_test(NAME journey
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/journey.sh $<TARGET_FILE:sst> $<CONFIG>)
# Run from the source tree, where a player runs the game and where the
# in-game `help` command finds sst.doc.
# TIMEOUT because the script's own `timeout` guard needs coreutils,
# which macOS runners do not have.
set_tests_properties(journey PROPERTIES
                     WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
                     TIMEOUT 120)

# The same journey with -t: with no terminal to draw on, full-screen
# mode has to fall back to the plain one rather than hang. ctest hands
# the test pipes for stdin and stdout, so this covers the no-tty path
# only -- the terminfo probe in tui_terminal_capable() needs a real
# pty and is checked by hand for now (issue #13). TERM=dumb is pinned
# so a regression here fails as the original hang rather than as the
# quicker error exit an unknown TERM would give.
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

# The only test that drives curses itself, through a pty that tmux
# provides. It reports SKIPPED rather than passing where tmux is
# missing, so a machine without it cannot look like coverage.
# The timeout is far above the ~11s it takes to pass: every check
# inside polls before giving up, so a broadly broken TUI spends its
# time timing out one poll at a time. Better that it reaches the end
# and names each failure than that ctest kills it and reports only
# that something hung.
add_test(NAME tui
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/tui.sh $<TARGET_FILE:sst> $<CONFIG>)
set_tests_properties(tui PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 300)

# The journey that passes game time: movement, combat, docking, and
# the events and pauses that come with them. Seeded per game so a
# failure is reproducible; see the script for why some of its
# assertions are made of the whole battery rather than of one game.
# About 50 seconds here, the slowest test in the suite by four times --
# nearly all of it the busy-wait that types the game's dramatic lines
# out slowly. The timeout leaves six times that for a slower runner.
add_test(NAME tournament
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/tournament.sh $<TARGET_FILE:sst> $<CONFIG>)
set_tests_properties(tournament PROPERTIES TIMEOUT 300)

# Byte-for-byte recordings of fixed journeys. The rest of the suite
# asks whether things happened; this one asks whether anything changed,
# which is the only question that covers the game's arithmetic. It
# needs the seeded generator to be the game's own, which it is.
add_test(NAME golden
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/golden.sh $<TARGET_FILE:sst>)
set_tests_properties(golden PROPERTIES TIMEOUT 120)

add_test(NAME instructions
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/instructions.sh
                 ${CMAKE_CURRENT_SOURCE_DIR})
set_tests_properties(instructions PROPERTIES TIMEOUT 30)

# The score sheet's columns, read off the format strings in finish.c
# rather than off any output -- the only way to reach ten of the twelve
# singular forms and four of the five skill labels, which no journey
# earns. See the script.
add_test(NAME score
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/score.sh
                 ${CMAKE_CURRENT_SOURCE_DIR})
set_tests_properties(score PROPERTIES TIMEOUT 30)

# Static analysis, which is a different question from the warnings the
# ci-* presets make fatal: those are what the compiler notices while
# generating code, where this walks paths and tracks what has been
# written to. An optional cppcheck, clang-tidy, or scan-build check left
# this leg absent everywhere, so this target uses the `gcc -fanalyzer`
# capability the project and Linux CI actually have.
# The script skips where no compiler offers it and fails rather than
# skipping on Linux CI, where GCC is the compiler the build already
# uses. Darwin skips by default even where a Homebrew gcc-N is on the
# path: turning that loose on the macOS SDK under -Werror risks a red
# job from a Darwin-only warning, on a check Linux has already
# delivered. Naming CC, or configuring the tree with such a compiler,
# asks for it there deliberately.
#
# Two things the flags below leave out on purpose. CMAKE_C_FLAGS_<CONFIG>
# is not passed, so this analyses unoptimised code in every preset --
# -O3 makes -fanalyzer slower and noisier, and nothing in the tree reads
# NDEBUG, which is the only thing dropping the Release flags would
# otherwise change. And the script adds -Werror whatever SST_WERROR says, so
# `ctest --preset debug` is warning-fatal for this one test where the
# build it was configured from is not. That is deliberate and it is the
# one place this file departs from the policy above: gating on
# SST_WERROR would leave the test with nothing to fail on, since
# deciding which diagnostics count by pattern is what the first version
# tried and got wrong. The cost lands wherever the analyzer differs from
# the one this was written against, older or newer. CI's own gcc is not
# the one it was developed on, so the first run of this test on a pull
# request is the first time the tree meets that analyzer. The same holds
# for a contributor's own gcc, which reddens their ctest rather than CI;
# an ubuntu-latest default bump would redden every pull request and main
# alike, with no local change to blame. Saying at the site why a
# diagnostic is wrong does not clear it -- under -Werror a comment
# silences nothing. Two things do clear it. Restructuring so the
# analyzer can see the invariant is one, and is what #147 did for
# newqad(), which is why that line moved rather than merely gaining a
# comment. A scoped #pragma GCC diagnostic, with the same justification
# beside it, is the other. Widening the gate is neither. #147.
#
# The flags come from here rather than being written out again in the
# script. CMAKE_C_FLAGS already carries the defines, the warning set and
# the -DDEBUG a Debug build adds, and the standard and curses includes
# are the two the build adds separately. Restating any of it in a sixth
# file is how the build recipe drifted before -- see AGENTS.md's note on
# the presets -- and it had already started: the first version of this
# test passed no -std at all, so it analysed as C23 while every preset
# compiled as gnu17.
set(analyze_flags "-std=gnu${CMAKE_C_STANDARD} ${CMAKE_C_FLAGS}")
foreach(dir ${CURSES_INCLUDE_DIRS})
    set(analyze_flags "${analyze_flags} -I${dir}")
endforeach()
# The compiler and the source list come from the build too, for the
# same reason as the flags: a tree configured with a different
# CMAKE_C_COMPILER would otherwise be analysed by whatever gcc the
# script found, and a source added later would go unanalysed with
# nothing to notice. The canary still decides whether the compiler
# named here can analyse at all, so an AppleClang still skips. On
# Darwin only CC and this compiler are tried, never a gcc that merely
# happens to be on the path.
add_test(NAME analyze
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/analyze.sh
                 "${analyze_flags}" "${CMAKE_C_COMPILER}"
                 "$<TARGET_PROPERTY:sst,SOURCES>")
# SKIP_RETURN_CODE, or the 77 the script exits with where no compiler
# offers -fanalyzer is just a failure -- which is what macOS CI and any
# clang-only machine would have seen.
#
# The timeout went from 180 to 300 with the analyzer depth the script
# now asks for -- factor=30 in tests/analyze.sh, which is where to look
# if this number ever needs revisiting. The run measured 15-17s here
# before and 31-36s after, so a margin of roughly eleven times is now
# roughly nine. That margin is for a runner slower than this machine,
# which is the only thing a timeout here can catch -- a real hang would
# sit far past either number.
set_tests_properties(analyze PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 300)

# The shell inside .github/workflows/ is the one kind of shell here that
# nothing else checks -- and the least likely to be exercised before it
# matters, since a step guarded by `runner.os == 'macOS'` runs only on a
# runner nobody can reproduce locally. #187.
#
# The tree to check goes in as an argument, because the script reads
# .github/workflows rather than anything the build produces. Not because
# it would otherwise be lost: with no argument the script falls back to
# its own directory's parent, so it finds the right tree either way.
# This is the house pattern the instructions and score tests already
# use, and it makes the target visible in the ctest command rather than
# inferred. Note it does not rescue a copied build/ -- the path is baked
# in at configure time, so a copy's build/ still names the original.
add_test(NAME workflow
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/workflow.sh
                 ${CMAKE_CURRENT_SOURCE_DIR})
# 77 where the linting cannot be done -- no actionlint, or an
# actionlint whose shellcheck integration is inert -- the way analyze
# skips without an analyzer. The script fails rather than skipping when
# CI is set and the platform is Linux, which is where CI installs the
# tool; a leg that quietly checked nothing is what #187 is about.
set_tests_properties(workflow PROPERTIES SKIP_RETURN_CODE 77 TIMEOUT 60)
