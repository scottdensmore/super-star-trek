# Super Star Trek

Super Star Trek is an old text-only game, an early example of a turn-based space strategy sim, written in BASIC. In this game, you are the captain of the starship Enterprise, and your mission is to scout the federation space and eliminate all the invading Klingon ships. You will have to manage the ship energy carefully, use phasers and torpedoes to destroy the Klingons, and find starbases to repair damages and replenish your energy. All of this, rendered with a few characters on screen and a lot of imagination.

Super Star Trek is probably the most famous early text-only game ever created and it inspired many other videogames. Since the code was public domain, during the years, it was changed and improved many times. There are literally thousands of versions out there. All of them are nice, but my preference goes to the classic 1978 version called Super Star Trek.

[Read more about Star Trek and Super Star Trek.](https://en.wikipedia.org/wiki/Star_Trek_%281971_video_game%29)

### Full-screen mode

Run `sst -t` for a full-screen interface: the short-range scan and the
ship's status stay on screen in two panels while the usual game
conversation scrolls below them. It needs a terminal of at least 72x24
(columns by rows) and can be combined with the fixed-coordinate option
(`sst -f -t`).
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
A terminal too short for the panels is the panels' problem, not the
conversation's: they lose rows off the bottom, the grid keeping its
numbering for as long as it has rows to number, and the conversation
keeps three rows down to six — two of it and the line the prompt or the
pager sits on. Below six there is not that much left to divide and it
keeps what there is: two rows at five and one at four. So the prompt
stays on screen down to four rows, and down to six a paged command shows
a line of itself before stopping to ask — at five its one line goes to
the blank that `lrscan`, `chart` and `status` open with, though
`srscan`, which has none, still shows a line there, and at four the
pager takes the only row and a paged command shows nothing of itself at
all. Shorter than four the panels have taken the screen: the game still
reads what you type, with nothing on it to say so.

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
lines returns as the last of them. An answer you had already started
typing usually comes back with its question, on the row below it. You
may get back only one of them. Where the question ended its own line the
two sit on separate rows, so a wrapped answer needs three rows and a
window of one or two — five rows or fewer — cannot hold the pair; a
question that has itself wrapped needs a row more again. The question is
what goes, leaving your own typing with nothing above it to say what it
answers, and on a very wide terminal it is the answer that goes instead,
however tall the window is. Shorter still and only part of whichever
survived is left, and sometimes neither comes back, leaving older
conversation on screen with the game waiting behind it.

When the question and everything you typed are both on screen, carry on
and answer — that is the ordinary case now. When they are not, give the
terminal another row or two first: that usually brings it back with
your typing untouched, which beats retyping it. A very wide terminal is
the exception — more rows will not bring the answer back there, though
making it narrow enough will.

If it does not come back, or the terminal cannot grow, do not just
answer it, and do not press Enter either: whatever you typed is still
in the buffer, and a yes/no question reads the *first* letter, so an
`N` typed after a `y` you cannot see answers yes.
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
`-t` prints a notice and plays in the classic display instead. That
game stays classic to the end — enlarging the window will not bring the
panels up part way through it.

A game that went classic gets the choice made again for the next one,
though. So if the terminal was too small and you have since made it
bigger, answering yes to "Do you want to play again?" starts the next
game with the panels. If you resized it and it is still too small, the
game says what size it read back rather than leaving you to wonder —
72x24 is the whole of what it takes, and a tmux status bar can cost you
the row that decides it. Leave the terminal alone and it stays quiet:
you were told at startup and nothing has changed since.

If the size it names is not your window's, something has exported
`LINES` or `COLUMNS`. Curses believes those over the terminal, so the
game is measuring what they say — which is how a 100x30 window can be
told it is too small. Resizing moves only whichever of the two was not
pinned, and if that carries the measurement past 72x24 the panels come
up at the pinned size rather than your window's. Pinned smaller, they
are drawn narrow or short in a window with room to spare, and they stay
that way however you drag it; pinned larger, they are drawn as though
the window were bigger, so the frame runs off the edge and wraps back
onto the row below — the worse of the two, and not the tidy clipping a
window merely dragged small would get. Unset them and start again.

It only goes that way. Panels that are up stay up for the rest of the
session however you resize, as above: they clip rather than give way to
the classic display.

### Building

The build is CMake, driven through presets:

```sh
cmake --preset debug           # or release
cmake --build --preset debug
./build/debug/sst
```

You need a C17 compiler, CMake 3.21 or newer, and ncurses
(`libncurses-dev` on Debian and Ubuntu; already present on macOS).
`ctest --preset debug` runs the test suite, which includes a full-screen
session driven through a real terminal and needs `tmux` — without it
that one test reports as skipped.

`cmake --list-presets` shows them all. There are four: `debug` and
`release`, and `ci-debug` and `ci-release`, which add `-Werror` and are
exactly what CI runs — so you can reproduce a CI warning locally rather
than discovering it in a pull request.

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

