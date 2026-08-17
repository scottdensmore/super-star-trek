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

# tmux hands the client's environment to the pane, so an exported LINES
# or COLUMNS in whoever is running this reaches the game -- and curses
# believes those over the terminal. The panels would then be sized to a
# number that has nothing to do with the pane: measured with
# COLUMNS=190 exported, before #164 refused such a size, the "half
# grown" case below saw `Terminal is 190x22` where it asserts 80x22 and
# the "regrown" case drew 190-column panels inside a 100-column pane,
# never reaching a prompt. Since #164 that leak refuses instead, so the
# panel cases would fail on their panels being absent rather than
# broken -- a different failure, the same cause. The cases below that
# want these variables set them through start's env prefix, so clearing
# them here costs nothing. Before the first tm call, because
# the tmux server captures its environment when it is created.
unset LINES COLUMNS

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
# Piping the output -- `head` to keep what broke and drop the pane behind
# it, `grep -q` to ask whether anything did -- closes the pipe early, and
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

# How many lines of the scrollback carry $1. For a line the retry
# repeats word for word, presence proves nothing: the startup notice
# already put one copy there, so it is the second that is the retry's.
scrollback_count() {
	scrollback | grep -cF -- "$1"
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

# Wait for column $1 to be blank on every row from $2 to $3, up to about
# four seconds. Polled rather than asked once, for the reason wait_gone
# gives: a resize is not instantaneous, and a capture taken before the
# game has repainted still holds the old image -- tmux truncates its own
# buffer on a shrink but keeps the character in the last column, so
# asking too early sees the stale cell that is about to be erased.
#
# The direction is safe either way. A column that is going to come clean
# does so within the timeout; one that is not, never does, and a screen
# still showing the wider render fails rather than passes while it
# waits.
#
# The row count carries the same caveat as uniform_col's, and no more of
# one: it rules out a capture with nothing in it, not a capture of
# nothing. Only a capture that came back empty altogether -- no session,
# no server, and `screen` swallows the error -- or a pane shorter than
# $2 is caught here. A game that died with its pane still open is not:
# capture-pane emits a row per pane row whether anything is running in
# it or not, so the rows are all present and blank, and what decides the
# verdict then is whether the dead screen happens to hold a character in
# that column. Prove the panels are drawn before asking.
wait_col_clean() {
	i=0
	while [ "$i" -lt 40 ]; do
		screen | awk -v col="$1" -v lo="$2" -v hi="$3" '
			NR >= lo && NR <= hi {
				n++
				c = substr($0, col, 1)
				if (c != "" && c != " ") dirty = 1
			}
			END { exit (n > 0 && !dirty) ? 0 : 1 }' && return 0
		i=$((i + 1))
		sleep 0.1
	done
	return 1
}

# Wait until $1 is on screen on at least $2 lines. For text the game
# prints that the panels print too: one line is the panel that was
# always there, and only the second says the command actually ran.
# `wait_for` cannot tell those apart, so it returns before the command
# has printed anything and the caller measures the screen it meant to
# replace.
#
# Lines and not occurrences -- grep -c counts a line once however many
# times it matches. Every caller so far wants text the panels put on a
# row of their own, so the two are the same here; a caller counting
# something that can repeat on one row would undercount, and see a
# timeout blaming its command.
wait_count() {
	i=0
	while [ "$i" -lt 40 ]; do
		[ "$(screen | grep -cF -- "$1")" -ge "$2" ] && return 0
		i=$((i + 1))
		sleep 0.1
	done
	return 1
}

# The mirror of wait_col_clean: wait for column $1 to carry something on
# some row from $2 to $3. Used for the precondition of a check that the
# column comes clean, where asking once races the render it is waiting
# for.
#
# No row-count guard, because this one cannot pass without a row: it
# succeeds only on having seen a character.
wait_col_dirty() {
	i=0
	while [ "$i" -lt 40 ]; do
		screen | awk -v col="$1" -v lo="$2" -v hi="$3" '
			NR >= lo && NR <= hi {
				c = substr($0, col, 1)
				if (c != "" && c != " ") seen = 1
			}
			END { exit seen ? 0 : 1 }' && return 0
		i=$((i + 1))
		sleep 0.1
	done
	return 1
}

# Wait until the pane's $1 (a tmux format, e.g. '#{pane_width}') is $2.
#
# Every resize assertion in this file that expects a *change* fails
# loudly if the resize did nothing. The two that expect nothing to
# change do not: an older tmux without `resize-window`, a window-size
# setting that refuses it, or a session that has died all leave
# `resize-window` writing to a stderr nobody reads, and a case that
# asserts "the game did not end" then passes without a SIGWINCH ever
# being delivered. So ask the pane, and fail if the resize never took.
wait_pane() {
	i=0
	while [ "$i" -lt 40 ]; do
		[ "$(tm display-message -p -t "$pane" "$1" 2>/dev/null)" = "$2" ] && return 0
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

# --- no column keeps text from a wider terminal -----------------------
# The message window is inset a column on each side -- newwin(msgh,
# msgw, panelh, 1), with msgw asked for as COLS-2 rather than COLS-1 --
# so the first and last columns of the screen belong to no curses
# window at all. Only sync_size() writes to stdscr, and only to erase
# it, so before that nothing erased what a wider render left in them:
# after a shrink the tails of words stayed there. At 41x24 a short-range
# scan left `4`, `G`, `3`, `A`, `5` down the right-hand edge, one from
# each of the status lines still on screen, and a stray digit beside the
# COMMAND> prompt.
#
# Only the rows below the panels. The panels span the full width
# between them -- quadw plus statw is COLS -- so their own rows have no
# unowned column, which is why the right-hand edge is clean for the
# first thirteen rows and dirty under them.
#
# The left inset column is unowned in exactly the same way, but nothing
# checks it here because nothing can put text there: below the panels
# it is unowned at every width, so it is blank from startup and stays
# blank. Asserting it would be asserting a constant.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "edge columns: the game never reached its command prompt"
	dump
# The full-screen display has to be up before the count below can mean
# anything. `to_command` matches the prompt in classic mode too, and
# there the panels are not on screen at all -- so the panel's own
# "Klingons Left" is missing, the scan's is the only one, the count
# never reaches two, and the failure would blame the scan for a
# fallback. The panel checks after the shrink would catch it in the end,
# but only after saying something untrue on the way past.
elif ! screen | grep -qF ' Status '; then
	fail "edge columns: the full-screen display never came up"
	dump
else
	# A scan, because it is the widest thing the game prints: its
	# status column runs well past column 41, which is where the
	# screen's last column will be after the shrink.
	#
	# Waited for by counting, not by `to_command` and not by `wait_for`.
	# `to_command` returns on its first look, because the prompt the
	# scan was typed at is still on screen -- every other caller in this
	# file runs it where no prompt is up yet. `wait_for` on the scan's
	# own text returns just as early, because the status panel prints
	# the same words: at 100 columns "Klingons Left" is on screen before
	# the scan runs, and it is the second occurrence that is the scan.
	#
	# Sequencing it matters beyond tidiness. Unsynchronised, the resize
	# can beat the scan to the screen, and the case then measures the
	# banner it was supposed to replace -- still a real test of the
	# stale column, but not the one the comments here describe.
	tm send-keys -t "$pane" 'srscan' Enter
	if ! wait_count 'Klingons Left' 2; then
		fail "edge columns: the scan never reached the screen"
		dump
	# Something has to occupy that column at the wide size on a row the
	# shrink keeps. If nothing does, the shrink leaves nothing behind
	# and the check below passes by describing a screen that was blank
	# there all along.
	#
	# Rows 14 to 24 rather than 14 down, because the shrink is to 24
	# rows and takes the ones below that with it. Text at column 41 on
	# row 25 would satisfy a precondition for staleness that the shrink
	# then discards, leaving the assertion below with nothing to find
	# and no way to say so.
	#
	# The scan is what puts it there in practice -- `4 G 3 A 5`, one per
	# status line -- but the banner above reaches that column too, so
	# this asserts only that the column is occupied, which is all the
	# check below needs.
	elif ! wait_col_dirty 41 14 24; then
		fail "edge columns: nothing occupied column 41 on a row the shrink keeps, so there was no stale text to leave behind"
		dump
	else
		tm resize-window -t "$session" -x 41 -y 24
		sleep 1
		# The panels have to have followed the shrink and the terminal
		# has to have shrunk at all, or what follows measures an empty
		# screen. Not a fallback check: `tui_active` is set once in
		# tui_init() and cleared only in tui_shutdown(), so a resize
		# cannot drop the game back to the classic display -- that is
		# what the ` Status ` guard before the scan is for, where the
		# display might never have come up in the first place.
		if ! screen | head -1 | grep -qF 'Quadrant' ||
		   ! screen | head -1 | grep -qF 'Status'; then
			fail "edge columns: the panels are not on the top row at 41 columns"
			dump
		elif [ "$(screen | head -1 | wc -c)" -gt 46 ]; then
			fail "edge columns: the terminal did not shrink"
			dump
		# Row 13 is the panels' bottom border, so 14 down is the message
		# area. Column 41 is the last column of the screen: the message
		# window is 39 wide at x-offset 1, so it owns 2 through 40 and
		# nothing owns 41.
		elif ! wait_col_clean 41 14 24; then
			fail "edge columns: the last column kept text from the wider terminal"
			dump
		fi
	fi
fi

# --- a restored prompt does not land on live conversation --------------
# restore_curline()'s erase branch works out how many rows the pair
# occupies at the old width and rubs out that many, counting back from
# the last row with anything on it. That is right when those rows really
# are the stump of the line -- a width change truncates the line in
# place and leaves it there.
#
# A height shrink does not. wresize() keeps the top of the window and
# drops the rows below the cut, and the pending prompt is the last thing
# written, so the cut reaches it. Here it takes the whole of it, the
# pair being one row; a pair that wraps can lose its lower rows and keep
# its first, which is `half stump:` further down. Either way the rows
# the erase counts back over are older conversation, and the reprint
# lands on top of it.
#
# Three asks with the first two answered, then 80x24 to 78x20: the
# window drops from eleven rows to seven, taking the pending question
# with it, and the reprint overwrote the `n` that answered the second
# ask. What is left reads as though the game asked the same question
# twice in a row, which it never does -- every ask is answered before
# the next, so two adjacent identical questions cannot be a real
# transcript. That is what this asserts.
#
# `rest 5000` because "Are you sure?" is written with prout, which ends
# its own line: the ended branch is the one that reaches the erase with
# a question whose answer sits on the row below.
start 80 24 'tournament 7 short novice pw'
if ! to_command; then
	fail "reprint: the game never reached its command prompt"
	dump
elif ! screen | grep -qF ' Status '; then
	fail "reprint: the full-screen display never came up"
	dump
else
	# Two asks, each answered, then a third left pending.
	askfail=
	for answered in 1 2; do
		tm send-keys -t "$pane" 'rest 5000' Enter
		wait_for 'Are you sure' || askfail="ask $answered never came"
		tm send-keys -t "$pane" 'n' Enter
		to_command || askfail="ask $answered never came back to the prompt"
	done
	tm send-keys -t "$pane" 'rest 5000' Enter
	wait_for 'Are you sure' || askfail="the third ask never came"
	if [ -n "$askfail" ]; then
		fail "reprint: $askfail"
		dump
	# The conversation has to have the shape the defect needs before the
	# resize: three questions and the two answers between them, all on
	# screen at once. Without that the shrink has nothing live to
	# overwrite and the check below passes on a screen that could not
	# have shown the fault.
	elif [ "$(screen | sed -n '14,24p' | grep -cF 'Are you sure')" -ne 3 ] ||
	     [ "$(screen | sed -n '14,24p' | grep -cx ' *n *')" -ne 2 ]; then
		fail "reprint: the conversation is not three asks and two answers, so the shrink has nothing live to overwrite"
		dump
	else
		tm resize-window -t "$session" -x 78 -y 20
		sleep 1
		# Load-bearing twice over, so do not weaken it to a plain panel
		# check. tmux scrolls the panel top out of row 1 on a height
		# shrink, so a game that has not repainted by the time the
		# sleep expires fails here rather than passing the assertion
		# below on a screen that never had the shape. Verified by
		# stopping the game with SIGSTOP across the resize: this is the
		# check that fires.
		if ! screen | head -1 | grep -qF 'Quadrant'; then
			fail "reprint: the panels are not on the top row after the shrink, so the game had not repainted"
			dump
		# Rows 14 down are the message area at twenty lines. Two
		# adjacent rows carrying the same question is the artifact:
		# the game answers each ask before the next, so it cannot
		# have printed that.
		elif screen | awk 'NR >= 14 {
		                     t = $0
		                     sub(/^ +/, "", t); sub(/ +$/, "", t)
		                     if (t != "" && t == prev) { found = 1 }
		                     prev = t
		                   }
		                   END { exit found ? 0 : 1 }'; then
			fail "reprint: the restored question was printed over live conversation, leaving the same line twice in a row"
			dump
		fi
	fi
fi

# --- a wrapped prompt is not stacked when the terminal widens ----------
# row_starts_line() compares the row on screen against the line as it
# was, and bounds how much it compares by the narrower of the two
# widths. The row holds real characters only where they agree: a shrink
# truncates it to the new width, a grow keeps what the old width held
# and pads the rest. Comparing past that runs into the padding and
# mismatches against a stump that is perfectly intact, which sends a
# width change down the scroll path and leaves the stump standing with a
# fresh copy under it.
#
# It takes a line longer than the old message window, and that needs no
# long line: make_windows() caps quadw at QUADW but lets msgw keep
# tracking COLS down, so 50 columns gives a 48-column message window and
# the 57-character skill question wraps in it. Widening from there is the
# shape. Without the bound this leaves two copies, and each further step
# that keeps the question wrapped adds another -- 50, 55, 58, 100 gives
# one, two, three, four. Once the window is wide enough to hold the
# question on one row the next grow matches and erases again, so a drag
# straight out to 120 stops at two rather than climbing.
#
# The setup conversation rather than a game, because that question is the
# ready-made line longer than the window. `rest 5000`'s ask is the
# fourteen characters of `prout("Are you sure? ")` at events.c:420,
# trailing space included -- end_line() keeps it -- and the case above
# only ever narrows to 78 columns, so it cannot reach this bound. It
# would below sixteen, where msgw drops under fourteen and even that
# question wraps; nothing sends it there.
start 80 24
if ! wait_for 'regular, tournament, or frozen'; then
	fail "stacking: the game never asked its first question"
	dump
elif ! screen | grep -qF ' Status '; then
	fail "stacking: the full-screen display never came up"
	dump
else
	setupfail=
	tm send-keys -t "$pane" 'tournament' Enter
	wait_for 'tournament number' || setupfail="the tournament number was never asked"
	tm send-keys -t "$pane" '7' Enter
	wait_for 'Short, Medium, or Long' || setupfail="the length was never asked"
	tm send-keys -t "$pane" 'short' Enter
	wait_for 'Are you a Novice' || setupfail="the skill question never came"
	if [ -n "$setupfail" ]; then
		fail "stacking: $setupfail"
		dump
	else
		tm resize-window -t "$session" -x 50 -y 24
		sleep 1
		# The question has to have actually wrapped, or the grow below
		# has no over-long line to mismatch on and the count passes
		# without exercising the bound at all. Two rows: the first cut
		# at 48 columns, the tail on the next.
		if [ "$(screen | grep -c 'Are you a Novice')" -ne 1 ] ||
		   ! screen | grep -qx ' *s player? *'; then
			fail "stacking: the skill question did not wrap at 50 columns, so the widening below cannot reach the bound"
			dump
		else
			tm resize-window -t "$session" -x 100 -y 24
			sleep 1
			# One copy. Counting the question's opening rather than
			# the whole of it, so the wrapped stump's first row
			# counts too -- that row is what a second copy leaves
			# behind.
			if [ "$(screen | grep -c 'Are you a Novice')" -ne 1 ]; then
				fail "stacking: widening the terminal left more than one copy of the wrapped question"
				dump
			# And one copy whole, on a row of its own, with the tail
			# the 48-column wrap left gone. Counting openings alone
			# says nothing about what follows them: an erase that
			# rubbed out the stump's first row and left its tail
			# behind counts one and reads
			#     Are you a Novice, ... player?
			#     s player?
			# on screen. This is also the repaint guard for the
			# widening -- the panel check that guards the shrink in
			# the case above cannot serve here, because a width-only
			# resize never scrolls the panels out of row 1, so a
			# screen that has not repainted still passes it. Neither
			# half of this can be satisfied by the stale 50-column
			# screen, which has the question cut at `Emeritu`.
			elif ! screen | grep -qx ' *Are you a Novice, Fair, Good, Expert, or Emeritus player? *' ||
			     screen | grep -qx ' *s player? *'; then
				fail "stacking: the widened screen does not show the question once and whole, with the wrapped tail gone"
				dump
			fi
		fi
	fi
fi

# --- a stump taller than the window is still rubbed out ----------------
# The erase branch anchors `rows` back from the last non-blank row and
# clamps a negative result to 0. Clamping means the stump does not fit:
# its own top has scrolled off, so row 0 holds a middle row of it and
# not its first. row_starts_line() rightly says that row does not begin
# the line -- and taking the scroll path on the strength of that keeps
# what is left of the stump and writes a fresh copy under it, which is
# the stacking the erase exists to prevent.
#
# So the clamp is its own answer: it only happens when the stump is
# taller than the window, and a stump taller than the window is one that
# is certainly on screen. Erase unconditionally there.
#
# 30x5 gives a two-row message window 28 columns wide, and the skill
# question needs three rows in it. Widening to 60 leaves the tail behind
# without this -- a lone ` ?` row above the restored question, which is
# what a player sees.
#
# Five rows and not fifteen. The panels used to take PANELH rows at any
# height, so fifteen left the two this needs; they yield to the
# conversation now, which keeps a floor of MSGMIN, so a two-row window
# only happens where there are not even MSGMIN rows to give. The wrap
# is unchanged at 30 columns, which is why only the height moved -- and
# the precondition below is what caught the change rather than the case
# going quietly green on an unclamped anchor.
start 80 24
if ! wait_for 'regular, tournament, or frozen'; then
	fail "tall stump: the game never asked its first question"
	dump
elif ! screen | grep -qF ' Status '; then
	fail "tall stump: the full-screen display never came up"
	dump
else
	tallfail=
	tm send-keys -t "$pane" 'tournament' Enter
	wait_for 'tournament number' || tallfail="the tournament number was never asked"
	tm send-keys -t "$pane" '7' Enter
	wait_for 'Short, Medium, or Long' || tallfail="the length was never asked"
	tm send-keys -t "$pane" 'short' Enter
	wait_for 'Are you a Novice' || tallfail="the skill question never came"
	if [ -n "$tallfail" ]; then
		fail "tall stump: $tallfail"
		dump
	else
		tm resize-window -t "$session" -x 30 -y 5
		sleep 1
		# The stump has to be taller than the conversation on screen,
		# or the anchor is never clamped and the widening below proves
		# nothing. Three rows needed, two available, so the question's
		# own opening scrolls off: what is left is ` , Expert, or
		# Emeritus player` and a lone ` ?`.
		#
		# The absent opening is the assertion that pins it, and it is
		# the one worth keeping. Wrapping alone does not: one row
		# taller, at 30x6, the message window is three rows, the whole
		# stump fits, nothing clamps -- and a check of "not whole on
		# any row" plus a surviving fragment passes there just the
		# same, leaving the case testing the unclamped path in silence.
		if screen | grep -qF 'Are you a Novice' ||
		   ! screen | grep -qF 'Emeritus player'; then
			fail "tall stump: the question is not clipped across more rows than the window has, so the erase anchor is not clamped"
			dump
		else
			tm resize-window -t "$session" -x 60 -y 5
			sleep 1
			# Whole, once, and nothing of the old wrap left above
			# it. The tail row is what the clamp used to strand:
			# at 28 columns the question breaks so that its last
			# row is a lone `?`.
			if ! screen | grep -qx ' *Are you a Novice, Fair, Good, Expert, or Emeritus player? *'; then
				fail "tall stump: the widened screen does not show the question once and whole"
				dump
			elif screen | grep -qx ' *? *'; then
				fail "tall stump: a row of the old wrap was left above the restored question"
				dump
			fi
		fi
	fi
fi

# --- half a wrapped stump is still found ------------------------------
# A height shrink does not always take the pending line away entire. It
# drops the rows below the cut, so a pair that wraps can lose its lower
# rows and keep its first -- the stump is then shorter than the row
# count says and begins further down than the arithmetic does.
#
# Asking only at the calculated row missed it, took the scroll path, and
# wrote a second copy under the row that had survived: the whole
# ` COMMAND> phasers manual ...` line twice, one above the other. The
# search now runs from the bottom up over as many rows as the pair could
# occupy, so it finds a stump wherever the cut left it.
#
# Both axes at once, which is what makes it reachable, and at sizes the
# display supports rather than below the minimum: 80x25 to 79x24. None
# of the three cases above can get here -- `reprint:` shrinks the height
# with a one-row pair, and `stacking:` and `tall stump:` change the
# width alone.
#
# A command typed and not entered, because the pair has to be longer
# than the window: `proutn("COMMAND> ")` at sst.c:214 is nine columns --
# the space before it on screen is the window's own margin, not part of
# the prompt -- and the text is seventy-five, against a seventy-eight
# column window at 80 columns. So six characters land on the second row.
# It is never submitted, so nothing depends on the command being
# sensible.
LONGCMD='phasers manual 100 200 300 400 500 600 700 800 900 1000 1100 1200 1300 1400'
start 80 25 'tournament 7 short novice pw'
if ! to_command; then
	fail "half stump: the game never reached its command prompt"
	dump
elif ! screen | grep -qF ' Status '; then
	fail "half stump: the full-screen display never came up"
	dump
else
	# A scan first, so the conversation fills the window and the pair
	# sits on its last two rows -- which is what puts the cut through
	# the middle of the pair rather than above it.
	tm send-keys -t "$pane" 'srscan' Enter
	if ! to_command; then
		fail "half stump: the scan never came back to the command prompt"
		dump
	else
		tm send-keys -t "$pane" "$LONGCMD"
		sleep 0.5
		# It has to have wrapped, or the cut cannot fall inside it and
		# the case tests nothing new. A row holding the opening and the
		# end together is a pair that fits on one row.
		# The whole command has to be echoed before the wrap test
		# below can mean anything: that test reads "the row holding
		# the opening does not hold the end", which is equally true
		# of a pair that wrapped and of one only half echoed. Taking
		# the second for the first would run the case on an unwrapped
		# pair, where the assertion passes without reaching the scan.
		#
		# The last four characters and not the last nine: the wrap
		# falls between them, so the second row reads `0 1400` and a
		# search for `1300 1400` finds nothing on a pair that is
		# perfectly whole.
		if ! screen | grep -q 'phasers manual 100'; then
			fail "half stump: the typed command never reached the screen"
			dump
		elif ! screen | grep -q '1400'; then
			fail "half stump: the typed command is not fully echoed, so the wrap test below cannot tell a wrap from a half-drawn line"
			dump
		elif screen | grep 'phasers manual 100' | grep -q '1400'; then
			fail "half stump: the command did not wrap, so the shrink cannot cut through the middle of it"
			dump
		else
			tm resize-window -t "$session" -x 79 -y 24
			sleep 1
			if ! screen | head -1 | grep -qF 'Quadrant'; then
				fail "half stump: the panels are not on the top row after the shrink, so the game had not repainted"
				dump
			elif [ "$(screen | grep -c 'phasers manual 100')" -ne 1 ]; then
				fail "half stump: the surviving row of the stump was kept and a second copy written under it"
				dump
			fi
		fi
	fi
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

# --- the conversation keeps rows however short the terminal ------------
# The panels are PANELH rows deep and used to take them whatever the
# terminal had. At thirteen rows and fewer that was the whole screen:
# the message window was placed at row PANELH, off the bottom, so the
# player saw no conversation, no COMMAND> prompt and no sign the game
# was waiting -- and it was, since typing still worked, blind. #94.
#
# Just above that band the display was on screen and empty instead. The
# pager's page height was LINES-PANELH-1, which is zero at fourteen rows
# and one at fifteen, so a command that pages went by screen after
# screen of `[HIT SPACE BAR TO CONTINUE]` with nothing above it. #95.
#
# The panels now give up rows rather than the conversation: the message
# window keeps a floor of three, two of text and the row the prompt or
# the pager sits on. Nothing changes at sixteen rows and above, where
# the panels already fit with that much left over.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "short conversation: the game never reached its command prompt"
	dump
else
	# Down to four, which is where the prompt stops fitting: the panels
	# keep PANELMIN and the conversation gets what is left, one row at
	# four and none at three. Both documents state that floor, so it is
	# asserted rather than left to be found by hand.
	for h in 15 14 13 11 9 6 5 4; do
		tm resize-window -t "$session" -x 72 -y "$h"
		sleep 1
		# The panels have to still be drawn, or what follows passes by
		# describing a screen with nothing on it.
		if ! screen | head -1 | grep -qF 'Quadrant'; then
			fail "short conversation: the panels are not on screen at 72x$h"
			dump
		# The prompt is the whole point: at these heights it was off
		# the bottom of the screen and the game read commands blind.
		elif ! screen | grep -qF 'COMMAND'; then
			fail "short conversation: no COMMAND prompt on screen at 72x$h, so nothing shows the game is waiting"
			dump
		fi
	done
	# And the game is still answering, not merely showing a prompt.
	# Waited for by the scan's own output and not by `to_command`,
	# which returns on its first look -- the command is sent at a live
	# prompt, so the echoed ` COMMAND> srscan` satisfies it before the
	# game has done anything, and a hung game would pass. The file
	# documents that trap beside the `reprint:` case.
	tm resize-window -t "$session" -x 72 -y 13
	sleep 1
	tm send-keys -t "$pane" 'lrscan' Enter
	expect_re "short conversation: the game stopped answering at 72x13" "$LRSCAN"
	# And the floor below it, which is the other thing both documents
	# now promise: at three rows the message window's origin is off the
	# bottom, mvwin() refuses it, and nothing of the conversation is
	# drawn -- the game still reads what is typed, with nothing on
	# screen to say so. Nothing else in this file reaches that path any
	# more: it used to be everything at thirteen rows and below, and
	# the panels yielding rows moved it to three.
	#
	# Growing back is half the point. The refused move is what used to
	# strand the window below the screen for good, so the recovery is
	# asserted with it rather than left to the cases above.
	#
	# Asked of what the conversation is actually holding, not of
	# `COMMAND`. The `lrscan` above leaves the game at its pause
	# prompt, so `COMMAND` is already off the screen before the shrink
	# and its absence afterwards would hold whatever the geometry did
	# -- a PANELMIN of 2 would put a one-row conversation here and
	# redraw the pause prompt into it, and a check for `COMMAND` would
	# still pass.
	tm resize-window -t "$session" -x 72 -y 3
	sleep 1
	if screen | grep -qE 'CONTINUE|HIT SPACE BAR|Long-range'; then
		fail "short conversation: the conversation is on screen at 72x3, where the documents say there is none"
		dump
	fi
	tm resize-window -t "$session" -x 72 -y 24
	sleep 1
	if ! to_command; then
		fail "short conversation: the conversation did not come back after 72x3"
		dump
	fi
fi

# --- and paged output has something to page ----------------------------
# The other half of the same geometry. With a page height of zero every
# line of a paged command triggered a pause, and the message window was
# one row, so the line was printed and immediately written over by
# `[HIT SPACE BAR TO CONTINUE]`. What a player saw was the prompt alone,
# again and again, and never a word of the scan.
#
# Fourteen rows because that is where the page height was exactly zero.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "short paging: the game never reached its command prompt"
	dump
else
	tm resize-window -t "$session" -x 72 -y 14
	sleep 1
	if ! screen | head -1 | grep -qF 'Quadrant'; then
		fail "short paging: the panels are not on screen at 72x14"
		dump
	else
		tm send-keys -t "$pane" 'lrscan' Enter
		# Wait for the pause itself before measuring. Asking too early
		# finds neither the prompt nor the output and would report the
		# defect on a screen that had simply not been drawn yet.
		i=0
		saw_pause=
		while [ "$i" -lt 40 ]; do
			if screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
				saw_pause=yes
				break
			fi
			i=$((i + 1))
			sleep 0.1
		done
		# A line of what is being paged, not just the prompt asking to
		# page it. LRSCAN matches the scan's own heading in either of
		# its forms. Only the working one can appear here -- a new
		# game zeroes every damage[] and no time passes between the
		# prompt and this scan -- but the shared pattern costs
		# nothing and needs no special case.
		if [ -z "$saw_pause" ]; then
			fail "short paging: the scan never paused at 72x14, so there is no paging to measure"
			dump
		elif ! screen | grep -qE "$LRSCAN"; then
			fail "short paging: the scan paused with none of its output on screen at 72x14"
			dump
		fi
	fi
fi

# --- and six rows is where that stops being true ----------------------
# The floor both documents give. A two-row window pages one line, and
# LRSCAN spends it on the blank it opens with (`skip(1)`, reports.c),
# so five rows shows the pager over an empty row -- which is why the
# documents say six for a paged command and note that SRSCAN, having no
# leading blank, still shows a line at five.
#
# Asserted because the sentence was wrong the first time it was written
# and had to be measured by hand to find out. Six is the boundary; the
# case above already covers well inside it.
start 100 30 'tournament 7 short novice pw'
if ! to_command; then
	fail "paging floor: the game never reached its command prompt"
	dump
else
	tm resize-window -t "$session" -x 72 -y 6
	sleep 1
	if ! screen | head -1 | grep -qF 'Quadrant'; then
		fail "paging floor: the panels are not on screen at 72x6"
		dump
	else
		tm send-keys -t "$pane" 'lrscan' Enter
		i=0
		while [ "$i" -lt 40 ]; do
			screen | grep -qE 'CONTINUE|HIT SPACE BAR' && break
			i=$((i + 1))
			sleep 0.1
		done
		if ! screen | grep -qE 'CONTINUE|HIT SPACE BAR'; then
			fail "paging floor: the scan never paused at 72x6"
			dump
		elif ! screen | grep -qE "$LRSCAN"; then
			fail "paging floor: at 72x6 the scan paused with none of its output on screen, so the six-row floor both documents give is wrong"
			dump
		fi
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
# Those two cases are panel heights of twelve and eleven, which used to
# mean screens of twelve and eleven rows. The panels yield MSGMIN to the
# conversation now, so a panel of twelve rows is a screen of fifteen and
# one of eleven a screen of fourteen -- and a sweep of 12/11/9 alone
# stopped visiting either, leaving the block catching over-draw in
# general but neither of the boundaries it was written for. So 15 and 14
# for those, and 12/11/9 kept for shrinks well past both.
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
	for h in 15 14 12 11 9; do
		tm resize-window -t "$session" -x 72 -y "$h"
		sleep 1
		# Nothing numeric belongs on the panels' bottom border --
		# the only caption it ever carries is SENSORS DAMAGED -- and
		# a grid line that landed on it brings its row label along.
		#
		# Row h-3, not row h. The panels used to fill a screen this
		# short, so the last row of it was their border; they yield
		# MSGMIN rows to the conversation now, so the border is at
		# LINES-MSGMIN and row h is the conversation's last row,
		# which at this point holds ` COMMAND>`. Asserted at row h
		# the check still passed while a grid line landed on the
		# border at row h-3 -- the regression this block exists for,
		# invisible.
		if ! screen | head -1 | grep -qF 'Quadrant'; then
			fail "short: the panels are not on screen at 72x$h"
			dump
		elif ! screen | awk -v h="$h" 'NR == h - 3 && /[0-9]/ { bad = 1 }
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
	# 72x24 is the floor exactly, so this asserts the gate accepts
	# its own minimum -- MINROWS and MINCOLS are the numbers under
	# test. Not the only guard: raising MINCOLS to 73 on a copy of
	# the tree outside the repository failed four checks -- the two
	# "72x24:" ones, the briefing that carries on from them, and this.
	# It costs no session of its own, and the
	# boundary is worth saying out loud in the block that sits on it.
	# The frame, not want_quadtitle: setup has not run, so the
	# quadrant panel has no coordinates in its title yet. " Quadrant "
	# is what the fallback helper greps for to prove the panels are
	# *absent* at this same moment, so it is the right positive too.
	expect "wrap: the panels were refused at exactly 72x24" ' Quadrant '
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
#
# Those two counts were measured when the panels took PANELH rows at
# any height, so 20x8 and 80x14 left a one-row or off-screen window.
# They leave three rows each now that the panels yield. The sizes are
# kept because what they exercise -- the corner drag and the
# height-only branch -- does not depend on that, and every assertion
# below runs at 80x24, which the change does not reach. The reason
# 80x23 had to be added is the part that no longer describes these
# two.
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
		# Two heights, because the panels yielding rows moved what
		# each one shows, and the order between them matters.
		#
		# Five first, and reached straight from 24: this is the only
		# thing in this file that reaches the `maxy > 1` guard, and
		# the guard only fires while the pair is being rebuilt into
		# a two-row window. Coming here by way of fifteen instead
		# rebuilds it there, where the guard is satisfied either
		# way, and the shrink that follows only truncates what is
		# already right -- a broken build and a working one then
		# render identically at five rows. Measured: they do.
		#
		# This is the state both documents describe to the player --
		# a window too short to hold the pair keeps the answer and
		# loses the question -- and every other check here looks at
		# the screen only after growing back, so a change that kept
		# the question and dropped the answer would leave both
		# documents wrong with the suite green, and a player typing
		# over a `y` they cannot see.
		#
		# Proved against a broken game, which is what step 4 asks of
		# a test that runs one: `maxy > 1` -> `maxy > 2` on the
		# newline guard in restore_curline() suppresses the newline
		# only at a two-row window, so the pair packs onto the two
		# rows -- `Are you sure? ybbb...` over `bbb...`, the
		# question kept and the answer displaced. Built in a copy
		# outside the repository, that fails here and nowhere else
		# in this file.
		tm resize-window -t "$session" -y 5
		sleep 1
		if ! screen | awk '/Are you sure\?/ { q++ }
		                   /^ *yb+$/ { a++ }
		                   /^ *b+$/ { c++ }
		                   END { exit (q == 0 && a == 1 && c == 1) ? 0 : 1 }'; then
			fail "wrapped answer: at 80x5 the answer did not survive without its question"
			dump
		fi
		# Back up, so fifteen is reached from a window that holds the
		# whole pair rather than from the stump five left.
		tm resize-window -t "$session" -y 24
		sleep 1
		# Fifteen rows used to leave a two-row message window and now
		# leaves three, so all of it fits. Strictly the better
		# outcome and asserted as such rather than relaxed: exactly
		# one question, and both rows of the answer under it.
		tm resize-window -t "$session" -y 15
		sleep 1
		if ! screen | awk '/Are you sure\?/ { q++ }
		                   /^ *yb+$/ { a++ }
		                   /^ *b+$/ { c++ }
		                   END { exit (q == 1 && a == 1 && c == 1) ? 0 : 1 }'; then
			fail "wrapped answer: at 80x15 the question and its wrapped answer are not all on screen"
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
# window both of this round's fixes are invisible from the far end:
# the display recovers once the terminal is back at 80x24, so a check
# that only looks there passes against builds that lose the question
# at every other step of the way.
#
# Four rows, where this used to say fourteen. The panels took PANELH
# rows at any height then, so fourteen left one; they yield to the
# conversation now, which keeps MSGMIN, and a one-row window only
# happens where there is not that much to give -- LINES of 4. Moved
# rather than relaxed: the guard this reaches is `maxy > 1` on the
# write path, which is about a window of one row and nothing else, so
# testing it anywhere else would be testing nothing.
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
		tm resize-window -t "$session" -x 80 -y 4
		sleep 1
		for w in 78 76 74 72; do
			tm resize-window -t "$session" -x "$w" -y 4
			sleep 1
			if ! screen | awk '/Are you sure\?/ { n++ }
			                   END { exit n == 1 ? 0 : 1 }'; then
				fail "ended: at ${w}x4 the question is not on screen exactly once"
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
		# Row 8, not row 11: the panels give rows up to the
		# conversation on a short terminal, so at eleven lines they
		# are LINES-MSGMIN deep and their bottom border is row 8.
		# It was row 11 while they took the whole screen, which is
		# the state #94 was about.
		if ! screen | awk 'NR == 8 && /SENSORS DAMAGED/ { seen = 1 }
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

# --- and a size bigger than the terminal is refused -------------------
# The gate is LINES < 24 || COLS < 72, asked of curses -- and where the
# environment has pinned a dimension, curses' number need not be the
# window's. A pinned size that clears the floor is drawn whatever the
# terminal is, and the display then does not fit the screen it is on.
#
# Both axes break, differently. COLUMNS=190 in a 100-column pane runs
# the frame off the edge so it wraps back over the quadrant panel's
# coordinate row. LINES=40 in a 20-row pane draws the panels fine --
# they cap at PANELH -- and puts the message window's lower rows off
# the bottom, so the conversation walks off screen and what is left
# reads as garbage: measured, ` COMMAND> t     7.00`, a prompt on top
# of half a status line.
#
# The height case is run at 30 rows rather than the 20 it was measured
# at, and not because 20 fails the floor -- with LINES=40 curses says
# 40, so the floor gate never fires and the refusal is the oversize one
# either way. It is the advice that differs. A 20-row window is under
# the floor, so the notice takes the compound form, "Grow to 72x24,
# unset LINES, and rerun sst -t." -- whose "unset" is lowercase, where
# this greps for "Unset LINES". 30 rows clears the floor and gets the
# capitalised form.
#
# Neither is the clipping the manual promises for a small terminal.
# Falling back is: the classic display fits any window. #164.
fallback "oversize COLUMNS" 100 30 "LINES/COLUMNS make it" 'env COLUMNS=190'
# Asserting the advice rather than the size report, which the COLUMNS
# case above already covers -- smallwindow picks that line from the
# window, not from which axis is at fault, so both cases take the same
# branch for it.
#
# What this reaches is the LINES-only return in tui_refusal_blame(),
# which the COLUMNS cases cannot: they are a COLUMNS fault. It was the
# only arm reaching it until the pinned-height pair added for #169 --
# "pinned height" and "pinned height unchanged" -- which run the same
# LINES=40 in a 100x30 pane, and the first of them asserts the same
# advice line. This one still earns its place as the startup half:
# those two are about the play-again retry, which a fallback() arm
# never reaches.
#
# It does not cover the 24 that blames() is passed, though -- with
# curses at 40 against a 30-row window the oversize half answers first
# and the floor is never looked at. "env size" is what guards the 24,
# its LINES=20 being under it.
#
# Proved by forcing the LINES half to FALSE on a copy of the tree
# outside the repository: this check failed, and so did "env size",
# which wants both variables named. That was two arms when it was
# written and is three now, "pinned height" having joined them --
# measured again on the #169 branch rather than assumed.
fallback "oversize LINES" 100 30 "Unset LINES" 'env LINES=40'

# The other direction of the same fault, and the reason the advice is
# chosen from the window rather than from curses: a pin *below* the
# floor in a window with room to spare. COLUMNS=60 in a 100-column
# terminal is refused for being under 72, and telling the player to grow
# a window that is already 100 wide is advice that cannot work -- it is
# the case README and sst.doc name, how a 100x30 window comes to be
# refused. Growing was what the game said here until #164.
# The whole line, tail and all, because README.md and sst.doc quote it
# verbatim. Grepping only "Unset COLUMNS" would let the rest drift and
# leave both manuals quoting a line the game does not print.
fallback "undersize pin" 100 30 "Unset COLUMNS, rerun sst -t -- classic for now." 'env COLUMNS=60'

# And a pin that matches the window exactly, which is the half of the
# rule nothing else reaches. COLUMNS=70 in a 70-column window is under
# the floor and invisible to a size comparison -- curses says 70 and so
# does the terminal -- so blames() has to ask pinned() rather than
# compare. Left to a comparison the player is told to grow the window,
# grows it, and the pin refuses them again in silence.
#
# Proved by replacing that half with `curses_v < floor_v && curses_v !=
# term_v` on a copy outside the repository: this check failed and every
# other check in the file passed.
fallback "pin equal to the window" 70 24 "Grow to 72x24, unset COLUMNS" 'env COLUMNS=70'

# --- an exported size still has the last word -------------------------
# tui_init() asks the terminal its size with an ioctl, because a second
# initscr() re-reads nothing and a retry would otherwise see the size it
# was turned down at. That override stops at LINES and COLUMNS.
#
# ncurses honours those over the terminal, and a dimension it has taken
# from the environment stops following the terminal thereafter. With
# both pinned nothing about the size can move at all, so the display
# would never follow the window again -- worse than a classic display
# that resizes correctly. With one, the free dimension still moves.
# pinned()'s comment in tui.c carries the measurement for the one-pinned
# case; the both-pinned half follows from resized() being a single
# yes/no over the pair, which tui_init()'s comment says.
# So a pinned dimension is left alone, and this is the arm that holds
# that line for the case where both are pinned.
#
# LINES=20 COLUMNS=70 is under the 72x24 floor while the terminal is
# comfortably over it, so believing the environment is the whole
# difference between the panels and the fallback.
#
# Proved by deleting the getenv pair, so the ioctl overrode the
# environment again, and running this file on a copy of the tree outside
# the repository: this check failed. It was the only one at the time.
# Of the pinned cases added since, the oversize and undersize-pin ones
# depend on the same override and would fail with it; "pin equal to the
# window" would not, its pin matching the window exactly, which is what
# that case is for.
start 100 30 'tournament 7 short novice pw' 'env LINES=20 COLUMNS=70'
if ! wait_scrollback 'Unset LINES and COLUMNS'; then
	# Both pins are under the floor in a window with room for the
	# panels, so both are named -- and the notice is the pin's rather
	# than "Terminal too small", which would be false of a 100x30
	# window. That naming is also the both-blamed form of the blame
	# rule, which no other case reaches.
	fail "env size: an exported 20x70 did not refuse the panels in a 100x30 terminal"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "env size: the game never reached a command prompt"
	dump
else
	# And the same at the play-again boundary, which is where both of
	# this arm's defects lived. The size the game uses is pinned, so
	# nothing about it can have changed and there is nothing to say --
	# but the two sides of that judgment read different sources for a
	# while: the refusal size came from curses, which the environment
	# pins, and the comparison from the terminal, which it does not.
	# They could never agree, so the notice printed every game, quoting
	# a size that was not the player's window. Nothing here resizes,
	# which is the point.
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "env size: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "env size: never asked about another game"
			dump
		else
			tm send-keys -t "$pane" 'y' Enter
			if ! wait_for 'regular, tournament, or frozen'; then
				fail "env size: the second game never started"
				dump
			else
				# A retry that succeeds switches to the
				# alternate screen and wipes what is on it; a
				# refused one leaves the classic conversation
				# where it was. So the question from before
				# the retry is still on screen exactly when
				# the panels did not come up.
				#
				# Not a grep for " Quadrant ": the first
				# game's own setup line, "The Enterprise is
				# currently in Quadrant 3 - 2  Sector 7 - 9",
				# is still on this screen and matches it. That
				# check failed here on a correct build.
				#
				# Proved against the narrower fix that was
				# proposed first -- the guard on tui_init()'s
				# first call only, leaving a retry free to
				# override -- built on a copy of the tree
				# outside the repository. That alternative
				# passes every other check in this file; this
				# one is what says no to it.
				if ! screen | grep -qF 'play again'; then
					fail "env size: the retry overrode a pinned size"
					dump
				fi
				if scrollback | grep -qF 'staying classic'; then
					fail "env size: a player who changed nothing was told the terminal moved"
					dump_scrollback
				fi
			fi
		fi
	fi
fi

# --- the fallback leaves no curses signal handler behind ---------------
# tui_init() calls initscr() before it can know the terminal is too
# small for the panels, and endwin() does not take back the SIGWINCH
# handler initscr() installed. So a game that fell back to the classic
# display carried a handler for the rest of the session, where a game
# started without -t has none -- which is what made every blocking read
# in the classic path interruptible there and nowhere else. That was
# #150, fixed by retrying on EINTR in both readers; this is the cause
# behind it. #152.
#
# The retries mean there is nothing left to see in what the game prints:
# both builds behave identically. So this asks the kernel instead.
# /proc/<pid>/status gives SigCgt, the mask of signals the process has
# handlers for, and SIGWINCH is 28 -- so bit 27. Linux only; skipped
# elsewhere, since the fix is worth having either way and the rest of
# this file already runs on macOS.
#
# Three arms, because two of them cannot tell each other apart. The
# fallback game and the plain game both expect "no", so a
# sigwinch_caught() that answered "no" for any reason -- an awk slip, a
# /proc format that moved -- would satisfy both and the case would pass
# having measured nothing. The full-screen game at 80x24 is the arm that
# expects "yes": it is the one that fails if the reading stops working,
# and it pins the other half of the fix besides, that the restore runs
# only on the give-up path and leaves real curses alone.
if [ ! -r /proc/self/status ]; then
	# A developer on macOS gets a note. Linux CI does not: /proc is the
	# only way to see this, so a leg where it went missing -- hidepid, an
	# unusual container, a format that moved -- would otherwise report a
	# plain pass with the three arms silently absent, which under ctest
	# shows no output at all. Same reasoning as the tmux guard at the top
	# of this file.
	if [ -n "${CI:-}" ] && [ "$(uname -s)" = Linux ]; then
		fail "sigwinch: no readable /proc on Linux CI, so the disposition went unchecked"
	else
		printf 'note: no /proc, so the SIGWINCH disposition went unchecked\n' >&2
	fi
else
	if ! command -v pgrep >/dev/null 2>&1; then
		fail "sigwinch: pgrep is missing, so the game's process could not be found"
	fi
	sigwinch_caught() {
		# $1 is the pane's shell; sst is its child.
		# The game, not whatever else the pane's shell has going.
		# Taking the first child would read `sleep 30` under a shell
		# that keeps it as one -- dash does; bash execs it into the
		# pane pid and leaves no children at all -- and `sleep`'s mask
		# is 0, which is the answer two of the three arms want, so
		# they would pass on a corpse. Either shell now answers nopid
		# instead, which fails.
		gamepid=
		for c in $(pgrep -P "$1" 2>/dev/null); do
			case $(tr '\0' ' ' < "/proc/$c/cmdline" 2>/dev/null) in
			*sst*) gamepid=$c; break ;;
			esac
		done
		[ -n "$gamepid" ] || { echo "nopid"; return; }
		mask=$(awk '/^SigCgt:/ { print $2 }' "/proc/$gamepid/status" 2>/dev/null)
		[ -n "$mask" ] || { echo "nomask"; return; }
		awk -v m="$mask" 'BEGIN {
			n = 0
			for (i = 1; i <= length(m); i++) {
				c = index("0123456789abcdef", substr(m, i, 1)) - 1
				n = (n * 16) + c
			}
			# bit 27, reached by halving rather than shifting:
			# POSIX awk has no >> operator.
			for (i = 0; i < 27; i++) n = int(n / 2)
			print (n % 2) ? "yes" : "no"
		}'
	}
	for arm in 'fallback:-t:40:10:no' 'plain::40:10:no' 'full-screen:-t:80:24:yes'; do
		mode=$(echo "$arm" | cut -d: -f1)
		flag=$(echo "$arm" | cut -d: -f2)
		cols=$(echo "$arm" | cut -d: -f3)
		rows=$(echo "$arm" | cut -d: -f4)
		want=$(echo "$arm" | cut -d: -f5)
		cleanup
		if ! tm new-session -d -x "$cols" -y "$rows" -s "$session" \
				-c "$startdir" \
				"'$sstq' $flag tournament 7 short novice pw; sleep 30" \
				2>/dev/null; then
			fail "sigwinch: could not start a ${cols}x${rows} tmux session for the $mode game"
			continue
		fi
		# The fallback arms have to have actually fallen back. "no" is
		# also the answer from a game that never reached initscr() at
		# all -- an unknown TERM sends tui_init() home before it, and
		# the arm would then pass having exercised none of this. The
		# full-screen arm does not cover that: it takes another branch
		# and still says "yes".
		if [ "$want" = no ] && [ "$flag" = -t ] &&
		   ! wait_scrollback 'Terminal too small'; then
			fail "sigwinch: the $mode game did not fall back, so the arm would prove nothing"
			dump_scrollback
			continue
		fi
		if ! wait_for 'COMMAND'; then
			fail "sigwinch: the $mode game never reached its command prompt"
			dump
			continue
		fi
		got=$(sigwinch_caught "$(tm display-message -p -t "$pane" '#{pane_pid}')")
		if [ "$got" = nopid ] || [ "$got" = nomask ]; then
			fail "sigwinch: could not read the $mode game's signal mask ($got)"
		elif [ "$got" != "$want" ]; then
			fail "sigwinch: the $mode game's SIGWINCH handler is '$got' where '$want' was wanted"
			dump
		fi
	done
