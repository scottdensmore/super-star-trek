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
	# stdout as well as stderr, because this can run from the PIPE trap,
	# where stdout is the pipe that has just been closed and a tmux writing
	# to it would take the signal itself. kill-server writes nothing there
	# today, so this is hardening and not a fix -- but a cleanup reached
	# from a PIPE trap should not depend on the state of stdout at all.
	tm kill-server >/dev/null 2>&1
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
# PIPE too, which every other script here that sets traps already had.
# Piping the output -- `head` to take the first lines, `grep -q` to ask
# whether anything failed at all -- closes the pipe early, and
# dash then dies of the signal with its EXIT trap unrun. bash runs that
# trap before it re-raises, so this leaks where /bin/sh is dash and not
# where it is bash. What is left is worse here than in the other scripts,
# where an unrun EXIT trap strands a temporary directory: this one leaves
# the private tmux server running with a game inside it, under a per-run
# socket name nobody would think to look for.
trap 'finish; exit 141' PIPE

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

# Whether every row from $2 to $3 carries the same character in column
# $1. That is how a wrapped line shows up: the panel borders run down a
# column, and text that spilled onto the next row lands on one of them.
# Which character it is does not matter -- capture-pane renders the
# line-drawing set as it pleases -- only that they agree.
#
# It answers "they agree", not "the panels are there". A screen whose
# rows are all shorter than column $1 agrees with itself: capture-pane
# strips trailing blanks, every row reads as empty, and this passes.
# The row count rules out a capture with nothing in it, not a capture
# of nothing. Prove the panels are drawn before asking.
#
# Scalar rather than `length(array)`, which POSIX awk does not have:
# CI runs this suite on macOS too, where /usr/bin/awk is the one true
# awk, and a version that could not index by array would have failed
# the assertion rather than the game.
uniform_col() {
	screen | awk -v col="$1" -v lo="$2" -v hi="$3" '
		NR >= lo && NR <= hi {
			c = substr($0, col, 1)
			if (n++ == 0) first = c
			else if (c != first) bad = 1
		}
		END { exit (n > 0 && !bad) ? 0 : 1 }'
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

# --- a shrink past the minimum clips instead of wrapping ---------------
# make_windows() promises that a terminal shrunk below the 72x24 the
# display asks for "keeps its size and the screen clips". It did not
# clip: draw_status_line() wrote the whole line a character at a time
# with nothing stopping it at the window edge, so curses wrapped what
# did not fit onto the next row of the same window -- over the left
# border and the padding column, and on the last line over the bottom
# border. At 40 columns that read as `xtsKlingons Left 2` and a bottom
# border of `     7.00qj`.
#
# 40 columns rather than 71: both wraps need the text to overrun, and
# "Time Left" is the shortest line there is, so the one that lands on
# the border only shows up well below the minimum.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "sub-minimum: the game never reached its command prompt"
	dump
else
	tm resize-window -t "$session" -x 40 -y 24
	sleep 1
	# The panels have to actually be there and actually be too narrow,
	# or everything below this passes by describing an empty screen.
	# Both captions on the top row, not the bare word: plain mode says
	# "currently in Quadrant 3 - 2" in its scrolling prose, so a game
	# that fell back to it -- TERM=dumb, no tty -- matched the panel
	# check and then had its conversation measured as though it were a
	# panel. The status caption is drawn down to exactly 40 columns and
	# dropped at 39, so this guard and the width below have to move
	# together.
	if ! screen | head -1 | grep -qF 'Quadrant' ||
	   ! screen | head -1 | grep -qF 'Status'; then
		fail "sub-minimum: both captions are not on the top row at 40 columns, where the status one is drawn down to exactly 40"
		dump
	elif [ "$(screen | head -1 | wc -c)" -gt 45 ]; then
		fail "sub-minimum: the terminal did not shrink"
		dump
	elif screen | grep -qF '2500.0 units'; then
		fail "sub-minimum: the status lines still fit, so nothing was clipped"
		dump
	else
		# Column 30 is the status panel's left border: the quadrant box
		# is 29 wide, so its own border ends at 29. Every row of the
		# panels carries the same character there. Which character is
		# not the point -- capture-pane renders the line-drawing set as
		# it pleases -- but a row that disagrees with the others is text
		# that wrapped into the border.
		if ! uniform_col 30 2 12; then
			fail "sub-minimum: a status line wrapped over the panel border"
			dump
		fi
		# Row 13 is the panels' bottom border -- they are PANELH rows
		# deep and start at the top of the screen. Nothing numeric
		# belongs on it: the only caption it ever carries is SENSORS
		# DAMAGED.
		if ! screen | awk 'NR == 13 && /[0-9]/ { bad = 1 }
		                   END { exit bad ? 1 : 0 }'; then
			fail "sub-minimum: a status line wrapped over the bottom border"
			dump
		fi
		# A clipped line spends its last column on a marker, so what is
		# left cannot be read as a whole smaller value -- `Torpedoes
		# 1` for ten of them was what clipping in silence looked like.
		# Column 39 is the last one the text gets at this width: the
		# status window starts at 30 and its right border is column 40.
		# Every line here is longer than the eight columns of room, so
		# every one of the ten carries the marker.
		if [ "$(screen | awk 'NR >= 3 && NR <= 12 && substr($0, 39, 1) == ">"' | wc -l)" -ne 10 ]; then
			fail "sub-minimum: a clipped status line does not say it was clipped"
			dump
		fi
	fi

	# Narrower still, where the panel's own title no longer fits. It is
	# the one string not drawn a character at a time, so the clipping
	# above does not cover it, and " Status " wrapped onto the row
	# beneath and sat on its border exactly as the status lines had.
	# The check above cannot see it: at 40 columns the title still fits.
	tm resize-window -t "$session" -x 33 -y 24
	sleep 1
	if ! screen | head -1 | grep -qF 'Quadrant'; then
		fail "sub-minimum: the quadrant panel went missing at 33 columns"
		dump
	elif ! uniform_col 30 2 12; then
		fail "sub-minimum: the status title wrapped over the panel border"
		dump
	fi

	# Widening puts it all back, which is what the README promises a
	# player who dragged too far. Checked here rather than at the end
	# of the sweep so that what it proves is this section's own
	# clipping being undone, and not the geometry restore that the
	# "panels come back from a squeeze" block covers on its own.
	tm resize-window -t "$session" -x 80 -y 24
	sleep 1
	if ! screen | grep -qF '2500.0 units'; then
		fail "sub-minimum: widening did not bring the clipped lines back"
		dump
	fi
	# Asked separately from the line above, not as its else: a repaint
	# that brought the text back and left the markers behind is exactly
	# the failure worth catching, and chaining the two hides it behind
	# the one that passed.
	if ! screen | awk 'NR >= 2 && NR <= 12 && />/ { bad = 1 }
	                   END { exit bad ? 1 : 0 }'; then
		fail "sub-minimum: a clip marker survived the widening"
		dump
	fi

	# The quadrant panel's own titles. The 33 above cannot reach them:
	# that window is still its full 29 wide there, so only the status
	# title is under any pressure. Column 1 from here on -- it is the
	# quadrant box's left border that a wrapped quadrant title lands
	# on -- from a title that did not fit, or from a grid line, which
	# clips against the same edge once the window has shrunk with the
	# screen.
	#
	# Last, because this is below the geometry the panels are built for.
	# The panels themselves come back from that now, but what a squeeze
	# that deep leaves of the pending prompt does not (#100), so a
	# session that has been there is not the clean subject the widths
	# above want. Each width is asserted
	# on both sides of its threshold: a guard that gives up a column
	# early looks exactly like one that gives up on time.
	#
	# The coordinates need 19 columns.
	tm resize-window -t "$session" -x 19 -y 24
	want_quadtitle "sub-minimum: the quadrant went unnamed while it still fitted"
	tm resize-window -t "$session" -x 18 -y 24
	sleep 1
	# uniform_col agrees with itself on a screen that drew nothing, so
	# every call below needs the panels shown to be there first. The
	# bare " Quadrant " stands in from here down to 13.
	if screen | head -1 | grep -qE 'Quadrant [0-9] - [0-9]'; then
		fail "sub-minimum: the coordinates stayed past the room for them"
		dump
	elif ! screen | head -1 | grep -qF 'Quadrant'; then
		fail "sub-minimum: nothing stood in for the title at 18 columns"
		dump
	elif ! uniform_col 1 2 12; then
		fail "sub-minimum: text wrapped over the quadrant panel border"
		dump
	fi
	# Sixteen is not a threshold, and is here for what only it shows:
	# at 18 the in-game title is exactly as wide as the window, so an
	# unguarded one overwrites the right border without ever wrapping,
	# and the border check below cannot see it -- the regex above is
	# what catches that. Three columns narrower it genuinely wraps,
	# which is the fault this whole section is about.
	tm resize-window -t "$session" -x 16 -y 24
	sleep 1
	if ! screen | head -1 | grep -qF 'Quadrant'; then
		fail "sub-minimum: the panels went missing at 16 columns"
		dump
	elif ! uniform_col 1 2 12; then
		fail "sub-minimum: text wrapped over the quadrant panel border"
		dump
	fi
	# And the bare title itself needs 13, below which the box carries
	# no caption at all.
	tm resize-window -t "$session" -x 13 -y 24
	sleep 1
	if ! screen | head -1 | grep -qF 'Quadrant'; then
		fail "sub-minimum: the panel went unlabelled while the label fitted"
		dump
	fi
	tm resize-window -t "$session" -x 12 -y 24
	sleep 1
	if screen | head -1 | grep -qF 'Quadrant'; then
		fail "sub-minimum: the panel label stayed past the room for it"
		dump
	elif ! screen | sed -n '2p' | grep -q '[0-9]'; then
		fail "sub-minimum: the panels are not drawn at 12 columns"
		dump
	elif ! uniform_col 1 2 12; then
		fail "sub-minimum: the panel label wrapped over the panel border"
		dump
	fi
fi

# --- a shrink past the panels' height clips too ------------------------
# The width sweep above is only half of it. The panels are PANELH rows
# deep, and a terminal shorter than that shrinks them with it -- but the
# draw loops were told which row to write on by the caller and never
# asked whether that row still existed. Two ways to fail, one on each
# side of the last row: at twelve rows the tenth grid line was drawn
# onto the bottom border, and at eleven wmove() failed outright, left
# the cursor where it was, and the line landed on the row above --
# `mq 9  . . . . . . . . . .10 .x`, two grid rows and the border in one.
#
# Twelve and eleven because they are those two cases; nine for a shrink
# well past both.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "short: the game never reached its command prompt"
	dump
else
	# Before shrinking anything: at a size with room for all of them,
	# every row is still drawn. The sweep below only asks that nothing
	# lands where it should not, which a bound that clips two rows too
	# many satisfies perfectly -- at every size, including this one.
	tm resize-window -t "$session" -x 80 -y 24
	sleep 1
	# `^.` rather than `^x`: the leading cell is the panel's left
	# border, and capture-pane renders the line-drawing set as it
	# pleases -- the rest of this file is careful not to depend on it.
	# The two spaces are what tell a grid row from the column header,
	# which has one; without them ` 10  ` also matches the header and
	# `Torpedoes     10`.
	if ! screen | grep -qE '^. 10  '; then
		fail "short: the last grid row is missing at 80x24"
		dump
	# Unconfined, which is only safe because `Time Left` appears
	# nowhere else on this screen -- not in the briefing, not at the
	# prompt. A `status` command anywhere in this block would put it in
	# the conversation too, and this would need `cut -c30-`.
	elif ! screen | grep -qF 'Time Left'; then
		fail "short: the last status line is missing at 80x24"
		dump
	fi
	for h in 12 11 9; do
		tm resize-window -t "$session" -x 72 -y "$h"
		sleep 1
		# The panels fill a screen this short, so the last row of it
		# is their bottom border. Nothing numeric belongs there --
		# the only caption it ever carries is SENSORS DAMAGED -- and
		# a grid line that landed on it brings its row label along.
		if ! screen | head -1 | grep -qF 'Quadrant'; then
			fail "short: the panels are not on screen at 72x$h"
			dump
		elif ! screen | awk -v h="$h" 'NR == h && /[0-9]/ { bad = 1 }
		                               END { exit bad ? 1 : 0 }'; then
			fail "short: a panel line was drawn onto the bottom border at 72x$h"
			dump
		fi
	done
	# The wmove() failure put two grid rows on one line, which the
	# border check above catches only because that line was the
	# border. Asked for directly as well, so the assertion does not
	# depend on where the collision happened to land: no row of the
	# panels carries two grid labels.
	tm resize-window -t "$session" -x 72 -y 11
	sleep 1
	# Two row labels on one line. Told apart from the column header by
	# the spacing of the label itself: fmt_quad_line() writes "%2d "
	# and then " %c" per cell, so a grid row has two spaces after its
	# number where the header has one. Nothing here depends on what is
	# in the quadrant -- an earlier version ended the pattern at a grid
	# cell it expected to be a dot, which this seed provides and
	# another galaxy would not.
	#
	# Confined to the quadrant panel's columns, which is what lets the
	# pattern end at the second label: the status panel beside it is
	# full of digits. Twenty-nine is QUADW, which that panel keeps at
	# any width at or above it, and this only ever runs at 72.
	if screen | cut -c1-29 | grep -qE '^.q? *[0-9]+  .*[^0-9]1?[0-9]'; then
		fail "short: two grid rows were drawn onto one line"
		dump
	fi
fi

# --- the panels come back from a squeeze ------------------------------
# make_windows() used to resize only the status and message windows,
# never the quadrant panel: it was made once and left to curses, which
# resizes a window that spans the screen along with the screen -- down
# and back up again. So a panel squeezed past its own size stopped being
# 29 columns and started tracking the terminal, and a squeeze returned
# from came back broken at a size where nothing on screen explained why.
#
# Two shapes, one cause. Squeezed narrow, the quadrant box came back as
# wide as the whole screen, so its right border was past the status
# panel rather than beside it. Squeezed short, it came back taller than PANELH,
# so its bottom border ended up at the bottom of the screen instead of
# row 13, with the message window and the prompt hidden under it.
#
# Two of the four sizes are for coverage and two are for diagnosis.
#
# 20x8 squeezes both axes at once, which is the gesture a player makes,
# and catches everything 25x24 and 80x13 do. 82x13 is the one nothing
# else reaches: it returns to a terminal *narrower* than the squeeze,
# and mvwin() refuses a move that does not fit at the window's current
# size, so a message window still 80 columns wide could not be put back
# into an 80-column screen. Every other case grows back at least as
# wide as it shrank and hides that.
#
# 25x24 and 80x13 catch nothing 20x8 misses. They are here to say which
# axis broke -- 20x8 fires on both at once, and a report naming the
# width alone or the height alone is worth two lines of script. Drop
# them only if that stops being worth it, not because they look
# redundant.
#
# 30 columns and 14 rows still recovered before the fix; 29 and 13 are
# the first that did not, so 13 is the boundary itself and 25 is a few
# columns clear of the other one.
#
# A session of its own for each, because the damage outlives the
# squeeze that caused it: run back to back, the second squeeze starts
# from a display the first already broke, and whichever assertion fires
# is not the one that describes it.
for squeeze in "25 24" "80 13" "20 8" "82 13"; do
	# shellcheck disable=SC2086
	set -- $squeeze
	start 100 30 'tournament 7 short novice pw'
	if ! to_command; then
		fail "restore: the game never reached its command prompt"
		dump
	else
		tm resize-window -t "$session" -x "$1" -y "$2"
		sleep 1
		tm resize-window -t "$session" -x 80 -y 24
		sleep 1
		# The panels have to be drawn at all before their corners
		# mean anything: a screen with nothing on it agrees with
		# itself about every column.
		if ! screen | head -1 | grep -qF 'Quadrant'; then
			fail "restore: after $1x$2 the panels are not on screen to be checked"
			dump
			continue
		fi
		# The corners are what say the boxes are two boxes. Asked as
		# "differs from its neighbour" rather than by naming a glyph,
		# because capture-pane renders the line-drawing set as it
		# pleases: a corner differs from the border running into it,
		# and a border that lost its corner does not. Column 28 has
		# to be drawn as well -- two blanks differ from nothing and
		# agree with each other, so without that a panel narrower
		# than 28 would be reported as one that came back too wide.
		if ! screen | awk 'NR == 1 { exit (substr($0, 28, 1) == " " ||
		                                   substr($0, 29, 1) == substr($0, 28, 1)) }'; then
			fail "restore: after $1x$2 the quadrant panel came back the wrong width"
			dump
		fi
		if ! screen | awk 'NR == 12 { above = substr($0, 1, 1) }
		                   NR == 13 { exit substr($0, 1, 1) == above }'; then
			fail "restore: after $1x$2 the quadrant panel came back too tall"
			dump
		fi
		# And the conversation is back under the panels, which is the
		# half a player notices first. Asked by typing rather than by
		# looking, which was once because the pending line came back
		# a stump -- #100, fixed, and the block below now asks by
		# looking for exactly that reason. Typing still asks a
		# different question than looking does, and the one this
		# block wants: a fresh prompt has to *land* somewhere, and
		# where it lands is what the message window being back in
		# its place means -- row 14 is the first under panels that
		# end at 13.
		tm send-keys -t "$pane" Enter
		sleep 1
		if ! screen | awk 'NR >= 14 && /COMMAND>/ { found = 1 }
		                   END { exit found ? 0 : 1 }'; then
			fail "restore: after $1x$2 the conversation did not come back under the panels"
			dump
		fi
	fi
