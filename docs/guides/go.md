# Go Guide

Go uses winpane through the C ABI DLL via cgo. Two approaches are available: load the DLL at runtime with `syscall.LoadDLL` (no compiler dependency), or link via cgo with the C header (requires a C compiler on the build machine).

This guide uses `syscall.LoadDLL` since it is the more common approach for Go on Windows and avoids requiring gcc/clang.

## Setup

Build the DLL from source:

```sh
cargo build -p winpane-ffi --release
```

Copy `target/release/winpane_ffi.dll` next to your Go binary (or somewhere on PATH).

## Loading the DLL

```go
package winpane

import (
	"fmt"
	"syscall"
	"unsafe"
)

var (
	dll                    = syscall.MustLoadDLL("winpane_ffi.dll")
	ProcCreate             = dll.MustFindProc("winpane_create")
	ProcDestroy            = dll.MustFindProc("winpane_destroy")
	ProcLastError          = dll.MustFindProc("winpane_last_error")
	ProcHudCreate          = dll.MustFindProc("winpane_hud_create")
	ProcSurfaceDestroy     = dll.MustFindProc("winpane_surface_destroy")
	ProcSurfaceSetText     = dll.MustFindProc("winpane_surface_set_text")
	ProcSurfaceSetRect     = dll.MustFindProc("winpane_surface_set_rect")
	ProcSurfaceShow        = dll.MustFindProc("winpane_surface_show")
	ProcSurfaceHide        = dll.MustFindProc("winpane_surface_hide")
	ProcSurfaceSetPosition = dll.MustFindProc("winpane_surface_set_position")
	ProcSurfaceSetSize     = dll.MustFindProc("winpane_surface_set_size")
	ProcSurfaceSetOpacity  = dll.MustFindProc("winpane_surface_set_opacity")
	ProcPollEvent          = dll.MustFindProc("winpane_poll_event")
	// Add more procs as needed
)
```

## Struct definitions

Match the C ABI layout exactly. All fields must be in the same order as `winpane.h`.

```go
const ConfigVersion = 1

type Color struct {
	R, G, B, A uint8
}

type HudConfig struct {
	Version uint32
	Size    uint32
	X       int32
	Y       int32
	Width   uint32
	Height  uint32
}

type TextElement struct {
	Text       *byte // null-terminated C string
	X          float32
	Y          float32
	FontSize   float32
	Color      Color
	FontFamily *byte // null-terminated C string, or nil
	Bold       int32
	Italic     int32
	Interactive int32
}

type RectElement struct {
	X             float32
	Y             float32
	Width         float32
	Height        float32
	Fill          Color
	CornerRadius  float32
	HasBorder     int32
	BorderColor   Color
	BorderWidth   float32
	Interactive   int32
}

type Event struct {
	EventType   int32
	SurfaceID   uint64
	Key         [256]byte
	MouseButton int32
	MenuItemID  uint32
}

const (
	EventNone               = 0
	EventElementClicked     = 1
	EventElementHovered     = 2
	EventElementLeft        = 3
	EventTrayClicked        = 4
	EventTrayMenuItemClicked = 5
	EventPipSourceClosed    = 6
	EventAnchorTargetClosed = 7
	EventDeviceRecovered    = 8
)
```

## Hello world

