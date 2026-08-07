#!/bin/sh
# Drives the full-screen display through a real terminal.
#
# Everything else in the suite talks to the game through pipes, which
# reaches the no-terminal fallback but never exercises curses itself.
# This one runs the game inside tmux so there is a genuine pty, and
# reads the screen back the way a player would see it.
#
# It covers the ways into a running game -- answering the setup
# questions, giving the answers on the command line, thawing a saved
# game, and starting a second game after quitting -- because each of
# them arrives at the panels differently. It also covers the two
# refusals only a real terminal can produce: one curses cannot drive,
# and one too small for the panels. Both have to fall back to the
# classic display and keep playing.
#
# The size matters. 80x24 is what most terminal emulators open at and
# 72x24 is the smallest the TUI accepts; at both the message window is
# only ten lines. The startup banner used to overflow it, which is how
# the pager came to be waiting for a keystroke before the game had
# asked the player anything, and to eat the first letter of the answer;
# the banner gives up its blank lines at these sizes now and fits
# exactly, which the checks in the loop below hold it to. Ten lines of
# paging but eleven rows: the prompt lands on the last one.
#
# Usage: tui.sh /path/to/sst [build-type]
# Exits 77 (ctest SKIP_RETURN_CODE) when there is no tmux to run in.
#
# Needs a sleep that takes a fraction, which GNU and BSD both have but
# POSIX does not promise; busybox would need whole seconds.

set -u

