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
