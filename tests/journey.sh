#!/bin/sh
# Plays a short scripted game in plain mode and checks the output.
#
# This guards the rule in AGENTS.md that plain mode (no -t) keeps
# working: a stray printf, a TUI change leaking into plain mode, or a
# prompt that stops consuming input all show up here. The byte ceiling
# catches the input-exhausted reprompt loop, which floods stdout.
#
# Only commands that pass no game time are used. Anything that advances
# the clock can schedule an event, and an event's [HIT SPACE BAR]
# prompt eats a byte of the piped script and desynchronizes every
# command after it. A navigation/combat journey needs a seeded
# tournament game instead.
#
# Usage: journey.sh /path/to/sst [build-type]

set -u

SST="${1:-./sst}"
BUILD_TYPE="${2:-}"
# Anything further is passed to the game. Running the same journey with
# -t is how we check that full-screen mode falls back to this one when
# there is no terminal: not byte-identical output (the galaxy is seeded
# from the clock, and the fallback adds a notice), but the same
# assertions, exit 0, and no escape sequences.
case "$BUILD_TYPE" in
	Debug|Release|RelWithDebInfo|MinSizeRel|"") ;;
	*)
		echo "usage: journey.sh <sst> [build-type] [game args...]" >&2
		echo "  '$BUILD_TYPE' is not a build type; game arguments go after it" >&2
		exit 1
		;;
esac
[ "$#" -ge 1 ] && shift	# drop the binary
[ "$#" -ge 1 ] && shift	# drop the build type; "$@" is now the game's
GAME_ARGS="$*"

MAXBYTES=1048576
DUMPBYTES=8192

# An explicit template is the one form both GNU and BSD mktemp accept.
work=$(mktemp -d "${TMPDIR:-/tmp}/sst-journey.XXXXXX")
if [ -z "${work:-}" ] || [ ! -d "$work" ]; then
	echo "FAIL: could not create a temporary directory" >&2
	exit 1
fi
trap 'rm -rf "$work"' EXIT
trap 'rm -rf "$work"; exit 130' INT
trap 'rm -rf "$work"; exit 143' TERM
trap 'rm -rf "$work"; exit 129' HUP
# PIPE too: piping this script into `head` would otherwise kill it
# outright, before any other trap runs.
trap 'rm -rf "$work"; exit 141' PIPE
out="$work/out.txt"
rc="$work/rc.txt"

# Bound the run in wall-clock time too, where coreutils timeout exists.
run() {
	if command -v timeout >/dev/null 2>&1; then
		timeout 60 "$@"
	else
		"$@"
	fi
}

# Head caps the capture so a runaway game cannot fill the disk; the
# subshell records the game's own exit status before that cap applies.
(
	printf 'regular\nshort\nnovice\nxyz\nsrscan\nstatus\nlrscan\nchart\ndamages\nreport\nscore\ncommands\nbogus\nquit\nn\n' \
		| run "$SST" "$@" 2>&1
	echo $? > "$rc"
) | head -c "$MAXBYTES" > "$out"

status=$(cat "$rc" 2>/dev/null || echo "no-status")
bytes=$(wc -c < "$out" | tr -d ' ')	# BSD wc pads its output

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

want() {
	if ! grep -q "$2" "$out"; then
		fail "$1 (expected output to contain: $2)"
	fi
}

want_re() {
	if ! grep -qE "$2" "$out"; then
		fail "$1 (expected output to match: $2)"
	fi
}

want_count() {
	got=$(grep -c "$3" "$out")
	if [ "$got" -lt "$2" ]; then
		fail "$1 (expected at least $2 lines matching '$3', found $got)"
	fi
}

if [ "$status" != "0" ]; then
	fail "game exited with status $status, want 0"
fi
if [ "$bytes" -ge "$MAXBYTES" ]; then
	fail "game produced at least $MAXBYTES bytes -- runaway output?"
fi

# Startup and the command loop.
want "startup banner missing" "SUPER- STAR TREK"
want "setup did not ask for game length" "Short, Medium, or Long"
want "command prompt missing" "COMMAND>"

