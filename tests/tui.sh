#!/bin/sh
# Drives the full-screen display through a real terminal.
#
# Everything else in the suite talks to the game through pipes, which
# is enough for the fallback path but never exercises curses itself.
# This one runs the game inside tmux so there is a genuine pty, and
# reads the screen back the way a player would see it.
#
# The size matters. 80x24 is what most terminal emulators open at and
# 72x24 is the smallest the TUI accepts; at both the message window is
# only ten lines, so the startup banner overflows it -- which is how
# the pager came to be waiting for a keystroke before the game had
# asked the player anything, and to eat the first letter of the answer.
#
# Usage: tui.sh /path/to/sst
# Exits 77 (ctest SKIP_RETURN_CODE) when there is no tmux to run in.
#
# Needs a sleep that takes a fraction, which GNU and BSD both have but
# POSIX does not promise; busybox would need whole seconds.

set -u

SST="${1:-./sst}"
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
trap 'cleanup' EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM
trap 'cleanup; exit 129' HUP

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

screen() { tm capture-pane -t "$pane" -p 2>/dev/null; }

dump() {
	printf '  --- screen ---\n' >&2
	screen | sed 's/^/  | /' >&2
}

# Wait until the screen shows $1, up to about eight seconds. Polling
# beats a fixed sleep: curses paints in its own time, and a machine
# under load should not turn into a failure.
wait_for() {
	i=0
	while [ "$i" -lt 80 ]; do
		if screen | grep -qF "$1"; then return 0; fi
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
	screen | grep -qF "$2" || { fail "$1"; dump; }
}

unwanted() {
	if screen | grep -qF "$2"; then fail "$1"; dump; fi
}

# tmux takes a shell command line, not an argument list, so the path
# has to survive being quoted into one.
sstq=$(printf "%s" "$SST" | sed "s/'/'\\\\''/g")

# Start the game on a terminal $1 wide by $2 tall, passing $3 (if any)
# on the command line. It reads sst.doc from the current directory, and
# dies with the session.
start() {
	cleanup
	tm new-session -d -x "$1" -y "$2" -s "$session" -c "$srcdir" \
		"'$sstq' -t ${3:-}; sleep 30" 2>/dev/null || {
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
wait_for 'Novice, Fair, Good, Expert' || fail "setup did not reach the skill question"
tm send-keys -t "$pane" 'novice' Enter
sleep 0.5
tm send-keys -t "$pane" 'xyz' Enter
if ! to_command; then
	fail "the game never reached its command prompt"
	dump
	exit 1
fi

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
to_command || fail "the manual never finished paging"

# The game is still listening. Long-range scan says something the
# always-on panels never do, so this cannot pass on their text.
tm send-keys -t "$pane" 'lrscan' Enter
if ! wait_for 'Long-range scan for Quadrant'; then
	fail "the game did not carry on after the paging prompt"
	dump
fi

# --- setup answered on the command line ------------------------------
# The same bug reaches further than the banner. With the setup answers
# in argv the player never types during setup at all, so the game runs
# on into the briefing -- and a pause there waits for, and eats, the
# first character of their first command.
#
# The briefing's fixed text is already longer than the window, so the
# case does not rest on the galaxy that gets generated; the seed only
# decides how much further past the threshold it goes, and rand()
# differing between C libraries can blunt that but not invert it. A
# pause before the player has touched the keyboard is wrong either way.
start 80 24 'tournament 5 long emeritus xyz'
if ! wait_for 'COMMAND'; then
	fail "argv setup: the game never reached its command prompt"
	dump
else
	unwanted "argv setup: paused before the player had typed anything" "CONTINUE"
	unwanted "argv setup: paused before the player had typed anything" "HIT SPACE BAR"
	# Long-range scan rather than short: its heading appears only in
	# the message window, so waiting for it proves the command was
	# read whole. Sampling the screen after a fixed sleep instead
	# would pass on a slow machine that had not repainted yet.
	tm send-keys -t "$pane" 'lrscan' Enter
	if ! wait_for 'Long-range scan for Quadrant'; then
		fail "argv setup: the first command was not accepted"
		dump
	fi
	unwanted "argv setup: the first command lost a character" "UNRECOGNIZED"
fi

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'tui OK\n'
