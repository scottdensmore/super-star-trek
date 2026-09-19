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