```go
package main

import (
	"fmt"
	"time"
	"unsafe"

	"yourmodule/winpane" // your wrapper package
)

func cstr(s string) *byte {
	b := append([]byte(s), 0)
	return &b[0]
}

func lastError() string {
	ret, _, _ := winpane.ProcLastError.Call()
	if ret == 0 {
		return "<nil>"
	}
	// Read C string from pointer
	p := (*[1 << 20]byte)(unsafe.Pointer(ret))
	for i := 0; i < len(p); i++ {
		if p[i] == 0 {
			return string(p[:i])
		}
	}
	return "<unknown>"
}

func main() {
	var ctx uintptr
	ret, _, _ := winpane.ProcCreate.Call(uintptr(unsafe.Pointer(&ctx)))
	if int32(ret) != 0 {
		fmt.Println("create failed:", lastError())
		return
	}
	defer winpane.ProcDestroy.Call(ctx)

	cfg := winpane.HudConfig{
		Version: winpane.ConfigVersion,
		Size:    uint32(unsafe.Sizeof(winpane.HudConfig{})),
		X:       100, Y: 100,
		Width:   300, Height: 100,
	}

	var hud uintptr
	ret, _, _ = winpane.ProcHudCreate.Call(ctx, uintptr(unsafe.Pointer(&cfg)), uintptr(unsafe.Pointer(&hud)))
	if int32(ret) != 0 {
		fmt.Println("hud create failed:", lastError())
		return
	}
	defer winpane.ProcSurfaceDestroy.Call(hud)

	// Add a background rect
	rect := winpane.RectElement{
		X: 0, Y: 0, Width: 300, Height: 100,
		Fill:         winpane.Color{20, 20, 30, 200},
		CornerRadius: 8,
	}
	key := cstr("bg")
	winpane.ProcSurfaceSetRect.Call(hud, uintptr(unsafe.Pointer(key)), uintptr(unsafe.Pointer(&rect)))

	// Add text
	text := winpane.TextElement{
		Text:     cstr("Hello from Go"),
		X:        16, Y: 16,
		FontSize: 18,
		Color:    winpane.Color{255, 255, 255, 255},
	}
	key2 := cstr("msg")
	winpane.ProcSurfaceSetText.Call(hud, uintptr(unsafe.Pointer(key2)), uintptr(unsafe.Pointer(&text)))

	winpane.ProcSurfaceShow.Call(hud)

	fmt.Println("HUD visible. Press Ctrl+C to exit.")
	for {
		time.Sleep(time.Second)
	}
}
```

## Polling events

```go
var event winpane.Event
for {
	ret, _, _ := winpane.ProcPollEvent.Call(ctx, uintptr(unsafe.Pointer(&event)))
	if int32(ret) != 0 {
		break // no more events (returns 1 when empty)
	}
	switch event.EventType {
	case winpane.EventElementClicked:
		key := string(event.Key[:cstrLen(event.Key[:])])
		fmt.Printf("clicked: surface=%d key=%s\n", event.SurfaceID, key)
	}
}

func cstrLen(b []byte) int {
	for i, c := range b {
		if c == 0 {
			return i
		}
	}
	return len(b)
}
```

## Alternative: cgo with the C header

If you prefer cgo, copy `winpane.h` to your project and use `#cgo` directives:

```go
package winpane

/*
#cgo LDFLAGS: -L${SRCDIR} -lwinpane_ffi
#include "winpane.h"
*/
import "C"
import "unsafe"

func Create() (*C.WINPANE_WinpaneContext, error) {
	var ctx *C.WINPANE_WinpaneContext
	if C.winpane_create(&ctx) != 0 {
		return nil, fmt.Errorf("winpane: %s", C.GoString(C.winpane_last_error()))
	}
	return ctx, nil
}
```

This gives you type-checked struct access and avoids manual pointer arithmetic, but requires gcc/clang on the build machine.

## Alternative: JSON-RPC host

If you want to avoid cgo and DLL loading entirely, spawn `winpane-host` as a subprocess and talk JSON-RPC over stdin/stdout. See the [Python guide](python.md) for the protocol pattern, which is the same from any language. Go's `os/exec` and `encoding/json` packages work well for this.

## Next steps

- [FFI design](../design/ffi.md) - C ABI conventions, type mapping, error handling
- [C header reference](../../crates/winpane-ffi/include/winpane.h) - Full generated header
- [Cookbook](../cookbook.md) - Recipes with Rust examples you can translate
- [Limitations](../limitations.md) - Known constraints
