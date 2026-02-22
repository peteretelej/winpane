@echo off
set WINPANE=..\..\crates\winpane-ffi
set LIB_DIR=..\..\target\debug
cl /W4 /I %WINPANE%\include hello_hud.c /link /LIBPATH:%LIB_DIR% winpane.lib
cl /W4 /I %WINPANE%\include custom_draw.c /link /LIBPATH:%LIB_DIR% winpane.lib
