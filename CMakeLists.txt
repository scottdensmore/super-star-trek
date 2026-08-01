cmake_minimum_required(VERSION 3.21)
include(CMakePrintHelpers)

project(sst
        LANGUAGES C
        VERSION 1.0)

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DSCORE -DCAPTURE -DCLOAKING")

if(${CMAKE_BUILD_TYPE} STREQUAL "Debug")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DDEBUG")
    cmake_print_variables(CMAKE_C_FLAGS)
endif()

add_executable(sst sst.c ai.c battle.c events.c finish.c moving.c osx.c planets.c reports.c setup.c tui.c tuifmt.c)

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

add_test(NAME instructions
         COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/tests/instructions.sh
                 ${CMAKE_CURRENT_SOURCE_DIR})
set_tests_properties(instructions PROPERTIES TIMEOUT 30)