done

# --- the pending prompt comes back from a squeeze ---------------------
# The block above asks by typing, because what a squeeze left of the
# line the game was part way through was a stump. This asks by looking,
# which is what a player does: they squeeze a window, drag it back, and
# read what is on screen before touching the keyboard.
#
# sync_size() reprinted that line only when the terminal shrank. The
# shrink-time write went into a window that could be one row tall and
# narrower than the line, so it wrapped, scrolled, and left the tail --
# and growing back did nothing to repair it. From 20x8 the first setup
# question came back as ` , or frozen game?`, and a pending pause as
# ` CONTINUE]`.
#
# Worse than cosmetic where the stump is a pause: it is still pending,
# so the first keystroke of the next command is eaten by it. Typing
# `srscan` into that stump fed `rscan` to the parser and the game
# answered with the whole command list.
#
# Asked at the first setup question because it is pending with no
# keystrokes spent to get there, and because the answer is read by the
# same reader that serves every later prompt. The opening pause reaches
# the same code, but the setup answers this suite passes on the command
# line run straight past it to the command prompt.
#
# And asked again at a pager pause, which is the half of #100 that did
# more than look wrong: the pause is still waiting, so the first
# keystroke of whatever you type next is eaten by it. `chart` fills the
# window and stops, which is the one pause reachable without spending a
# keystroke to reach it.
#
# Asked as "exactly one", not "at least one". A stump fails it by
# absence, and so does the other direction: reprinting without erasing
# is what once stood eight copies of a question up the window, and a
# check for presence alone would pass that.
#
# 20x8 is the corner drag, the commonest gesture and the one #100's
# table was measured from. 10x8 is past every minimum the display has,
# and the stump is shorter still: a build without the fix leaves
# ` game?` of the question and ` UE]` of the pause. #100's table says
# "nothing at all" at that size, which is what the pre-fix build does
# not do -- measured, and corrected on the issue.
#
# The panels recover at both sizes since #88, so the conversation is
# the only thing left that does not.
for squeeze in "20 8" "10 8"; do
	# shellcheck disable=SC2086
	set -- $squeeze
	start 80 24
	if ! wait_for 'regular, tournament, or frozen'; then
		fail "pending: the game never asked its first question"
		dump
	else
		tm resize-window -t "$session" -x "$1" -y "$2"
		sleep 1
		tm resize-window -t "$session" -x 80 -y 24
		sleep 1
		# No keystroke between the resize and the look. One would
		# answer the question and take it off screen.
		if ! screen | awk '/Would you like a regular, tournament, or frozen game\?/ { n++ }
		                   END { exit n == 1 ? 0 : 1 }'; then
			fail "pending: after $1x$2 the question is not on screen exactly once"
			dump
		fi
	fi

	start 80 24 'tournament 7 short novice pw'
	if ! to_command; then
		fail "pending: the game never reached its command prompt"
		dump
	else
		tm send-keys -t "$pane" chart Enter
		if ! wait_for 'HIT SPACE BAR TO CONTINUE'; then
			fail "pending: chart did not fill the window and stop"
			dump
		else
			tm resize-window -t "$session" -x "$1" -y "$2"
			sleep 1
			tm resize-window -t "$session" -x 80 -y 24
			sleep 1
			if ! screen | awk '/\[HIT SPACE BAR TO CONTINUE\]/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "pending: after $1x$2 the pause is not on screen exactly once"
				dump
			fi
			# A control, not a second check of #100: the pause is
			# still pending either way, so this passes against a
			# build with the defect too. It says the assertion
			# above was taken at a real pause rather than at
			# something that merely reads like one, and it would
			# fail a fix that put the line back by re-prompting
			# and swallowing the pause -- which is the obvious
			# wrong way to make the check above pass.
			tm send-keys -t "$pane" Space
			sleep 1
			if screen | grep -qF 'HIT SPACE BAR TO CONTINUE'; then
				fail "pending: after $1x$2 Space did not page the chart on"
				dump
			fi
		fi
	fi
