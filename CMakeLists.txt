cmake_minimum_required(VERSION 3.1.0)

project(fnd CXX)

find_package(Boost 1.60 REQUIRED COMPONENTS system filesystem)
include_directories( ${Boost_INCLUDE_DIR} )

file(GLOB SOURCES "src/*.cpp")

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wno-long-long -pedantic")

add_executable(fnd ${SOURCES})

target_compile_features(fnd PRIVATE cxx_range_for)

target_link_libraries(fnd ${Boost_LIBRARIES})

install(TARGETS fnd DESTINATION bin)
