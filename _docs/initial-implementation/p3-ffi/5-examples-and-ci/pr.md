add C examples, .def file, and CI header check

Two C example programs (hello_hud, custom_draw) validate the FFI API end-to-end with proper error handling. Added winpane.def with all 35 exported symbols, CMake and MSVC build scripts for C consumers, and CI verification that the generated header compiles cleanly.

- Added generated winpane.h to .gitignore
- Updated phases-progress.md marking P3 complete