done

# --- a wrapping answer survives a grow --------------------------------
# restore_curline() is called on a grow now, which it was not before
# #100. That was excluded once for a reason: a grow reprinted a
# question whose answer had wrapped, because the search that looks for
# the pair already on screen read a single row and a wrapped pair is
# two. It never matched, so it reprinted, and the copies stacked up the
# window. The joined-row search is what makes the grow call safe, and
# nothing else in this suite exercises it on a grow.
#
# Height only, at a fixed width, which is the shape that duplicates.
# A width change takes the other branch, which works out how many rows
# the stump occupies and erases them before writing -- so a missed
# match there costs a rewrite, not a copy. Only the height-only branch
# scrolls by one and writes without erasing, leaving what was already
# there. Cutting the search back to one row -- `int start = r;` --
# gives 2 copies, then 3, then 4 over the three steps below, and passes
# every other check in this file.
start 72 24
if ! wait_for 'regular, tournament, or frozen'; then
	fail "wrap: the game never asked its first question"
	dump
else
	# The question is 53 columns and the window 70, so an answer this
	# long puts the pair over two rows and into the joined path.
	tm send-keys -t "$pane" 'regularbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
	sleep 1
	for h in 30 36 42; do
		tm resize-window -t "$session" -x 72 -y "$h"
		sleep 1
		if ! screen | awk '/Would you like a regular, tournament, or frozen game\?/ { n++ }
		                   END { exit n == 1 ? 0 : 1 }'; then
			fail "wrap: grown to 72x$h the question is not on screen exactly once"
			dump
			break
		fi
		# Not asserting the answer here, though it is the half the
		# player is looking at. Two attempts at it were blind: a
		# per-row grep for the run of b's could never match, because
		# the answer is what wraps and the run is split across two
		# rows; and joining the rows back does not reassemble it
		# either, because what capture-pane returns for the two rows
		# does not concatenate to what was typed. Counting b's over
		# the whole screen needs a baseline, since the banner has
		# its own. The question alone kills the mutant this block is
		# for, so the gap is coverage rather than a hole -- issue
		# #115.
	done