fi

# --- and a resize does not end a game that fell back ------------------
# The fallback exists to keep the player playing on a terminal the
# panels cannot use. It printed "Terminal too small (need 72x24)" and
# then ended the game the moment the player did the obvious thing about
# that and made the window bigger:
#
#      COMMAND> rest 5000
#      Are you sure?
#      [Transmission ends.]
#
# readinput() reads the classic display with fgets, which returns NULL
# for a read that was interrupted as well as for one that reached the
# end of the input -- and the session ended either way. tui_init()'s
# giving up was what made the difference: initscr() had already run and
# installed a SIGWINCH handler, and endwin() did not take it away, so
# the resize interrupted the read, where a game started without -t
# installs no handler and survived the same resize. Measured at the
# failure: feof 0, ferror 1, errno EINTR.
#
# Past tense since #152, which puts that disposition back on the give-up
# path -- so the resize no longer reaches the read at all.
#
# Which leaves this case pinning the promise rather than either half of
# the fix. Delete the retry and it stays green, because nothing
# interrupts the read; delete the restore and it stays green too,
# because the retry catches it, as it did from #150 to #152. Only losing
# both brings the ending back. Measured, not reasoned: built with the
# retry gone and the restore kept, the whole file passes.
#
# The arms above pin the restore on its own. Nothing pins the retry by
# observation -- no signal the fallback game now carries can interrupt
# an fgets -- and it stays because reading an interrupted read as the
# end of the input is wrong whatever the signals happen to be.
#
# Any resize does it, in either direction, from any start below 72x24 --
# the grow is not what matters, the interrupted read is. 40x10 to 100x30
# is simply the shape a player would produce.
start 40 10 'tournament 7 short novice pw'
if ! wait_scrollback 'Terminal too small'; then
	fail "resize fallback: the game did not fall back to the classic display at 40x10"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "resize fallback: the classic display never reached a command prompt"
	dump
