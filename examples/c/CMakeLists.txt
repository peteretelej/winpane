cmake_minimum_required(VERSION 3.20)
project(winpane_examples C)

set(WINPANE_DIR "${CMAKE_SOURCE_DIR}/../../crates/winpane-ffi")
set(WINPANE_INCLUDE "${WINPANE_DIR}/include")
set(WINPANE_LIB_DIR "${CMAKE_SOURCE_DIR}/../../target/debug")

add_executable(hello_hud hello_hud.c)
target_include_directories(hello_hud PRIVATE ${WINPANE_INCLUDE})
target_link_directories(hello_hud PRIVATE ${WINPANE_LIB_DIR})
target_link_libraries(hello_hud winpane)

add_executable(custom_draw custom_draw.c)
target_include_directories(custom_draw PRIVATE ${WINPANE_INCLUDE})
target_link_directories(custom_draw PRIVATE ${WINPANE_LIB_DIR})
target_link_libraries(custom_draw winpane)