fi

# --- a prompt that ended its own line comes back too -------------------
# #100 put back the line the game was part way through. A prompt that
# finished its line before it stopped to wait is the other half: prout
# appends the newline, note_out() clears curline there, and
# restore_curline() returns at once with nothing to put back. So the
# question vanishes entirely while the reader goes on waiting behind
# it, and the next keystroke is eaten by a prompt that is not on
# screen. `rest 5000` is the shortest way to one -- events.c prouts
# "Are you sure? " and then reads with ja().
#
# 20x8 for the corner drag; 80x14 for a height-only squeeze, which
# reaches the other placement branch.
# 80x23 is the one that matters for the duplication guard below. Both
# regressions it was written for reproduce only on a one-row height
# shrink: at 20x8 and 80x14 the count is 0 or 1 whatever the build, so
# the guard was decorative until this size was added.
for squeeze in "20 8" "80 14" "80 23"; do
	# shellcheck disable=SC2086
	set -- $squeeze
	start 80 24 'tournament 7 short novice pw'
	if ! to_command; then
		fail "ended: the game never reached its command prompt"
		dump
	else
		tm send-keys -t "$pane" 'rest 5000' Enter
		if ! wait_for 'Are you sure'; then
			fail "ended: rest did not ask whether it was wise"
			dump
		else
			tm resize-window -t "$session" -x "$1" -y "$2"
			sleep 1
			tm resize-window -t "$session" -x 80 -y 24
			sleep 1
			if ! screen | awk '/Are you sure\?/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "ended: after $1x$2 the question is not on screen exactly once"
				dump
			fi
			# The reader is still the one waiting, and it is a
			# y/n reader: N has to be taken as the answer rather
			# than as the first letter of a command. A control --
			# it passes against a build with the defect too, and
			# would fail a "fix" that reprinted the question by
			# re-asking it and left two readers stacked.
			# Counted, not looked for: at a one-row height
			# shrink the echoed `COMMAND> rest 5000` survives,
			# so COMMAND> is already on screen before N is sent
			# and a presence check cannot fail there. What N
			# produces is one more of them.
			before=$(screen | grep -c 'COMMAND>')
			tm send-keys -t "$pane" N Enter
			sleep 1
			after=$(screen | grep -c 'COMMAND>')
			if [ "$after" -le "$before" ]; then
				fail "ended: after $1x$2 N did not answer the question"
				dump
			fi
		fi
	fi

	# And once the answer is part typed, both come back -- #117 --
	# which is asserted as the pair and not just the question, since
	# the answer is the half that ends games: at 80x23 the question
	# used to return without it, and a screen showing a clean
	# unanswered question is what makes a player type over a `y` they
	# cannot see. The count stays exact because both ways of spanning
	# the row boundary between an ended prompt and its answer
	# duplicated it instead: once per width step one way, once on a
	# one-row height shrink the other.
	#
	# Its own session. Run on from the block above, the first `rest`
	# has left its own answered question in the window, so two are on
	# screen before this one starts and the count means nothing.
	start 80 24 'tournament 7 short novice pw'
	if ! to_command; then
		fail "ended: the game never reached its command prompt to type at"
		dump
	else
		tm send-keys -t "$pane" 'rest 5000' Enter
		if ! wait_for 'Are you sure'; then
			fail "ended: rest did not ask whether it was wise"
			dump
		else
			tm send-keys -t "$pane" y
			sleep 1
			tm resize-window -t "$session" -x "$1" -y "$2"
			sleep 1
			tm resize-window -t "$session" -x 80 -y 24
			sleep 1
			# Both halves, and both of them matter. The question
			# exactly once is the duplication guard the two
			# failed attempts needed. The answer under it is
			# #117 itself: at 80x23 the question came back
			# without it, which is the shape that ends games --
			# the screen reads as a fresh unanswered question
			# while the reader still holds the `y`, so a typed
			# `N` submits `yN` and ja() takes the y.
			if ! screen | awk '/Are you sure\?/ { n++; q = NR }
			                   { row[NR] = $0 }
			                   END {
			                       if (n != 1) exit 1
			                       a = row[q + 1]
			                       gsub(/^[ \t]+|[ \t]+$/, "", a)
			                       exit (a == "y") ? 0 : 1
			                   }'; then
				fail "ended: after $1x$2 the question and its part-typed answer are not both back, once each"
				dump
			fi
		fi
	fi