else
	# A question pending, so the resize lands while a read is waiting --
	# which is the only time the interrupted read can be mistaken for
	# the end of the input.
	tm send-keys -t "$pane" 'rest 5000' Enter
	if ! wait_for 'Are you sure'; then
		fail "resize fallback: rest did not ask whether it was wise"
		dump
	else
		tm resize-window -t "$session" -x 100 -y 30
		if ! wait_pane '#{pane_width}' 100; then
			fail "resize fallback: the terminal never resized, so nothing was tested"
			dump
		elif screen | grep -qF 'Transmission ends'; then
			fail "resize fallback: growing the terminal ended the game"
			dump
		# The question has to still be pending, not merely the game
		# alive. sst.doc promises the resize neither answers a
		# question nor turns a page, and a build that answered it
		# with an empty line would leave the game running and pass a
		# liveness check on its own.
		elif [ "$(screen | grep -c 'Are you sure')" -ne 1 ]; then
			fail "resize fallback: the question is not on screen exactly once after the resize"
			dump
		else
			# Alive, not merely silent: it still answers. Asked
			# for something only a live game produces -- after
			# the bug fired the screen still read ` COMMAND> rest
			# 5000` above `[Transmission ends.]`, so waiting for
			# `COMMAND` matched the dead game's own stale prompt
			# and only the grep above was doing any work. Nothing
			# has printed `Time Left` in this session: the classic
			# display has no panels, so it can only come from the
			# scan.
			tm send-keys -t "$pane" 'n' Enter
			tm send-keys -t "$pane" 'srscan' Enter
			expect "resize fallback: the game stopped answering after the resize" \
				'Time Left'
			# `Please answer with "Y" or "N":`, not `Beg your
			# pardon`: ja() loops until it sees y or n, so an
			# empty answer is re-asked rather than falling back
			# to the command prompt -- it never reaches the
			# unrecognised-command path. That is the one thing on
			# screen that separates a question answered by the
			# resize from one still pending, and it is looked for
			# after the poll above, so the game has had its say.
			unwanted "resize fallback: the resize answered the question with an empty line" \
				'Please answer'
		fi
	fi
