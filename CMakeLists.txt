cmake_minimum_required(VERSION 3.21)
project(sst C)

set(CMAKE_C_STANDARD 17)

set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -DSCORE -DCAPTURE -DCLOAKING")

add_executable(sst sst.c ai.c battle.c events.c finish.c moving.c osx.c planets.c reports.c setup.c)