done

# --- and a wrapping answer to an ended prompt ---------------------------
# The block above types one character, so it only ever exercises an
# answer of a single row. An ended prompt's answer has a row of its
# own, and the read-back has to step past all of them: allowing it
# exactly one row puts the question outside the rows the search joins,
# the match misses, and the height-only branch scrolls and writes
# without erasing -- a fresh copy of question and answer at every step.
# Three of them up the window over the drag below, with the conversation
# pushed off the top, which is the failure the old #117 gate existed to
# prevent and the reason this case is asserted separately.
#
# 87 characters at 80 columns: the message window is 78, so the answer
# takes two rows and the pair takes three. ja() reads the first letter
# of the first word, so a run of b's after the y is still a valid
# pending answer.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "wrapped answer: the game never reached its command prompt"
	dump
else
	tm send-keys -t "$pane" 'rest 5000' Enter
	if ! wait_for 'Are you sure'; then
		fail "wrapped answer: rest did not ask whether it was wise"
		dump
	else
		tm send-keys -t "$pane" 'ybbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
		sleep 1
		# That the answer really did wrap, before dragging anything.
		# The block exists for the case where it takes more than one
		# row, and the question count alone passes just as well on a
		# screen with no answer on it at all -- so a send-keys that
		# a later edit shortened, or that the game stopped taking,
		# would leave this asserting the one-row case it was added
		# to go beyond. The answer starts at column 0 of its own
		# row here, so unlike the wrapping block above it can be
		# read a row at a time.
		#
		# Proved by shortening the literal above to 'ybbb': both
		# this and the check after the drag fail, where the
		# question count alone stays green -- which is the whole
		# of the degradation this guards against.
		if ! screen | awk '/Are you sure\?/ { q = NR }
		                   { row[NR] = $0 }
		                   END {
		                       if (q == 0) exit 1
		                       a = row[q + 1]; b = row[q + 2]
		                       gsub(/^[ \t]+|[ \t]+$/, "", a)
		                       gsub(/^[ \t]+|[ \t]+$/, "", b)
		                       exit (a ~ /^yb+$/ && b ~ /^b+$/) ? 0 : 1
		                   }'; then
			fail "wrapped answer: the answer did not wrap, so the drag below proves nothing"
			dump
		fi
		for h in 23 24 23 24; do
			tm resize-window -t "$session" -y "$h"
			sleep 1
			if ! screen | awk '/Are you sure\?/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "wrapped answer: at 80x$h the question is not on screen exactly once"
				dump
				break
			fi
		done
		# Width as well as height, because they take different
		# branches and only this one reaches the erase. A width
		# change works out how many rows the stump occupies and
		# rubs them out before writing, and for an ended line that
		# count has to include the padding between the question and
		# its answer -- drop that and the erase leaves a row behind,
		# so the drag stacks one more question at every step: 2, 3,
		# 4, 5 across the four widths below. The height loop above
		# cannot see it, since its branch scrolls instead of
		# erasing, and the one-character answer in the block before
		# never mangles the pair at all.
		#
		# 72 is the narrowest the display will start at -- the drag
		# would work below it, nothing re-checks the floor -- and
		# the answer still wraps there: the window is 70, so 70
		# and 17.
		#
		# Proved against a copy of the tree outside the repository
		# with that padding line deleted: it fails here at the
		# first width step, where before this loop existed it left
		# the whole suite green.
		for w in 76 72 76 80; do
			tm resize-window -t "$session" -x "$w"
			sleep 1
			if ! screen | awk '/Are you sure\?/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "wrapped answer: at ${w}x24 the question is not on screen exactly once"
				dump
				break
			fi
		done
		# The state both documents describe to the player, asserted
		# where they describe it rather than only after recovery.
		# sst.doc and README.md promise that a window too short to
		# hold the pair keeps the answer and loses the question --
		# and every other check here looks at the screen only after
		# growing back, so a change that kept the question and
		# dropped the answer would leave both documents wrong with
		# the suite green, and a player typing over a `y` they
		# cannot see. Two message rows at fifteen, which is exactly
		# the answer and no room for the question above it.
		#
		# Proved against a broken game, which is what step 4 asks of
		# a test that runs one: `maxy > 1` -> `maxy > 2` on the
		# newline guard in restore_curline() suppresses the newline
		# only at a two-row window, so the pair packs onto the two
		# rows -- `Are you sure? ybbb...` over `bbb...`, the
		# question kept and the answer displaced. Built in a copy
		# outside the repository, that fails here and nowhere else
		# in this file.
		tm resize-window -t "$session" -y 15
		sleep 1
		if ! screen | awk '/Are you sure\?/ { q++ }
		                   /^ *yb+$/ { a++ }
		                   /^ *b+$/ { c++ }
		                   END { exit (q == 0 && a == 1 && c == 1) ? 0 : 1 }'; then
			fail "wrapped answer: at 80x15 the answer did not survive without its question"
			dump
		fi
		tm resize-window -t "$session" -y 24
		sleep 1
		# And whole again at the end, which is what the drag was for:
		# the question count alone never looks at the half a player
		# is typing.
		if ! screen | awk '/Are you sure\?/ { q = NR }
		                   { row[NR] = $0 }
		                   END {
		                       if (q == 0) exit 1
		                       a = row[q + 1]; b = row[q + 2]
		                       gsub(/^[ \t]+|[ \t]+$/, "", a)
		                       gsub(/^[ \t]+|[ \t]+$/, "", b)
		                       exit (a ~ /^yb+$/ && b ~ /^b+$/) ? 0 : 1
		                   }'; then
			fail "wrapped answer: after the drag the answer is not back whole under its question"
			dump
		fi
		# The control every other block here has, and the one this
		# one most needs: the assertions above read the screen, and
		# the claim the fix makes -- the one sst.doc now makes to
		# the player -- is that what is on screen is what the game
		# will act on. Only Enter proves the reader still holds the
		# `y` the restore put back. This block is the one where the
		# restore does the most work, moving into the middle of the
		# window and relying on the echo to rewrite the answer
		# there.
		#
		# Proved by sending C-u before the Enter, which clears the
		# reader without touching the screen: this fires, where
		# every assertion above it stays green.
		tm send-keys -t "$pane" Enter
		if ! wait_for 'It is stardate'; then
			fail "wrapped answer: the restored answer was not what the reader held"
			dump
		fi
	fi