fi

# --- and a resize does not answer a pause either ----------------------
# The same fault at the other reader. getch() in osx.c reads the pause
# prompt with read(), and a read the resize interrupted came back as a
# keypress -- so the resize answered `[HIT SPACE BAR TO CONTINUE]` and
# the page the player had not finished went by. At pause(1) the
# clearscreen() after it takes the screen with it.
#
# sst.doc promises against both in one sentence: resizing is safe
# "while the game waits for an answer or for a keystroke at a pause".
# Fixing only the answer half would have left the sentence half true.
#
# 60x20 so the terminal is under the panels' minimum and the help text
# is long enough to page.
start 60 20 'tournament 7 short novice pw'
if ! wait_scrollback 'Terminal too small'; then
	fail "resize pause: the game did not fall back to the classic display at 60x20"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "resize pause: the classic display never reached a command prompt"
	dump
else
	tm send-keys -t "$pane" 'help move' Enter
	if ! wait_for 'HIT SPACE BAR'; then
		fail "resize pause: help did not stop to page"
		dump
	else
		# The last line of the page being read, remembered before the
		# resize and compared with the last line after it.
		#
		# Two weaker forms of this do not work, both measured against
		# a build with the retry removed -- before #152. That
		# measurement is not reproducible now: with the SIGWINCH
		# disposition put back, a retry-removed build passes the whole
		# file, measured the same way. Like its twin above, this case
		# pins the promise rather than either half of the fix; the
		# sigwinch arms pin the restore, nothing pins getch()'s retry
		# by observation, and it stays for the reason osx.c gives --
		# reading an interrupted read as a keypress is wrong whatever
		# the signals happen to be. The two weaker forms are still
		# weaker, which is why they are recorded.
		#
		# A pause prompt on screen
		# afterwards proves nothing: an answered pause prints the
		# *next* page and stops at its own prompt, so `HIT SPACE BAR`
		# is there either way. And asking merely whether the
		# remembered line is still *somewhere* on screen proves
		# nothing either: the terminal grew, so the old page is still
		# above the new one, scrolled up rather than gone. It is the
		# position that moves -- the line above the prompt is the last
		# line of whichever page is being shown.
		lastline() {
			screen | grep -v '^ *$' | grep -v 'HIT SPACE BAR' \
				| tail -1 | sed 's/^ *//; s/ *$//'
		}
		marker=$(lastline)
		if [ -z "$marker" ]; then
			fail "resize pause: no help text on screen to remember before the resize"
			dump
		else
			# Height only, deliberately. The signal is what matters,
			# and a width change makes tmux reflow the wrapped help
			# text -- which would move `lastline` for a reason
			# having nothing to do with the fix, failing the case
			# loudly but wrongly the day the paragraph shifts.
			tm resize-window -t "$session" -x 60 -y 30
			if ! wait_pane '#{pane_height}' 30; then
				fail "resize pause: the terminal never resized, so nothing was tested"
				dump
			elif [ "$(lastline)" != "$marker" ]; then
				fail "resize pause: growing the terminal answered the pause and took the page with it"
				dump
			elif ! screen | grep -qF 'HIT SPACE BAR'; then
				fail "resize pause: the pause prompt is gone after the resize"
				dump
			else
				# And a real keystroke still answers it, so the
				# retry has not made the pause unanswerable.
				tm send-keys -t "$pane" Space
				if ! to_command; then
					fail "resize pause: the pause no longer answers to a keystroke"
					dump
				fi
			fi
		fi
	fi
