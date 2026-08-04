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
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra")
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
