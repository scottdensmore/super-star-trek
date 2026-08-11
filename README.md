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

Resizing the terminal is fine at any point. The display follows it
straight away — while the game is waiting for you to answer, and while
it is waiting for a keystroke at a pause — and both the question you
were asked and the answer you were halfway through typing survive, so
long as there is still room for them. Older text that no longer fits
scrolls away, as it would in any terminal.

Dragged smaller than the 72x24 the panels ask for, they clip rather
than rearrange themselves. A line with more to show than there is room
for ends in `>`, so a shortened number is never mistaken for a small
one, and a heading with no room left is dropped rather than shortened.
A terminal shorter than the panels themselves loses rows off the bottom
of them, the grid keeping its numbering so you can see where it stops.

Shorter than fourteen rows there is no room left for the conversation
at all: the panels are the whole screen and the prompt is not on it.
Just above that there is barely more — at fourteen rows the one line
left over goes entirely to the pager, so a scan pages past without
showing any of itself; at fifteen it comes one line at a time.

Growing the terminal back brings the panels with it, however far the
squeeze went: they redraw from the game state, so they return full of
live readings. The conversation cannot be rebuilt that way — curses
does not reflow it — so whatever scrolled away while the window was
small is gone for good, and an older line that was cut short stays cut
short. Every reading is a command away, though, and none of them costs
anything: `srscan`, `status`, `chart`, `damages`.

The line you are on usually comes back, and it is the one that matters:
a question or a pause the game was part way through writing is written
out again whole once there is room for it, so you are not left typing
into the tail of a prompt. A few prompts finish their line before they
stop to wait; those come back too, with the answer going on the row
below the question — or after it, where the window is down to its last
row.
Only their final line comes back, so a question asked over several
lines returns as the last of them. The one gap left is a question you
have already started answering: that one is left exactly as it was
before, so what comes back may be a stump of older conversation, or
nothing, with the game still waiting behind it.

Do not just answer that one, and do not press Enter either: whatever
you typed is still in the buffer, and a yes/no question reads the
*first* letter, so an `N` typed after a `y` you cannot see answers yes.
Press Ctrl-U first, which throws the line away, and then answer. What
you type lands on whatever the window happens to be showing; the line
it fell on clears when you press Enter, though the characters you
typed stay where they landed.

If the prompt is missing or cut off and you had not started answering
it, press Enter. Nothing waiting behind one commits you to anything
then: a pause moves on, the self-destruct password reads the empty
answer as a refusal and stands the sequence down, and a yes/no
question comes back only as `Please answer with "Y" or "N":`, without
saying again what it asked — answer `N` there and you are back at the
prompt, free to give the command again.

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