fi

# --- a terminal that grew gets the panels for the next game -----------
# The full-screen decision was made once, at startup, and never
# revisited: tui_init() ran before the play-again loop, so a player who
# started too small, grew the terminal, finished the game and answered
# yes got a second classic game on a terminal that would now hold the
# panels. Nothing on screen said so -- the startup notice had scrolled
# away a game ago. #154.
#
# The grow lands while the play-again question is pending, which is the
# shape a player produces: the notice tells them the terminal is too
# small, and making it bigger is the next thing they do about that.
#
# 70x20 is under the 72x24 floor, and 100x30 is clear of it.
start 70 20 'tournament 7 short novice pw'
if ! wait_scrollback 'Terminal too small'; then
	fail "regrown: the game did not fall back to the classic display at 70x20"
	dump_scrollback
elif ! wait_scrollback 'the next game gets panels'; then
	# The second line of the notice, which nothing used to assert --
	# so its wording could drift, and did, without a test noticing.
	# It is what the game tells a player about a terminal turned down
	# for size alone -- the pinned refusals have advice of their own,
	# which the cases above assert.
	fail "regrown: the notice does not say a bigger terminal gets the next game panels"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "regrown: the classic display never reached a command prompt"
	dump
else
	# That the first game is classic needs no check of its own: the
	# notice above prints only where tui_init() gave up, and it
	# returns FALSE there, so the panels cannot be up. Checking the
	# screen for " Quadrant " instead was wrong twice over -- the
	# classic display prints `Entering Quadrant 5 - 3` from cramlc()
	# on any move between quadrants, so it fires on a correct build.
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "regrown: quit did not reach the end-of-game questions"
		dump
	else
		# No: yes stops to ask for a file name and strands the test
		# there. Same reason as the replay case above.
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "regrown: never asked about another game"
			dump
		else
			tm resize-window -t "$session" -x 100 -y 30
			if ! wait_pane '#{pane_width}' 100; then
				fail "regrown: the terminal never resized, so nothing was tested"
				dump
			else
				tm send-keys -t "$pane" 'y' Enter
				if ! wait_for 'regular, tournament, or frozen'; then
					fail "regrown: the second game never started"
					dump
				else
					tm send-keys -t "$pane" 'regular' Enter
					expect "regrown: the second game lost the first answer" \
						'Short, Medium, or Long'
					tm send-keys -t "$pane" 'short' Enter
					expect "regrown: setup did not reach the skill question" \
						'Novice, Fair, Good, Expert'
					tm send-keys -t "$pane" 'novice' Enter
					sleep 0.5
					tm send-keys -t "$pane" 'xyz' Enter
					if ! to_command; then
						fail "regrown: the second game never reached its prompt"
						dump
					else
						# Both panels, not just the frame: an
						# empty box would also appear if the
						# retry came up but the game state
						# never reached it.
						#
						# These two failed before the retry
						# existed, and again on a copy built
						# with tui_init()'s ioctl size query
						# deleted -- a second initscr() reports
						# the size the first one was turned
						# down at, so the retry turns itself
						# down.
						want_quadtitle "regrown: the second game has no panels on a terminal that now fits"
						expect_status "regrown: the status panel is empty in the second game" \
							"Stardate"
						# And the panels that came back follow a
						# resize like any others. This is the
						# arm that pins the SIGWINCH handler
						# tui_init() re-installs on a retry:
						# curses installs one per process, so
						# the second initscr() gets none, and
						# without it no KEY_RESIZE ever arrives
						# and sync_size() is never told the
						# terminal moved. The panels would sit
						# at 100 columns on a 110-column screen,
						# which is what column 110 is asked
						# about -- blank if the display never
						# grew, the status panel's right border
						# if it did.
						#
						# Proved by deleting that re-install
						# and running this file on a copy of
						# the tree outside the repository: this
						# check alone failed, and nothing else
						# in the file noticed.
						tm resize-window -t "$session" -x 110 -y 34
						if ! wait_pane '#{pane_width}' 110; then
							fail "regrown: the terminal never grew again, so the resize went untested"
							dump
						elif ! wait_col_dirty 110 2 5; then
							fail "regrown: the panels do not follow a resize after coming back"
							dump
						fi
					fi
				fi
			fi
		fi
	fi