fi

# --- and a terminal too wide to read a row of --------------------------
# The joined-row search needs every row it appends to be exactly the
# window's width, since that is what makes its arithmetic mean columns.
# mvwinnstr() reads at most its buffer, so past that a row comes back
# short, the rows stop lining up, and the search misses a pair that is
# perfectly intact -- after which the write path stacks a copy at every
# step. An ended line with an answer is the only thing that spans two
# rows at this width, which is why nothing reached it before.
#
# 600 columns, well past the 511 the buffer holds. Deleting the guard
# leaves every other check in this file green: nothing else here runs
# wider than 82 columns. Proved that way -- a copy of the tree outside
# the repository with the guard deleted, built there, fails at the
# first step here and nowhere else.
start 600 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "wide: the game never reached its command prompt"
	dump
else
	tm send-keys -t "$pane" 'rest 5000' Enter
	if ! wait_for 'Are you sure'; then
		fail "wide: rest did not ask whether it was wise"
		dump
	else
		tm send-keys -t "$pane" 'y'
		sleep 1
		# The control this block cannot do without. The guard only
		# fires for an ended line with something typed, so with no
		# pending answer nothing below exercises it: the question
		# restores through the ordinary path and the keystroke
		# lands after it, and both checks hold for the wrong
		# reason. Proved by replacing the send-keys above with a
		# no-op, which leaves the whole suite green -- and with the
		# control here, that same no-op fails on this line instead.
		if ! screen | awk '/Are you sure\?/ { q = NR }
		                   { row[NR] = $0 }
		                   END {
		                       if (q == 0) exit 1
		                       a = row[q + 1]
		                       gsub(/^[ \t]+|[ \t]+$/, "", a)
		                       exit (a == "y") ? 0 : 1
		                   }'; then
			fail "wide: the answer never reached the reader, so the drags below prove nothing"
			dump
		fi
		for h in 23 24 23; do
			tm resize-window -t "$session" -y "$h"
			sleep 1
			if ! screen | awk '/Are you sure\?/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "wide: at 600x$h the question is not on screen exactly once"
				dump
				break
			fi
		done
		# And the cursor with it. The guard has to sit above the row
		# scan, which moves the cursor as it reads: returning past
		# it parks the next keystroke at column 0, so a typed
		# character lands on the question's first letter --
		# `Zre you sure?`.
		#
		# Column 0 is the whole of what this distinguishes. At this
		# width nothing is restored either way, so the keystroke
		# lands on the question's second column regardless and the
		# screen reads `AZe you sure?` on a correct build and on
		# main alike. That is the state to match; a `Z` in column 0
		# is the misplaced guard and nothing else.
		tm send-keys -t "$pane" 'Z'
		sleep 1
		if screen | grep -q 'Zre you sure'; then
			fail "wide: the guard ran after the row scan and moved the cursor to column 0"
			dump
		fi
	fi
