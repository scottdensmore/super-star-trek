# Super Star Trek

Super Star Trek is an old text-only game, an early example of a turn-based space strategy sim, written in BASIC. In this game, you are the captain of the starship Enterprise, and your mission is to scout the federation space and eliminate all the invading Klingon ships. You will have to manage the ship energy carefully, use phasers and torpedoes to destroy the Klingons, and find starbases to repair damages and replenish your energy. All of this, rendered with a few characters on screen and a lot of imagination.

Super Star Trek is probably the most famous early text-only game ever created and it inspired many other videogames. Since the code was public domain, during the years, it was changed and improved many times. There are literally thousands of versions out there. All of them are nice, but my preference goes to the classic 1978 version called Super Star Trek.

[Read more about Star Trek and Super Star Trek.](https://en.wikipedia.org/wiki/Star_Trek_%281971_video_game%29)

### Full-screen mode

Run `sst -t` for a full-screen interface: the short-range scan and the
ship's status stay on screen in two panels while the usual game
conversation scrolls below them. It needs a terminal of at least 72x24
and can be combined with the fixed-coordinate option (`sst -f -t`).
Without `-t` the game uses its classic scrolling display.

If full-screen mode isn't possible — the terminal is too small, the
game isn't attached to one (piped input, redirected output, a job with
no tty), or `TERM` names a terminal that can't address the cursor —
`-t` prints a notice and plays in the classic display instead.

### Building

The build is CMake, and a build type is required:

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug   # or Release
cmake --build build
./build/sst
```

You need a C17 compiler and ncurses (`libncurses-dev` on Debian and
Ubuntu; already present on macOS). `ctest --test-dir build` runs the
test suite, which includes a full-screen session driven through a real
terminal and needs `tmux` — without it that one test reports as skipped.

### Windows

There is no working Windows build at the moment.

The old instructions here were `cl /DWINDOWS /Fesst.exe *.c` from a
Visual Studio Developer Command Prompt. That has not worked for some
time: `tui.c` includes `<curses.h>` unconditionally, and `sst.c`,
`finish.c` and `osx.c` all call into it, so the full-screen display
cannot simply be left out of the file list either.

Making it work again means stubbing the display out under `#ifdef
WINDOWS` — `tui_init()` returning false, so the game always runs in its
classic scrolling form — and then actually building it on Windows to
find out what else has drifted. Nobody has a toolchain in play to check
that, so rather than leave instructions that fail, this says so.