fi

# --- a pinned dimension is not a pinned terminal ----------------------
# ncurses honours LINES and COLUMNS one at a time, so the arm above --
# where both are pinned -- says nothing about the far commoner case of
# one. COLUMNS alone is exported by plenty of Docker images, CI runners
# and shell profiles, and it is usually the right width; height is what
# is normally wrong. Taking either variable to pin the whole terminal
# would stand the retry down for all of them, silently.
#
# LINES= is exported empty on purpose. ncurses ignores any value that
# does not parse whole to a positive int -- empty, 0, negative,
# trailing rubbish -- and takes the terminal instead, so testing merely
# that the variable is set is stricter than ncurses and would stand the
# retry down again. This arm fails under either mistake.
#
# 80x20 is short and wide enough that only the free dimension has to
# move: growing to 80x30 clears 72x24 without the pinned width changing.
#
# This check was written before the fix and failed on the build that
# took either variable to pin the whole terminal. It fails under the
# set-or-not test too, which is the other mistake it is here for.
start 80 20 'tournament 7 short novice pw' 'env LINES= COLUMNS=80'
if ! wait_scrollback 'Terminal too small'; then
	fail "one pinned: 80x20 did not refuse the panels"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "one pinned: the classic display never reached a command prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "one pinned: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "one pinned: never asked about another game"
			dump
		else
			tm resize-window -t "$session" -x 80 -y 30
			if ! wait_pane '#{pane_height}' 30; then
				fail "one pinned: the terminal never resized, so nothing was tested"
				dump
			else
				tm send-keys -t "$pane" 'y' Enter
				if ! wait_for 'regular, tournament, or frozen'; then
					fail "one pinned: the second game never started"
					dump
				else
					tm send-keys -t "$pane" 'regular' Enter
					expect "one pinned: the second game lost the first answer" \
						'Short, Medium, or Long'
					tm send-keys -t "$pane" 'short' Enter
					expect "one pinned: setup did not reach the skill question" \
						'Novice, Fair, Good, Expert'
					tm send-keys -t "$pane" 'novice' Enter
					sleep 0.5
					tm send-keys -t "$pane" 'xyz' Enter
					if ! to_command; then
						fail "one pinned: the second game never reached its prompt"
						dump
					else
						want_quadtitle "one pinned: a pinned width stood the whole retry down"
					fi
				fi
			fi
		fi
	fi
