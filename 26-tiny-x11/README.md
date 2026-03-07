# 26-tiny-x11

Tiniest windowed desktop app -- raw X11 protocol over a Unix domain socket.

## Technique

Uses the `no_std` + libc pattern from example 03. Connects to the X11 display
server via an `AF_UNIX` socket at `/tmp/.X11-unix/X0` and speaks the raw X11
binary protocol directly -- no libX11, no xcb, just hand-crafted byte messages.

This is the absolute smallest way to create a windowed application on Linux.
The binary constructs each protocol request byte-by-byte: connection setup,
InternAtom, CreateWindow, ChangeProperty, MapWindow, and the event read loop.

## New concepts

- `AF_UNIX` sockets (paired with `AF_INET` from examples 20/23/25)
- Raw X11 binary protocol (what libX11/xcb abstract away)
- Window system architecture: atoms, properties, event masks
- Binary protocol construction with precise byte layouts

## X11 protocol sequence

1. `socket(AF_UNIX)` + `connect("/tmp/.X11-unix/X0")`
2. Send 12-byte connection request (little-endian, protocol 11.0, no auth)
3. Read setup response -> extract resource_id_base, root window, root_depth, root_visual, white_pixel
4. InternAtom "WM_PROTOCOLS" -> read reply
5. InternAtom "WM_DELETE_WINDOW" -> read reply
6. CreateWindow (parent=root, 400x300, white background, event mask)
7. ChangeProperty WM_NAME = "tiny-x11"
8. ChangeProperty WM_PROTOCOLS = [WM_DELETE_WINDOW]
9. MapWindow
10. Event loop: read 32-byte events, handle Expose/KeyPress/ClientMessage
11. close + exit

## Usage

```sh
cargo build --release
./target/release/tiny-x11
# Connected to X11 display :0
# Root window: 0x000001a3
# Window created: 0x00200001 (400x300)
# Window mapped
# [event] Expose
# Key pressed, exiting
```

## Limitations

- Hardcodes display `:0` (works for nearly all local setups)
- Requires X11 (not Wayland-only; XWayland works fine)
- No Xauth -- if connection is refused, run `xhost +local:` first
- Window has a solid white background only (no drawing commands)