# srscan: column header plus a grid row, so an empty grid is caught.
want "short-range scan grid header missing" "1 2 3 4 5 6 7 8 9 10"
want_re "short-range scan grid rows missing" "^10  "

# status: these lines start at column 1, unlike the copies srscan
# prints beside the grid, so they are attributable to the command.
want_re "status report missing stardate" "^ Stardate"
want_re "status report missing condition" "^ Condition"
want_re "status report missing klingon count" "^ Klingons Left"

# The remaining read-only reports, all converted to proutf for the TUI.
want "long-range scan missing" "Long-range scan for Quadrant"
want "star chart missing" "STAR CHART FOR THE KNOWN GALAXY"
want "damage report missing" "All devices functional."
want "status report command missing" "Klingon ships have been destroyed"
want "score report missing" "TOTAL SCORE"

# The command table prints for both `commands` and an unknown command,
# so require both occurrences rather than just one.
want_count "command list missing" 2 "SRSCAN    MOVE"
want "bad command was not rejected" "UNRECOGNIZED COMMAND"

# Plain mode must not emit terminal escape sequences -- curses output
# leaking out of the TUI is exactly what AGENTS.md forbids. The pager's
# clearscreen() does legitimately emit one, but this journey never
# pages; revisit if a future journey does.
if tr -cd '\033' < "$out" | grep -q .; then
	fail "escape sequences leaked into plain-mode output"
fi

# A line ending in a carriage return is a line-ending leak from a file
# the game read. Harmless-looking in plain mode, but under curses the
# CR sends the cursor home and the newline then wipes the line -- which
# is how the whole in-game manual once came out blank. The pager's own
# "\r<spaces>\r" wipe does end a line, so drop that shape first.
CR=$(printf '\r')
if sed "s/$CR *$CR\$//" "$out" | grep -q "$CR\$"; then
	fail "a line ends with a carriage return -- CRLF leaking from a file the game read"
fi

# Asking for -t without a terminal must say so, not silently ignore the
# flag: without this the fallback notice could be deleted and the test
# would still pass.
case " $GAME_ARGS " in
	# "classic display", not "using the classic display": the
	# too-small-terminal branch words it without the "the".
	*" -t "*) want "no fallback notice for -t" "classic display" ;;
esac

# No build may show the developer chatter to a player who did not ask
# for it -- the "debug" password is what asks. This once printed in
# every debug build and, in one case, in release builds too; harmless-
# looking until proutf began counting the newlines it writes, at which
# point it started costing whole pages of the briefing and of a fight.
# Not anchored: the first such line lands on the end of a prompt line.
#
# Opportunistic, because this journey plays a random galaxy: whether
# any base placement needs a second try is chance. It printed in ten of
# twelve games measured, so it does bite -- but a seeded journey (#12)
# is what would make it certain, and the combat check below needs one
# to bite at all, since nothing here takes a hit.
if grep -q 'DEBUG:' "$out"; then
	fail "developer chatter printed for a player who did not ask for it"
fi
if grep -qE 'Hit [0-9][^ ]* energy|Prob = [0-9]+ \(' "$out"; then
	fail "combat diagnostics printed for a player who did not ask for them"
fi

# The game has to finish because the journey finished, not because the
# script ran dry. Both endings print the farewell, so without this a
# journey that stops short -- an input desync, a prompt that stopped
# consuming -- would read as a pass.
if grep -q "Transmission ends" "$out"; then
	fail "the game ran out of input before the journey ended"
fi

# The farewell has to be the end of the game, not merely present.
last=$(grep -v '^[[:space:]]*$' "$out" | tail -n 1)
case "$last" in
	*"Great Bird of the Galaxy"*) ;;
	*) fail "game did not sign off cleanly (last line was: $last)" ;;
esac

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed. Captured %s bytes, showing at most %d:\n' \
		"$fails" "$bytes" "$DUMPBYTES" >&2
	head -c "$DUMPBYTES" "$out" >&2
	exit 1
fi

printf 'journey OK (%s bytes)\n' "$bytes"