fi

# --- an oversize pin is worded as one at the play-again prompt too -----
# The retry's notice quotes the size of the refusal, and since #164 made
# an oversize pin a refusal that size can be curses' rather than the
# window's. Left alone, a player who did exactly what the startup notice
# asked -- grow the terminal -- was answered with `Terminal is 190x30 --
# need 72x24, staying classic.`: a size that is not their window's, and
# a requirement the quoted size plainly clears.
#
# 100x20 with COLUMNS=190 refuses for height at startup, so the player
# gets the ordinary grow-the-terminal advice; growing to 100x30 clears
# that and leaves only the oversize width, which is the collision.
start 100 20 'tournament 7 short novice pw' 'env COLUMNS=190'
if ! wait_scrollback 'Terminal too small'; then
	fail "oversize retry: 100x20 did not refuse for height at startup"
	dump_scrollback
elif ! wait_scrollback 'Grow to 72x24, unset COLUMNS'; then
	# Both actions in one line, because at 100x20 with COLUMNS=190 the
	# window is short *and* the width is over it. Unsetting alone
	# leaves a 20-row window, and growing alone would need 190
	# columns, which nothing here drives -- so both actions really
	# are wanted in this shape. What the line must not say is "grow
	# the terminal and the next game gets panels", which the retry
	# below cannot keep.
	fail "oversize retry: startup promised panels a pinned width cannot give"
	dump_scrollback
elif scrollback | grep -qF 'the next game gets panels'; then
	fail "oversize retry: startup gave the grow-it advice over a pinned width"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "oversize retry: the classic display never reached a prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "oversize retry: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "oversize retry: never asked about another game"
			dump
		else
			tm resize-window -t "$session" -x 100 -y 30
			if ! wait_pane '#{pane_height}' 30; then
				fail "oversize retry: the terminal never resized, so nothing was tested"
				dump
			else
				tm send-keys -t "$pane" 'y' Enter
				if ! wait_for 'regular, tournament, or frozen'; then
					fail "oversize retry: the second game never started"
					dump
				# Proved by putting the old single-branch
				# notice back, on a copy of the tree outside
				# the repository: this check failed. It greps
				# for "LINES/COLUMNS make it" rather than
				# "Terminal is" because the old wording
				# contains the latter. The check below never
				# ran on that build -- the elif chain stops
				# here -- and would have failed too, the old
				# wording being exactly what it forbids.
				elif ! wait_scrollback 'LINES/COLUMNS make it'; then
					fail "oversize retry: the pinned size was not named at the play-again prompt"
					dump_scrollback
				elif scrollback | grep -qF 'need 72x24, staying classic'; then
					fail "oversize retry: asked for a 72x24 the terminal already has"
					dump_scrollback
				fi
			fi
		fi
	fi
fi

# --- and resizing the pinned axis itself is an act, not silence -------
# The "oversize retry" arm resizes the free axis. This one resizes the
# pinned one, which used to be skipped outright: at the size below,
# curses' 190 and this pane's 100 are held apart for good, so comparing
# them said "moved" every game, and the skip that fixed it took the real
# move with it. A player who widened the window was then treated as
# having done nothing -- the one case where the game watches a player
# act and says nothing back. #169.
#
# Scoped to this pane on purpose: a pin *equal* to the window compared
# perfectly well, and axis_moved()'s comment in tui.c has the general
# statement. It is the pin that differs that does the damage.
#
# COLUMNS=190 in a 100x30 pane clears the floor and is refused only for
# being bigger than the window, so width is both the pinned axis and the
# one to blame. Widening the pane to 150 moves that axis and nothing
# else: the height is free and never changes, so no comparison but the
# pinned one can produce a notice here.
#
# 190 is still over 150, so the retry refuses again and both halves of
# the notice have something to say -- the first line from the window,
# the second from the blame. The report is asserted by the new number,
# which startup's 100x30 line does not contain; the advice by its second
# copy, the first being startup's.
start 100 30 'tournament 7 short novice pw' 'env COLUMNS=190'
if ! wait_scrollback 'LINES/COLUMNS make it'; then
	fail "pinned axis: 100x30 with COLUMNS=190 was not refused as oversize"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "pinned axis: the classic display never reached a command prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "pinned axis: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "pinned axis: never asked about another game"
			dump
		else
			tm resize-window -t "$session" -x 150 -y 30
			if ! wait_pane '#{pane_width}' 150; then
				fail "pinned axis: the terminal never widened, so nothing was tested"
				dump
			else
				tm send-keys -t "$pane" 'y' Enter
				# The whole notice is printed before setup()
				# asks anything, so waiting for the question
				# is what makes the two checks below race
				# neither line of it.
				#
				# The size-report check below failed on the
				# build this was written against -- the skip
				# that #169 is about -- so this arm had its
				# failing first run rather than needing one
				# planted. Only that one: the advice count
				# after it never ran, the elif chain stopping
				# at the first failure, and it would have
				# failed too, there being no second notice at
				# all to carry either line.
				if ! wait_for 'regular, tournament, or frozen'; then
					fail "pinned axis: the second game never started"
					dump
				elif ! scrollback | grep -qF 'Terminal is 150x30 but LINES/COLUMNS make it 190x30.'; then
					fail "pinned axis: widening the pinned axis got no notice at all"
					dump_scrollback
				elif [ "$(scrollback_count 'Unset COLUMNS, rerun sst -t -- classic for now.')" -lt 2 ]; then
					fail "pinned axis: the notice reported the size but gave no advice"
					dump_scrollback
				fi
			fi
		fi
	fi
fi

# --- the same on the other axis, which is a different call site --------
# The "pinned axis" and "pinned unchanged" arms pin the width, so
# axis_moved()'s pinned branch is reached there only through its COLUMNS
# call; the LINES call runs in them too, but on the free branch. Three
# arms reach that call with the height pinned -- "env size"
# (LINES=20 COLUMNS=70), "pinned height unchanged" below, and this one
# -- and the first two never resize, so their pinned LINES call only
# ever answers FALSE. This is the only arm where it answers TRUE, and
# answering TRUE is what a build that skips a pinned axis cannot do:
# measured against main's tui.c, on a copy of the tree outside the
# repository, this arm failed there.
#
# What it does not do is tell which pair of statics that call is handed,
# and the first version of this comment claimed it did. At these numbers
# every comparison in play answers "moved", so the notice prints
# whichever pair is read -- measured, with refusedcols and
# refusedtermcols substituted at the LINES site, this arm passes
# unchanged. Call that mutation the transposed pair; it is caught by the
# silence arms instead, which need a comparison that answers FALSE, and
# it fails exactly three of them: "pinned height unchanged" below,
# "pinned unchanged" after it, and the older "unchanged" arm at the end
# of the file.
#
# LINES=40 in a 100x30 pane is the height mirror of the "pinned axis"
# arm: it clears the floor and is refused only for being taller than
# the window. Growing the pane to 35 rows moves the pinned axis and
# nothing else, and 40 is still over 35, so the retry refuses again and
# names the height.
#
# Asserted by the new number, which the startup notice (100x30) does not
# carry, and by the advice naming LINES -- blames() leaves a COLUMNS of
# 100 in a 100-column window unnamed.
start 100 30 'tournament 7 short novice pw' 'env LINES=40'
if ! wait_scrollback 'LINES/COLUMNS make it 100x40.'; then
	fail "pinned height: 100x30 with LINES=40 was not refused as oversize"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "pinned height: the classic display never reached a command prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "pinned height: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "pinned height: never asked about another game"
			dump
		else
			tm resize-window -t "$session" -x 100 -y 35
			if ! wait_pane '#{pane_height}' 35; then
				fail "pinned height: the terminal never grew, so nothing was tested"
				dump
			else
				tm send-keys -t "$pane" 'y' Enter
				# Proved by building main's tui.c against this
				# file, on a copy of the tree outside the
				# repository: the size-report check below
				# failed, the pinned axis being skipped
				# there. Only that one -- the advice count
				# after it never ran, the elif chain stopping
				# at the first failure, and it would have
				# failed too, there being no second notice at
				# all to carry either line.
				if ! wait_for 'regular, tournament, or frozen'; then
					fail "pinned height: the second game never started"
					dump
				elif ! scrollback | grep -qF 'Terminal is 100x35 but LINES/COLUMNS make it 100x40.'; then
					fail "pinned height: growing the pinned height got no notice at all"
					dump_scrollback
				elif [ "$(scrollback_count 'Unset LINES, rerun sst -t -- classic for now.')" -lt 2 ]; then
					fail "pinned height: the notice named the wrong variable or none"
					dump_scrollback
				fi
			fi
		fi
	fi