fi

# Looked at *during* the drag, not after it. At a one-row message
# window -- fourteen rows or fewer -- both of this round's fixes
# are invisible from the far end: the display recovers once the
# terminal is back at 80x24, so a check that only looks there
# passes against builds that lose the question at every other
# step of the way.
#
# Two breaks it catches, both found by review and neither seen by
# anything else in this file. Restoring the unconditional wscrl on
# the found path discards the prompt it has just matched, so the
# question alternates 0, 1, 0, 1 as the width steps. Relaxing the
# maxy > 1 guard on the write path scrolls the prompt off the only
# row there is, so it is 0 at every step.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "ended: the game never reached its command prompt at one row"
	dump
else
	tm send-keys -t "$pane" 'rest 5000' Enter
	if ! wait_for 'Are you sure'; then
		fail "ended: rest did not ask at a one-row window"
		dump
	else
		# Into the one-row window first, then along it.
		# Starting there instead sees neither break: the
		# height change is what leaves the prompt in the
		# state the width steps then mishandle.
		tm resize-window -t "$session" -x 80 -y 14
		sleep 1
		for w in 78 76 74 72; do
			tm resize-window -t "$session" -x "$w" -y 14
			sleep 1
			if ! screen | awk '/Are you sure\?/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "ended: at ${w}x14 the question is not on screen exactly once"
				dump
				break
			fi
		done
	fi
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
		# The caption is the fourth string in the panels written
		# whole rather than a cell at a time, so it needs the same
		# room asked for before it is drawn. Seventeen columns
		# inside a box that is only twenty-nine while the terminal
		# has room for it: below twenty the caption overran the
		# bottom-right corner and drew over it. Gone is the right
		# answer -- the dashes lose their explanation, but at that
		# width so has everything else.
		# Twenty columns is where it fits exactly, nineteen where it
		# does not. Both, so a guard that is too strict fails here
		# rather than passing quietly: dropping the caption a column
		# early looks the same as dropping it correctly unless the
		# last width it survives is asserted too.
		tm resize-window -t "$session" -x 20 -y 24
		sleep 1
		expect "sensors: the caption went early, before the room ran out" \
			"SENSORS DAMAGED"
		tm resize-window -t "$session" -x 19 -y 24
		sleep 1
		unwanted "sensors: the caption overran the box it captions" \
			"SENSORS DAMAGED"
		# Nineteen columns is under the twenty-nine the panels are
		# built for. What this asserts is only that the caption
		# returns once there is room for it, which is what the guard
		# above governs; that the panel itself comes back from a
		# squeeze that deep is the "panels come back from a squeeze"
		# block's job.
		tm resize-window -t "$session" -x 80 -y 24
		sleep 1
		expect "sensors: the caption did not come back with the room for it" \
			"SENSORS DAMAGED"
		# The caption belongs to the bottom border wherever the box
		# has ended up, not to the row PANELH names. A terminal too
		# short for the panel moves that border up, and naming the
		# row lost the caption from twelve rows down -- the field of
		# dashes with no reason given that it exists to prevent.
		tm resize-window -t "$session" -x 80 -y 11
		sleep 1
		# Asked of the row rather than of the screen: the fix is
		# about which row the caption lands on, and a whole-screen
		# search would pass a build that drew it over a grid line.
		# Eleven rows of panel, so its bottom border is row 11.
		if ! screen | awk 'NR == 11 && /SENSORS DAMAGED/ { seen = 1 }
		                   END { exit seen ? 0 : 1 }'; then
			fail "sensors: the dashes lost their reason on a short terminal"
			dump
		fi
		# The far end of the same question. Two rows leave a title and
		# a border, and the caption belongs on the border; one row
		# leaves only the title, and writing the caption over it is
		# what the height guard exists to stop. Nothing else reaches
		# these sizes -- the sweep above stops at nine rows.
		tm resize-window -t "$session" -x 80 -y 2
		sleep 1
		if ! screen | awk 'NR == 2 && /SENSORS DAMAGED/ { seen = 1 }
		                   END { exit seen ? 0 : 1 }'; then
			fail "sensors: the caption is not on the border of a two-row panel"
			dump
		fi
		tm resize-window -t "$session" -x 80 -y 1
		sleep 1
		# The positive control first: absence proves nothing on a
		# screen where nothing was painted, and this assertion is
		# nothing but an absence. It cannot ask for the title --
		# the caption is drawn at the same row and column, so a
		# build with the guard relaxed erases the title with the
		# caption, fails the control, and never reaches the line
		# that names the defect. Non-blankness is all the control
		# needs, and nothing can overwrite it.
		if ! screen | head -1 | grep -q '[^ ]'; then
			fail "sensors: nothing at all was drawn at one row"
			dump
		elif screen | grep -qF 'SENSORS DAMAGED'; then
			fail "sensors: the caption was drawn over the only row there is"
			dump
		fi
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