SST="${1:-./sst}"
BUILD_TYPE="${2:-}"
case "$SST" in
	/*) ;;
	*) SST="$(pwd)/$SST" ;;
esac
srcdir=$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)

if ! command -v tmux >/dev/null 2>&1; then
	# A developer without tmux gets a skip. CI does not: this is the
	# only test that runs curses at all, and a green build that quietly
	# skipped it would be worse than no test.
	if [ -n "${CI:-}" ]; then
		echo "FAIL: tmux is missing on CI, so the TUI went untested" >&2
		exit 1
	fi
	echo "SKIP: tmux is not installed, so there is no pty to test in" >&2
	exit 77
fi
if [ ! -x "$SST" ]; then
	echo "FAIL: $SST is not executable" >&2
	exit 1
fi

# Run on a socket of our own, never the default one. A developer's tmux
# server is usually sitting on the default socket with a day's real work
# in it, and anything server-wide -- this script's cleanup, or an agent
# tidying up after driving the TUI by hand -- takes that down too. -L
# buys a private server, so cleanup can be as blunt as it needs to be.
socket="sst-tui-$$"
session="tui"
pane="$session:0.0"

# -f /dev/null as well as -L: a private socket is not a private config,
# and `set -g base-index 1` -- one of the commoner things people put in
# ~/.tmux.conf -- moves the pane out from under the name used below, so
# every capture comes back empty and the game gets blamed for it.
tm() { tmux -f /dev/null -L "$socket" "$@"; }

# tmux leaves the socket file behind when the server exits, so a suite
# that ran often would slowly fill the directory with dead ones. This is
# where tmux puts them on both Linux and macOS: $TMUX_TMPDIR if it is
# set, /tmp otherwise.
cleanup() {
	tm kill-server 2>/dev/null
	rm -f "${TMUX_TMPDIR:-/tmp}/tmux-$(id -u)/$socket"
}

# The saved-game cases write a .trk file, which the game puts in the
# directory it is playing in. That must not be the source tree.
work=$(mktemp -d "${TMPDIR:-/tmp}/sst-tui.XXXXXX")
if [ -z "${work:-}" ] || [ ! -d "$work" ]; then
	echo "FAIL: could not create a temporary directory" >&2
	exit 1
fi

finish() { cleanup; rm -rf "$work"; }
trap 'finish' EXIT
trap 'finish; exit 130' INT
trap 'finish; exit 143' TERM
trap 'finish; exit 129' HUP

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

screen() { tm capture-pane -t "$pane" -p 2>/dev/null; }

# The same, with whatever has scrolled off. Only meaningful once the
# game has fallen back to the classic display: curses runs on the
# alternate screen, which keeps no history.
scrollback() { tm capture-pane -t "$pane" -p -S - 2>/dev/null; }

wait_scrollback() {
	i=0
	while [ "$i" -lt 80 ]; do
		if scrollback | grep -qF -- "$1"; then return 0; fi
		i=$((i + 1))
		sleep 0.1
	done
	return 1
}

dump() {
	printf '  --- screen ---\n' >&2
	screen | sed 's/^/  | /' >&2
}

# For failures where the evidence may be exactly what scrolled away.
dump_scrollback() {
	printf '  --- screen, with scrollback ---\n' >&2
	scrollback | sed 's/^/  | /' >&2
}

# Wait until the screen shows $1, up to about eight seconds. Polling
# beats a fixed sleep: curses paints in its own time, and a machine
# under load should not turn into a failure.
wait_for() {
	i=0
	while [ "$i" -lt 80 ]; do
		if screen | grep -qF -- "$1"; then return 0; fi
		i=$((i + 1))
		sleep 0.1
	done
	return 1
}

# Answer any paging prompts until the command prompt comes back. A
# space is only ever sent while a pause is actually showing, so this
# cannot quietly paper over a pause that should not have been there.
to_command() {
	i=0
	while [ "$i" -lt 40 ]; do
		screen | grep -qF 'COMMAND' && return 0
		if screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
			tm send-keys -t "$pane" Space
		fi
		i=$((i + 1))
		sleep 0.25
	done
	return 1
}

want() {
	screen | grep -qF -- "$2" || { fail "$1"; dump; }
}

# Like to_command, but reports in $saw whether $1 was on screen at any
# point along the way. For text meant to arrive and then scroll on:
# looking only once the prompt is back asks whether it is *still*
# there, which depends on how much the game had to say afterwards.
saw=""
to_command_seeing() {
	saw=""
	i=0
	while [ "$i" -lt 40 ]; do
		screen | grep -qF -- "$1" && saw=yes
		screen | grep -qF 'COMMAND' && return 0
		if screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
			tm send-keys -t "$pane" Space
		fi
		i=$((i + 1))
		sleep 0.25
	done
	return 1
}

# Wait for $2, or fail with $1 and the screen attached. Bare
# `wait_for ... || fail ...` reports the message and nothing else,
# which in a CI log is the least useful half of the story.
expect() {
	wait_for "$2" || { fail "$1"; dump; }
}

# The same for a regular expression.
expect_re() {
	i=0
	while [ "$i" -lt 80 ]; do
		screen | grep -qE -- "$2" && return 0
		i=$((i + 1))
		sleep 0.1
	done
	fail "$1"
	dump
}

# And the same confined to the status panel, which starts one column
# past the quadrant panel. The game prints "Stardate" into the message
# window too -- in the briefing, and in any `status` or `srscan` -- so
# an unconfined search would quietly start passing on that instead.
expect_status() {
	i=0
	while [ "$i" -lt 80 ]; do
		screen | cut -c30- | grep -qF -- "$2" && return 0
		i=$((i + 1))
		sleep 0.1
	done
	fail "$1"
	dump
}

# The quadrant panel's title carries coordinates only while a game is
# set up; without one the box keeps its bare " Quadrant " label, so a
# fixed-string match for that would pass on the empty frame it is
# supposed to catch.
#
# Matched on the first line of the screen, which is where that border
# sits. Anywhere would also match the message window: the briefing ends
# by saying which quadrant the Enterprise is currently in, so an
# unanchored search passes even with the panel title gone.
#
# Polled for the same reason wait_gone is: the prompt is written before
# the panels beside it are redrawn, so a single look the moment
# COMMAND> appears can catch them still blank.
want_quadtitle() {
	i=0
	while [ "$i" -lt 40 ]; do
		screen | head -1 | grep -qE ' Quadrant [0-9]+ - [0-9]+ ' &&
			return 0
		i=$((i + 1))
		sleep 0.1
	done
	fail "$1"
	dump
}

# wait_gone confined to the status panel's columns, for the same reason
# expect_status is: the game writes "Stardate" into the message window
# too, and waiting for it to leave the whole screen would be waiting on
# the wrong thing.
wait_gone_status() {
	i=0
	while [ "$i" -lt 40 ]; do
		screen | cut -c30- | grep -qF -- "$2" || return 0
		i=$((i + 1))
		sleep 0.1
	done
	fail "$1"
	dump
}

unwanted() {
	if screen | grep -qF -- "$2"; then fail "$1"; dump; fi
}

# Wait for $2 to leave the screen. A prompt is printed before the
# panels beside it are redrawn, so asking the instant the text appears
# can catch the previous game still framed for a few milliseconds --
# a race that would report a bug nobody has. Something that is meant to
# be gone goes within the timeout; something that is not, never does.
wait_gone() {
	i=0
	while [ "$i" -lt 40 ]; do
		screen | grep -qF -- "$2" || return 0
		i=$((i + 1))
		sleep 0.1
	done
	fail "$1"
	dump
}

# Long-range scan says something the always-on panels never do, so
# waiting for it proves a command was read whole. Damaged sensors
# answer differently and that proves it just as well -- what a
# truncated command gets is UNRECOGNIZED.
LRSCAN='[Ll]ong-range scan for Quadrant|LONG-RANGE SENSORS DAMAGED'

# tmux takes a shell command line, not an argument list, so the path
# has to survive being quoted into one.
sstq=$(printf "%s" "$SST" | sed "s/'/'\\\\''/g")

# Start the game on a terminal $1 wide by $2 tall, passing $3 (if any)
# on the command line and running it under $4 (if any, an `env ...`
# prefix). It reads sst.doc from the current directory, and dies with
# the session.

# Where the game plays. The source tree by default, because `help`
# reads sst.doc from the current directory; the saved-game cases below
# move it to a scratch directory, which they can because they never
# ask for help.
startdir="$srcdir"
start() {
	cleanup
	tm new-session -d -x "$1" -y "$2" -s "$session" -c "$startdir" \
		"${4:-} '$sstq' -t ${3:-}; sleep 30" 2>/dev/null || {
		echo "FAIL: could not start a ${1}x${2} tmux session" >&2
		exit 1
	}
}

# --- the game asks before it pauses, at every size it accepts -------
# 80x24 is what most emulators open at; 72x24 is the smallest the TUI
# will run in. The banner is longer than the message window at both, so
# it scrolls -- ordinary. Stopping to ask for a keystroke is not:
# nothing has been asked yet, so there is no reader to wait for, and
# the prompt sits exactly where the player is about to type an answer.
for size in 80x24 72x24; do
	cols=${size%x*}
	rows=${size#*x}
	start "$cols" "$rows"

	if ! wait_for 'regular, tournament, or frozen'; then
		fail "$size: the game never asked its first question"
		dump
		exit 1
	fi
	# Curses is really running, not the classic display falling back
	# inside the pty. Without this every assertion below is satisfied
	# by the plain path, and a broken tui_init() would still look
	# green -- from a script whose whole point is to exercise curses.
	# The panel titles are drawn in TUI mode and nowhere else.
	want "$size: the full-screen display never came up" " Status "
	want "$size: the full-screen display never came up" " Quadrant "

	unwanted "$size: paused with [HIT SPACE BAR] before asking anything" "HIT SPACE BAR"
	unwanted "$size: paused with [CONTINUE?] before asking anything" "[CONTINUE?]"

	# The whole ship, not the nacelle. Ten lines of banner fit the
	# window exactly, but only once the blank lines around its three
	# closing titles are given up; with them the top of the drawing
	# scrolls away before the player has ever seen it.
	#
	# Two lines, because the unfixed banner loses exactly five: the
	# first is the top of the drawing, the second is the last line it
	# loses. The closing titles never scroll and are no test of this.
	want "$size: the top of the banner scrolled away" "__________________"
	want "$size: the third line of the drawing scrolled away" "||    \----.________.----/"
	# A pattern that opens with a dash, so the day a helper stops
	# passing -- to grep, something says so.
	want "$size: the banner has no title" "-SUPER- STAR TREK"

	# With a pager waiting, its getch() swallowed the leading
	# character and the game saw "egular".
	tm send-keys -t "$pane" 'regular' Enter
	if ! wait_for 'Short, Medium, or Long'; then
		fail "$size: the game did not accept 'regular' as the first answer"
		dump
		exit 1
	fi
	unwanted "$size: the first answer lost a character" 'What is "'
done

# --- paging still works once the game is running --------------------
# Carrying on from the 72x24 game above: the rest of setup, then the
# briefing, which does page.
tm send-keys -t "$pane" 'short' Enter
expect "setup did not reach the skill question" 'Novice, Fair, Good, Expert'
tm send-keys -t "$pane" 'novice' Enter
sleep 0.5
tm send-keys -t "$pane" 'xyz' Enter

# The briefing is the only place the game states the mission and the
# deadline, and at novice skill it is the tutorial. It is longer than
# the ten-line window, so it has to stop and let itself be read --
# it used to print in one go and leave the player looking at its last
# four lines, because the newlines inside it were not counted.
#
# Only checked at this size. Both sizes in the loop above are 24 rows,
# so the message window is ten lines in each and the width has nothing
# to do with it.
expect "the briefing scrolled past its opening" 'The Federation is being attacked by'
# And it waits there to be read, rather than the opening merely
# flickering past on the way to the prompt.
sleep 0.5
want "the briefing did not wait to be read" "SPACE BAR"
want "the briefing's opening did not stay on screen" 'The Federation is being attacked by'

if ! to_command_seeing 'Good Luck!'; then
	fail "the game never reached its command prompt"
	dump
	exit 1
fi
# The rest of it arrived after the pause: the second half is reachable,
# not just the first half held.
[ -n "$saw" ] || fail "the rest of the briefing never arrived"

# A manual entry runs well past the ten-line message window, so it has
# to page -- the fix must only have quietened the pauses nobody asked
# for, not the ones that do the job.
tm send-keys -t "$pane" 'help move' Enter
if ! wait_for 'CONTINUE'; then
	fail "long output no longer pages"
	dump
fi
# And the pause really does hold: still there a moment later, waiting,
# rather than having flashed past on its own.
sleep 0.5
want "the paging prompt did not wait for a keystroke" "CONTINUE"

# One keystroke buys one screen, not the whole entry. A gate that
# switched paging off again after the first pause would let the rest of
# the manual scroll away, and answering pauses until the prompt returns
# would not notice: the manual runs to several screens, so the command
# prompt must not be back yet.
# Watched for a few seconds rather than sampled once: a single look
# after a fixed sleep passes on a machine that has not repainted yet.
tm send-keys -t "$pane" Space
i=0
while [ "$i" -lt 16 ]; do
	if screen | grep -qF 'COMMAND'; then
		fail "the manual stopped paging after its first screen"
		dump
		break
	fi
	i=$((i + 1))
	sleep 0.25
done

# Spaces get it moving again -- however many pages the entry runs to.
to_command || { fail "the manual never finished paging"; dump; }

# The game is still listening.
tm send-keys -t "$pane" 'lrscan' Enter
expect_re "the game did not carry on after the paging prompt" "$LRSCAN"

# --- and keeps its spacing where there is room -----------------------
# The other half of the same rule. The banner only gives its blank
# lines up when the window is too small for them; on a terminal that
# can afford them they are still there, which is what stops the fix
# from being "delete the spacing".
start 80 30
if ! wait_for 'regular, tournament, or frozen'; then
	fail "80x30: the game never asked its first question"
	dump
else
	want "80x30: the top of the banner is missing" "__________________"
	# The line after the ship's name is blank at this size and not at
	# 24 rows. awk rather than grep: the assertion is about a line
	# being empty, which grep cannot say on its own.
	if ! screen | awk '/THE USS ENTERPRISE/ {getline; if ($0 ~ /^[[:space:]]*$/) ok = 1}
	                   END {exit !ok}'; then
		fail "80x30: the banner gave up its spacing on a terminal with room for it"
		dump
	fi
fi

# --- the display follows the terminal ---------------------------------
# The windows used to be sized once and never again, so a terminal
# dragged mid-game left the panels at their old size with the rest of
# the screen empty beside them. Worse, curses reports a resize the same
# way it reports a keypress, so a drag could answer "hit space bar to
# continue" and page away text nobody had read.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "resize: the game never reached its command prompt"
	dump
else
	tm resize-window -t "$session" -x 120 -y 40
	sleep 0.5
	tm send-keys -t "$pane" 'lrscan' Enter
	expect_re "resize: the game stopped answering after a resize" "$LRSCAN"
	# The status panel is the right-hand one, so its border ends the
	# top line. On a 120-column terminal that line has to reach well
	# past where an 80-column one ended.
	if [ "$(screen | head -1 | wc -c)" -lt 100 ]; then
		fail "resize: the panels kept their old width"
		dump
	fi

	# Shrinking follows too, and the game keeps answering.
	tm resize-window -t "$session" -x 72 -y 24
	sleep 0.5
	tm send-keys -t "$pane" 'lrscan' Enter
	expect_re "resize: the game stopped answering after shrinking" "$LRSCAN"
	if [ "$(screen | head -1 | wc -c)" -gt 90 ]; then
		fail "resize: the panels kept their old width after shrinking"
		dump
	fi
fi

# --- a resize during a pause redraws without waiting for the player ---
# The windows used to be rebuilt only at the next prompt, so a terminal
# dragged while the game sat on "hit space bar to continue" left the
# panels at their old size until the player answered -- the new columns
# empty beside them, the old border stopping short of the edge. Nothing
# was wrong with the game, but it read as one that had stopped.
#
# A pause is where this is worth checking, because it is the one place
# the game is blocked on a keystroke with a full screen behind it. The
# resize must not be taken for that keystroke either: the page the
# player is reading has to still be there afterwards, and one space has
# to move on by exactly one page.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "resize pause: the game never reached its command prompt"
	dump
else
	# `help move` is several screens long, so it pages.
	tm send-keys -t "$pane" 'help' Enter
	sleep 0.3
	tm send-keys -t "$pane" 'move' Enter
	expect_re "resize pause: help never paged" 'CONTINUE|HIT SPACE BAR'
	# Guarded, because everything below reads the paused screen: with
	# no pause to interrupt there is nothing here to measure, and the
	# failure has already been reported.
	if screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
		tm resize-window -t "$session" -x 120 -y 40
		sleep 1

		# No keystroke has been sent. The panels have to have
		# followed anyway.
		if [ "$(screen | head -1 | wc -c)" -lt 100 ]; then
			fail "resize pause: the panels kept their old width while the game waited"
			dump
		fi
		# And the page the player was reading is still on screen,
		# still asking -- the resize was not eaten as the answer.
		if ! screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
			fail "resize pause: the resize answered the pause"
			dump
		fi
		if ! screen | grep -qF 'usual way to move'; then
			fail "resize pause: the page being read was lost"
			dump
		fi

		# One keystroke, one page.
		tm send-keys -t "$pane" Space
		expect "resize pause: one keystroke did not advance one page" \
			'horizontal and vertical displacements'
	fi
fi

# --- and the same when the terminal gets smaller ----------------------
# Shrinking is the harder direction and the one that stayed broken
# longest. Curses clips the message window when it takes the resize in,
# and it does that before the game is told, so the rows it throws away
# -- the bottom ones, where the newest text is -- are gone before any
# of this code runs. The prompt went with them: a screen waiting for a
# keystroke with nothing on it to say so.
#
# Starting large and ending at the minimum is the case that failed. A
# gentler shrink passed even before the fix, because the prompt happened
# to survive the clip.
start 120 40 'tournament 7 short novice pw'
if ! to_command; then
	fail "shrink pause: the game never reached its command prompt"
	dump
else
	tm send-keys -t "$pane" 'help' Enter
	sleep 0.3
	tm send-keys -t "$pane" 'move' Enter
	expect_re "shrink pause: help never paged" 'CONTINUE|HIT SPACE BAR'
	if screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
		tm resize-window -t "$session" -x 72 -y 24
		sleep 1

		if ! screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
			fail "shrink pause: shrinking took the prompt away"
			dump
		fi
		# The prompt alone is not enough: the text being read has
		# to come with it, or the player answers a pause about a
		# page they can no longer see. This one passes against a
		# pre-fix build too -- the clip took the prompt and left
		# this text -- so it is a property worth holding rather
		# than evidence of the bug.
		if ! screen | grep -qF 'usual way to move'; then
			fail "shrink pause: shrinking paged away unread text"
			dump
		fi
		# Exactly once. The prompt is reprinted when the clip took
		# it, and left alone when it did not.
		if [ "$(screen | grep -cE 'CONTINUE|HIT SPACE BAR')" -ne 1 ]; then
			fail "shrink pause: the prompt is on screen more than once"
			dump
		fi
	fi
fi

# --- a resize while an answer is half typed ---------------------------
# The other place the game blocks on the keyboard, and the one where
# there is something to lose: the line the player has typed so far.
# wgetnstr returned ERR on the resize and threw the partial line away
# with it, so a drag mid-word left the prompt looking untouched and the
# answer gone -- type the rest, press Enter, and the game gets only the
# rest. The reader takes a keystroke at a time now and keeps its own
# buffer, so the resize passes between keystrokes.
#
# Note what is *not* asserted here: the panels. On this path they
# followed the terminal even before the fix, because the failing
# wgetnstr was re-entered and curses redrew them on its way back in. A
# width check here would pass against the bug -- it was written, tried
# against a pre-fix build, and taken out again. The pause above is where
# the panels really did stay stale.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "resize typing: the game never reached its command prompt"
	dump
else
	# Half a command, no Enter: the game is inside the line read.
	tm send-keys -t "$pane" 'srsc'
	sleep 0.5
	tm resize-window -t "$session" -x 110 -y 32
	sleep 1

	# Still there to finish, and once. Half-eaten shows as a bare
	# prompt; echoed again shows as `srscsrsc`, which an unanchored
	# pattern would match just as happily -- it did, while the answer
	# really was being written twice.
	if ! screen | grep -qE 'COMMAND> *srsc *$'; then
		fail "resize typing: what had been typed did not survive the resize, or was echoed twice"
		dump
	fi
	# Once, and only once. A pending answer used to make the restore
	# decide the prompt had been clipped -- what follows the prompt on
	# screen is the answer, not the blanks it was looking for -- so it
	# wrote another copy at every step of a drag. Twelve widen steps
	# stood eight prompts up the window. The line-shape check above
	# cannot see that: the last copy looks perfectly correct.
	if [ "$(screen | grep -c 'COMMAND>')" -ne 1 ]; then
		fail "resize typing: the prompt was written out again over a pending answer"
		dump
	fi

	# Finishing it has to land after the answer, not on top of it.
	# The repaint leaves the cursor where an echo belongs; when it was
	# left in front of the answer instead, `an` typed over `sr` and
	# the line read `ansc` while the game held `srscan` -- a command
	# the screen never showed.
	tm send-keys -t "$pane" 'an'
	sleep 0.5
	if ! screen | grep -qE 'COMMAND> *srscan *$'; then
		fail "resize typing: finishing the answer did not land after it"
		dump
	fi
	tm send-keys -t "$pane" Enter
	expect "resize typing: the finished command did not run" 'Condition'
	if screen | grep -q 'UNRECOGNIZED'; then
		fail "resize typing: the finished command was not understood"
		dump
	fi
fi

# --- a shrink while an answer is half typed ---------------------------
# The direction nothing covered, and it is where every late defect in
# this change hid. Growing cannot clip, and the pause has no answer to
# lose, so a width shrink with something typed is the only path that
# exercises putting a clipped prompt *and* its answer back.
#
# Long enough to wrap at the narrower width, because a wrapped pair is
# the case that stumped the restore: the text on screen after the resize
# is not the string it is looking for, and the copies it wrote instead
# stacked up one per step of a drag.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "shrink typing: the game never reached its command prompt"
	dump
else
	answer=zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz
	tm send-keys -t "$pane" "$answer"
	sleep 0.5
	tm resize-window -t "$session" -x 76 -y 30
	sleep 1

	if [ "$(screen | grep -c 'COMMAND>')" -ne 1 ]; then
		fail "shrink typing: the prompt is on screen more than once"
		dump
	fi
	# The whole answer, counted rather than matched as one string:
	# at this width it wraps, and the message window starts at column
	# one, so joining the rows puts the left margin in the middle of
	# it. Every character of it has to still be there -- fewer means
	# the shrink ate some. More would mean a second copy, which the
	# prompt count above is what catches.
	if [ "$(screen | tr -cd z | wc -c)" -lt 80 ]; then
		fail "shrink typing: the answer did not survive the shrink whole"
		dump
	fi
	# And the conversation it was part of is still there.
	if ! screen | grep -qF 'Good Luck'; then
		fail "shrink typing: the conversation was pushed off the screen"
		dump
	fi
fi

# --- the same before the message window has filled --------------------
# The case above types at the command prompt, where the conversation has
# already filled the window, so the last row with anything on it *is*
# the bottom row. That makes it blind to where the clipped line is put
# back: bottom-anchored and line-anchored are the same row there, and a
# build that anchors to the bottom passes it.
#
# The setup questions are the other half, and every game starts there:
# the window is mostly empty, the live line sits in the middle of it,
# and anchoring to the bottom leaves the mangled copy stranded above
# with blank rows between. Same for anything after a clearscreen().
start 110 40
if ! wait_for 'regular, tournament, or frozen'; then
	fail "shrink setup: the game never asked its first question"
	dump
else
	# Long enough that the question and the answer wrap at 76 columns:
	# the question alone is 53 of the 74 the message window gets.
	setupanswer=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
	tm send-keys -t "$pane" "$setupanswer"
	sleep 0.5
	tm resize-window -t "$session" -x 76 -y 40
	sleep 1

	if [ "$(screen | grep -c 'regular, tournament, or frozen')" -ne 1 ]; then
		fail "shrink setup: the question is on screen more than once"
		dump
	fi
	# 49, not 40: the banner and the question carry nine a's of their
	# own at either size, so a bare "at least 40" is satisfied while
	# nine of the typed characters are missing. The z's in the section
	# above need no such allowance -- the screen has none of its own.
	if [ "$(screen | tr -cd a | wc -c)" -lt 49 ]; then
		fail "shrink setup: the answer did not survive the shrink whole"
		dump
	fi
fi

# --- a shrink with a conversation behind it ---------------------------
# " COMMAND> " is the most repeated string in the game, so deciding
# whether the live prompt survived a shrink by looking for that text
# finds an answered prompt further up instead. The cursor then sits in
# the middle of the history: the player types into a command they ran
# two turns ago, and the game reads a line that was never on screen.
#
# Needs a few commands behind it -- with an empty conversation there is
# no older prompt to match, which is why every earlier resize case here
# passes either way.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "shrink history: the game never reached its command prompt"
	dump
else
	for c in status damages; do
		tm send-keys -t "$pane" "$c" Enter
		sleep 0.5
	done
	tm resize-window -t "$session" -x 100 -y 24
	sleep 1
	tm send-keys -t "$pane" 'ch'
	sleep 0.5
	# What is typed has to be at the bottom, on a prompt of its own.
	if ! screen | grep -v '^ *$' | tail -1 | grep -qE 'COMMAND> *ch *$'; then
		fail "shrink history: typing after the shrink did not go to the live prompt"
		dump
	fi
	tm send-keys -t "$pane" 'art' Enter
	expect "shrink history: the command typed after the shrink did not run" \
		'STAR CHART'
fi

# --- Ctrl-D ends the session on its own keystroke ---------------------
# Under cbreak() the tty does no end-of-file handling of its own, so
# Ctrl-D arrives as a character. wgetnstr returned only on Enter, so it
# used to take Ctrl-D *and* Enter to leave; every other way out of the
# game takes one keystroke.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "ctrl-d: the game never reached its command prompt"
	dump
else
	tm send-keys -t "$pane" C-d
	expect "ctrl-d: the session did not end on the keystroke alone" \
		'May the Great Bird'
fi

# --- the panel says why the grid is masked ---------------------------
# srscan() heads its grid with SHORT-RANGE SENSORS DAMAGED. The panel
# has no line to spare for a heading, so it captions the box instead;
# without it the dashes are a field of nothing with no reason given.
#
# Only in a debug build, because damaging a device on purpose is what
# the debug command is for. Everything the caption depends on is in
# tests/test_tuifmt.c, which runs in both.
if [ "$BUILD_TYPE" = "Debug" ]; then
	start 80 24 'tournament 7 short novice debug'
	if ! to_command; then
		fail "sensors: the game never reached its command prompt"
		dump
	else
		unwanted "sensors: the caption is shown with the sensors working" \
			"SENSORS DAMAGED"
		# debugme(): three questions declined, then selective damage,
		# then one answer per device. S. R. Sensors is the first.
		tm send-keys -t "$pane" 'debug' Enter
		for a in n n n y y n n n n n n n n n n n n n n n n n; do
			tm send-keys -t "$pane" "$a" Enter
			sleep 0.1
		done
		expect "sensors: the grid gives no reason for its dashes" \
			"SENSORS DAMAGED"
	fi
fi

# --- setup answered on the command line ------------------------------
# The same bug reaches further than the banner. With the setup answers
# in argv the player never types during setup at all, so the game runs
# on into the briefing -- and a pause there waits for, and eats, the
# first character of their first command.
#
# The briefing's fixed text is already longer than the window, so the
# case does not rest on the galaxy that gets generated; the seed only
# decides how much further past the threshold it goes. A pause before
# the player has touched the keyboard is wrong either way.
#
# Played in the temporary directory: the replay below answers the
# score-recording question, and answering it the other way would write
# a save file into whatever directory the game is running in.
startdir="$work"
argv_ok=""
start 80 24 'tournament 5 long emeritus xyz'
if ! wait_for 'COMMAND'; then
	fail "argv setup: the game never reached its command prompt"
	dump
else
	argv_ok=yes
	unwanted "argv setup: paused before the player had typed anything" "CONTINUE"
	unwanted "argv setup: paused before the player had typed anything" "HIT SPACE BAR"
	# Long-range scan rather than short: what it says appears only in
	# the message window, so waiting for it proves the command was
	# read whole. Sampling the screen after a fixed sleep instead
	# would pass on a slow machine that had not repainted yet.
	tm send-keys -t "$pane" 'lrscan' Enter
	expect_re "argv setup: the first command was not accepted" "$LRSCAN"
	unwanted "argv setup: the first command lost a character" "UNRECOGNIZED"
fi

# --- a second game gets its own panels -------------------------------
# Carrying the same session on through quit and into a replay. The
# panels are wired to a flag that says a game is set up; getting that
# wrong showed the next player the last player's quadrant, or an empty
# frame around a game that was running (#21).
replay_case() {
	# Up before they are down: without this the blank-panel check
	# below would also pass on a run where curses had quietly fallen
	# back to the classic display.
	expect_status "replay: the panels were not up before quitting" "Stardate"
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "replay: quit did not reach the end-of-game questions"
		dump
		return
	fi
	# The panels are blank behind these questions. Quitting is an
	# ending like any other, and the ended game has no business still
	# being framed above the score.
	wait_gone_status "replay: the quit game is still on the panels" "Stardate"
	#
	# Answered "no" deliberately: "yes" writes a save file and stops
	# to ask what to call it, stranding the test at that prompt.
	tm send-keys -t "$pane" 'n' Enter
	expect "replay: never asked about another game" 'play again'
	tm send-keys -t "$pane" 'y' Enter
	if ! wait_for 'regular, tournament, or frozen'; then
		fail "replay: the second game never started"
		dump
		return
	fi
	# And still blank through the second game's setup questions, which
	# is where the last captain's quadrant used to be framed above
	# someone else's answers (#21). setup() is what clears it.
	wait_gone "replay: the last game is still on the panels during setup" "Stardate"
	# No banner this time, so nothing has overflowed the window -- but
	# the pager stays armed from the first game, and a pause here
	# would eat this answer just the same.
	unwanted "replay: paused before the second game's first question" "CONTINUE"

	tm send-keys -t "$pane" 'regular' Enter
	expect "replay: the second game lost the first answer" 'Short, Medium, or Long'
	tm send-keys -t "$pane" 'short' Enter
	expect "replay: setup did not reach the skill question" 'Novice, Fair, Good, Expert'
	tm send-keys -t "$pane" 'novice' Enter
	sleep 0.5
	tm send-keys -t "$pane" 'xyz' Enter
	if ! to_command; then
		fail "replay: the second game never reached its prompt"
		dump
		return
	fi
	# And back on for the new one.
	want_quadtitle "replay: the quadrant panel is not showing a quadrant"
	expect_status "replay: the status panel is empty" "Stardate"
}

# Only if there is a game to quit out of; otherwise 'quit' goes into
# whatever prompt is up and buries the real failure in noise.
[ -n "$argv_ok" ] && replay_case
startdir="$srcdir"

# --- a game frozen and thawed again -----------------------------------
# Thawing is its own way into a running game: it sets the flag the
# panels key off and then prints a report that can page, all before the
# player has typed anything. Played in a temporary directory because
# freezing writes a save file next to wherever the game is running.
startdir="$work"
start 80 24 'tournament 9 short novice pw'
if ! to_command; then
	fail "freeze: the game never reached its command prompt"
	dump
else
	# Freezing says nothing on success, as it always has, so the file
	# is the only evidence there is. -s, not -f: the file exists from
	# the fopen(), but the whole 4KB payload lands in one flush at the
	# fclose(), and starting the next session kills this one. Waiting
	# for the game to answer again makes sure that has happened.
	tm send-keys -t "$pane" 'freeze sav' Enter
	i=0
	while [ "$i" -lt 40 ] && [ ! -s "$work/sav.trk" ]; do
		i=$((i + 1))
		sleep 0.1
	done
	tm send-keys -t "$pane" 'lrscan' Enter
	expect_re "freeze: the game stopped answering after freezing" "$LRSCAN"
	if [ ! -s "$work/sav.trk" ]; then
		fail "freeze: no save file was written"
	else
		start 80 24 'frozen sav'
		if ! to_command; then
			fail "thaw: the thawed game never reached its prompt"
			dump
		else
			expect_status "thaw: the panels are not showing the thawed game" "Stardate"
			want_quadtitle "thaw: the quadrant panel has no quadrant"
			# The report printed on thawing can page, and this is
			# the same before-the-player-has-typed moment that #17
			# was about.
			tm send-keys -t "$pane" 'lrscan' Enter
			expect_re "thaw: the first command was not accepted" "$LRSCAN"
			unwanted "thaw: the first command lost a character" "UNRECOGNIZED"

			# Everything else here would still pass if the panels
			# were painted once and never again, which is most of
			# what the full-screen display is for. Warp factor is
			# the cheapest thing to move: nothing else changes, no
			# time passes, nothing shoots back. Done in this game
			# rather than the emeritus one above because a setup
			# attack there can leave the warp engines too damaged
			# to accept the command at all.
			want "the warp factor is not on the status panel" "Warp Factor   5.0"
			tm send-keys -t "$pane" 'warp 4' Enter
			expect "the status panel did not follow the game" "Warp Factor   4.0"
		fi
	fi
fi
startdir="$srcdir"

# --- terminals the full-screen display cannot use --------------------
# Only a pty reaches these. The piped journey covers the no-terminal
# case; the terminfo probe and the size check need a real terminal that
# curses can ask about and still turn down. Each has to fall back to the
# classic display and keep playing rather than exit, hang, or draw a
# screen that never fills in.
#
# $1 description, $2 cols, $3 rows, $4 expected notice, $5 env prefix.
fallback() {
	start "$2" "$3" '' "$5"
	# Searched through the scrollback: the notice prints before the
	# banner, and on a 20-row terminal there is not much room between
	# them. A line added to the preamble later would push it off the
	# visible screen and turn this into a mystery.
	if ! wait_scrollback "$4"; then
		fail "$1: no notice that the full-screen display was refused"
		dump_scrollback
		return
	fi
	unwanted "$1: drew the panels on a terminal it had refused" " Quadrant "
	if ! wait_for 'regular, tournament, or frozen'; then
		fail "$1: the classic display did not carry on to a prompt"
		dump
		return
	fi
	tm send-keys -t "$pane" 'regular' Enter
	expect "$1: the classic display did not take an answer" 'Short, Medium, or Long'
}

# TERM=dumb is what Emacs' shell buffer sets: curses starts happily on
# it and then cannot address the cursor, so the game would look hung.
fallback "TERM=dumb" 80 24 "cannot do full-screen mode" 'env TERM=dumb'
# Smaller than the panels need. 72x24 is the floor; 70x20 is under it.
fallback "undersized terminal" 70 20 "Terminal too small" ''

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'tui OK\n'