fi

# --- and a pinned height nobody touched is still silence ---------------
# The half the "pinned height" arm cannot prove, and the gap was not
# theoretical: measured, a build whose pinned LINES branch always
# answers "moved" passed the whole of this file before this arm existed,
# while showing a player who changed nothing the notice a second time.
# That is the defect #165 was for, surviving on the height because the
# one other arm that pins it cannot fail.
#
# That arm is "env size", which is already in this scenario and does not
# notice: the line it greps for is one its pane cannot print -- #176.
# This arm does not fix that one; it covers the axis, and leaves #176 to
# be fixed where it lives.
start 100 30 'tournament 7 short novice pw' 'env LINES=40'
if ! wait_scrollback 'LINES/COLUMNS make it 100x40.'; then
	fail "pinned height unchanged: 100x30 with LINES=40 was not refused"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "pinned height unchanged: the classic display never reached a prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "pinned height unchanged: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "pinned height unchanged: never asked about another game"
			dump
		else
			tm send-keys -t "$pane" 'y' Enter
			# Exactly one, for the reason its width sibling
			# gives: a capture that came back empty counts 0
			# and would pass a -gt test proving nothing.
			#
			# Proved by pinned-LINES-always-moved: prefixing
			# the LINES call site with pinned("LINES") ||,
			# so a pinned height reports moved without
			# axis_moved() being consulted at all. On a copy
			# of the tree outside the repository that failed
			# this check alone, where before this arm existed
			# the same build passed the whole file.
			# Not called a skip: that is this file's and
			# tui.c's word for main's !pinned(...) &&, which
			# is the opposite behaviour, suppressing the
			# notice where this forces it. A reader who
			# rebuilt the break from the wrong word would be
			# testing main's bug, not this one. Distinct
			# again from pinned-always-moved, which is both
			# axes and fails two, and from the transposed
			# pair, which fails three.
			if ! wait_for 'regular, tournament, or frozen'; then
				fail "pinned height unchanged: the second game never started"
				dump
			elif [ "$(scrollback_count 'LINES/COLUMNS make it')" -ne 1 ]; then
				fail "pinned height unchanged: the terminal did not move and was told about it anyway"
				dump_scrollback
			fi
		fi
	fi
fi

# --- and a pinned axis nobody touched is still silence -----------------
# The mirror of the "pinned axis" arm, and the half that one cannot
# prove on its own: a comparison that simply answered "moved" for every
# pinned axis would satisfy it. That is not a hypothetical shape -- it
# is what comparing curses' number against the terminal's does wherever
# the two differ, which at COLUMNS=190 in a 100-column pane is always,
# and it is the failure the skip was put in to stop.
#
# Same 100x30 pane and same COLUMNS=190 as that arm, so the refusal and
# the blame are its; only the resize is missing. Counted rather than
# searched for, because the startup notice has already put one copy of
# every line in the scrollback and it is the second that would be the
# bug.
#
# It holds that contract for a pinned width; "pinned height unchanged"
# above holds it for a pinned height. The "env size" arm earlier in this
# file looks like it holds it too and cannot: it greps a 100x30 pane for
# "staying classic", which is the wording of a branch that pane can
# never take. #176.
start 100 30 'tournament 7 short novice pw' 'env COLUMNS=190'
if ! wait_scrollback 'LINES/COLUMNS make it'; then
	fail "pinned unchanged: 100x30 with COLUMNS=190 was not refused as oversize"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "pinned unchanged: the classic display never reached a command prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "pinned unchanged: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "pinned unchanged: never asked about another game"
			dump
		else
			tm send-keys -t "$pane" 'y' Enter
			# Waited for by this check, so the game has had
			# every chance to print a second notice first.
			#
			# Proved by pinned-always-moved: replacing
			# axis_moved()'s pinned branch with a bare
			# return TRUE, and running this file on a copy
			# of the tree outside the repository. Two checks
			# fail under it, this one and "pinned height
			# unchanged" above; the two resize arms pass,
			# asking for a notice and getting one, which is
			# the whole reason the silence arms are here.
			# Not the same break as pinned-LINES-always-moved,
			# which that arm names and which is this one
			# narrowed to the height, nor as the transposed
			# pair: those catch one arm and three.
			if ! wait_for 'regular, tournament, or frozen'; then
				fail "pinned unchanged: the second game never started"
				dump
			# Exactly one, not "no more than one": a capture
			# that came back empty counts 0 and would pass a
			# -gt test while proving nothing. This way the
			# startup notice has to still be there.
			elif [ "$(scrollback_count 'LINES/COLUMNS make it')" -ne 1 ]; then
				fail "pinned unchanged: the terminal did not move and was told about it anyway"
				dump_scrollback
			fi
		fi
	fi
fi

# --- and a terminal grown but not grown enough is told so --------------
# The notice tells the player to grow the terminal. A player who does
# that and misses -- 80x22, or "80x24" inside tmux, where the status bar
# leaves the pane 23 rows -- used to get classic again in silence, and
# no way to tell a mis-sized window from a broken promise.
#
# Silence is still right for a player who did nothing: they were told at
# startup and nothing has changed since. So the line prints only where
# the terminal is a different size than the one that was turned down.
# That is the discrimination this case exists for, which is why it
# resizes rather than simply answering yes at 70x20.
#
# 80x22 is wide enough and one row short, so it fails on height alone.
start 70 20 'tournament 7 short novice pw'
if ! wait_scrollback 'Terminal too small'; then
	fail "half grown: the game did not fall back to the classic display at 70x20"
	dump_scrollback
elif ! wait_for 'COMMAND'; then
	fail "half grown: the classic display never reached a command prompt"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "half grown: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "half grown: never asked about another game"
			dump
		else
			tm resize-window -t "$session" -x 80 -y 22
			if ! wait_pane '#{pane_height}' 22; then
				fail "half grown: the terminal never resized, so nothing was tested"
				dump
			else
				tm send-keys -t "$pane" 'y' Enter
				if ! wait_for 'regular, tournament, or frozen'; then
					fail "half grown: the second game never started"
					dump
				# Said again, because the player acted on what
				# they were told and it did not work. This
				# check was written before the line existed
				# and failed on the build without it.
				#
				# The size it measured, not just the size it
				# wants: a player who grew a tmux pane to
				# "80x24" and lost the row to the status bar
				# has been told 72x24 twice already and
				# believes their window is right. 80x22 is
				# that mistake, one row further out. Asserting
				# the number is what makes this a check of
				# what the game measured rather than of what
				# it was compiled to want.
				elif ! wait_scrollback 'Terminal is 80x22'; then
					fail "half grown: the game did not say what size it measured"
					dump_scrollback
				elif ! wait_scrollback 'staying classic'; then
					fail "half grown: nothing said it is staying in the classic display"
					dump_scrollback
				fi
			fi
		fi
	fi
fi

# --- but a player who did not resize is not told twice -----------------
# The mirror of the case above, and the reason it cannot be satisfied by
# printing the line every time. Nothing changed, so the notice from
# startup still stands and repeating it is noise.
start 70 20 'tournament 7 short novice pw'
if ! wait_for 'COMMAND'; then
	fail "unchanged: the classic display never reached a command prompt at 70x20"
	dump
else
	tm send-keys -t "$pane" 'quit' Enter
	if ! wait_for 'score recorded'; then
		fail "unchanged: quit did not reach the end-of-game questions"
		dump
	else
		tm send-keys -t "$pane" 'n' Enter
		if ! wait_for 'play again'; then
			fail "unchanged: never asked about another game"
			dump
		else
			tm send-keys -t "$pane" 'y' Enter
			if ! wait_for 'regular, tournament, or frozen'; then
				fail "unchanged: the second game never started"
				dump
			else
				# Waited for by the check above, so the game
				# has had every chance to print it.
				#
				# Proved by dropping the resized test from
				# sst.c, so the line printed on every failed
				# retry, and running this file on a copy of
				# the tree outside the repository: this check
				# failed. It was the only one when that was
				# written; the pinned pair added for #169 --
				# "pinned unchanged" and "pinned height
				# unchanged" -- hold the same contract under a
				# pin, so the break now fails three, measured
				# again on that branch. This one is the arm
				# with no pin at all.
				# Through the scrollback, not the visible
				# pane: "staying classic" appears nowhere
				# else in this journey, so the wider search
				# is free and cannot go vacuous the day
				# setup() prints another line or two.
				if scrollback | grep -qF 'staying classic'; then
					fail "unchanged: the terminal did not move and was told about it anyway"
					dump_scrollback
				fi
			fi
		fi
	fi
fi

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'tui OK\n'
