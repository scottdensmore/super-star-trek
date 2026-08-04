#!/bin/sh
# Plays fixed journeys and compares every byte against a recording.
#
# The rest of the suite asks whether things happened: that a game
# docked, that nothing desynchronised, that no developer chatter got
# out. None of it asks whether a number is right. Nothing checked that
# combat arithmetic, time accounting, a starbase's resupply or the
# score came out as they should -- so a change to any of them was
# invisible unless it crashed or stopped a journey.
#
# This does the other thing. Each journey below is recorded in
# tests/golden/, and any difference at all is a failure. It cannot say
# whether the game is correct; it says exactly what changed, which is
# the question a reviewer actually has.
#
# What it will not catch is a change that alters no output. Adding a
# unit to the torpedo damage still destroys the same Klingon and prints
# the same line, and nothing here notices; changing it enough that the
# Klingon survives shows up at once. The one-character fix to the
# Super-commander's pursuit (#42) showed up here as four lines of the
# combat recording, which was the whole of its visible effect. These
# are recordings of what the game does, not proofs of what it should.
#
# It only works because a tournament number now names the same galaxy
# on every machine (see Rand() in sst.c). Before that these recordings
# would have been per-platform, which is why the seeded battery next
# door has to ask its questions in aggregate.
#
# When a change alters output on purpose, re-record and say so in the
# commit -- the diff of the fixtures is the evidence:
#
#     tests/golden.sh ./build/sst --update
#
# Usage: golden.sh /path/to/sst [--update]

set -u

if [ $# -lt 1 ]; then
	echo "usage: golden.sh <sst> [--update]" >&2
	exit 2
fi
SST="$1"
UPDATE="${2:-}"
case "$UPDATE" in
	""|--update) ;;
	*) echo "usage: golden.sh <sst> [--update]" >&2; exit 2 ;;
esac
case "$SST" in
	/*) ;;
	*) SST="$(pwd)/$SST" ;;
esac
if [ ! -x "$SST" ]; then
	echo "FAIL: $SST is not executable" >&2
	exit 1
fi
srcdir=$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)
golden="$srcdir/tests/golden"

MAXBYTES=262144
MINBYTES=1000		# a banner and a farewell alone come to about 700
DIFFLINES=40
CR=$(printf '\r')	# BSD sed reads a regex \r as a literal r

work=$(mktemp -d "${TMPDIR:-/tmp}/sst-golden.XXXXXX")
if [ -z "${work:-}" ] || [ ! -d "$work" ]; then
	echo "FAIL: could not create a temporary directory" >&2
	exit 1
fi
trap 'rm -rf "$work"' EXIT
trap 'rm -rf "$work"; exit 130' INT
trap 'rm -rf "$work"; exit 143' TERM
trap 'rm -rf "$work"; exit 129' HUP
trap 'rm -rf "$work"; exit 141' PIPE

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

run() {
	if command -v timeout >/dev/null 2>&1; then
		timeout 60 "$@"
	else
		"$@"
	fi
}

# $1 fixture name, $2 the whole script fed to the game, $3 the
# directory to play in (defaults to one of its own).
#
# A directory each, because a journey that freezes writes a save file
# beside itself and the next one must not find it -- except for the
# pair that is meant to, which passes the same name twice.
play() {
	name="$1"
	dir="$work/${3:-$name}"
	got="$work/$name.txt"
	rc="$work/$name.rc"
	mkdir -p "$dir"
	( cd "$dir" && printf '%s' "$2" | run "$SST" 2>&1; echo $? > "$rc" ) |
		head -c "$MAXBYTES" > "$got"

	status=$(cat "$rc" 2>/dev/null || echo "no-status")
	bytes=$(wc -c < "$got" | tr -d ' ')

	# Checked whether recording or comparing. A recording is only
	# worth having of a game that played properly; a comparison that
	# matches byte for byte can still be a game that started exiting
	# non-zero.
	if [ "$status" != "0" ]; then
		fail "$name: exited with status $status, want 0"
		return
	fi
	if [ "$bytes" -ge "$MAXBYTES" ]; then
		fail "$name: produced at least $MAXBYTES bytes -- runaway output?"
		return
	fi
	if [ "$bytes" -lt "$MINBYTES" ]; then
		fail "$name: only $bytes bytes -- the journey did not really play"
		return
	fi
	if [ "$(grep -v '^[[:space:]]*$' "$got" | tail -1)" != \
	     "May the Great Bird of the Galaxy roost upon your home planet." ]; then
		fail "$name: no farewell -- the journey did not reach the end"
		return
	fi
	if grep -q 'Transmission ends' "$got"; then
		fail "$name: the game ran out of input before the journey was over"
		return
	fi
	if grep -q 'DEBUG:' "$got"; then
		fail "$name: developer chatter would have been recorded as truth"
		return
	fi
	# A carriage return at the end of a line is a leak from a file the
	# game read, and recorded here it would be defended by the fixture
	# forever. The pager's own wipe ends a line too; drop that shape
	# first, exactly as journey.sh does.
	if sed "s/$CR *$CR\$//" "$got" | grep -q "$CR\$"; then
		fail "$name: a line ends with a carriage return"
		return
	fi

	if [ "$UPDATE" = "--update" ]; then
		cp "$got" "$golden/$name.txt"
		printf '  recorded %s (%s bytes)\n' "$name" "$bytes"
		return
	fi
	if [ ! -f "$golden/$name.txt" ]; then
		fail "$name: no recording -- run with --update to make one"
		return
	fi
	if ! cmp -s "$golden/$name.txt" "$got"; then
		fail "$name: the game no longer plays this journey the same way"
		# cmp as well as diff: a difference in trailing whitespace
		# gives two diff lines that look identical.
		cmp "$golden/$name.txt" "$got" 2>&1 | sed 's/^/  /' >&2
		printf '  --- first %d differing lines, recorded vs now ---\n' \
			"$DIFFLINES" >&2
		diff "$golden/$name.txt" "$got" 2>&1 | head -n "$DIFFLINES" |
			cat -v | sed 's/^/  /' >&2
		printf '\n' >&2
	fi
}

if [ "$UPDATE" = "--update" ]; then
	mkdir -p "$golden"
fi

# --- the journeys ----------------------------------------------------
# Chosen to put different parts of the game on the record rather than
# to be realistic: between them they read every report, compute an
# arrival time, cross quadrants, take and give fire, destroy Klingons
# with aimed torpedoes, spend a ship dry and have a starbase fill it up
# again, freeze and thaw, and end a game with the score sheet that goes
# with it.
#
# Still unrecorded, and worth adding when someone finds a seed that
# does it: a device taking damage and being repaired. None of the
# journeys below manages to get the ship hurt, so `damages` says "All
# devices functional." in every one of them.

# Everything that reports without passing time. Pins the wording and
# the numbers of the whole read-only surface.
play reports 'tournament 7
short
novice
pw
srscan
lrscan
status
chart
damages
report
computer 5 5
2

request 1
score
commands
quit
n
n
n
'

# Movement, weapons and the events that come with time passing.
play combat 'tournament 4
long
emeritus
pw
shields up
phasers automatic 200
lrscan
move manual 1 0
srscan
photons 1 5 5
lrscan
move manual 0 1
srscan
photons 1 5 5
damages
score
quit
n
n
n
'

# Torpedoes that connect. Aimed at three Klingons this galaxy really
# has, because a blind shot at the middle of the quadrant misses and
# the damage arithmetic behind a hit would go unrecorded.
play torpedoes 'tournament 23
long
emeritus
pw
srscan
photons 3 2 3 4 4 4 10
srscan
score
quit
n
n
n
'

# Calling for help, docking, and what a starbase gives back.
play docking 'tournament 16
short
novice
pw
phasers automatic 900
photons 1 5 5
status
call
dock
status
damages
chart
quit
n
n
n
'

# A game written out and read back in. The thawed half must describe
# the same galaxy as the half that was saved -- so it reads what the
# journey above wrote, in the same directory, and cannot be run on its
# own.
play freeze 'tournament 9
medium
good
pw
srscan
freeze sav
quit
n
n
n
'
# Thawing takes the randomize() path rather than the tournament seed,
# so this journey is only reproducible while it consumes no random
# numbers -- reading reports does not. Adding a move or a rest here
# would make it fail at random, and the reason would not be obvious.
play thaw 'frozen sav
report
srscan
chart
quit
n
n
n
' freeze

# An ending, with the score sheet that goes with it.
play ending 'tournament 21
short
novice
pw
srscan
destruct
pw
n
n
n
'

if [ "$UPDATE" = "--update" ]; then
	if [ "$fails" -ne 0 ]; then
		printf '\n%d journey(s) were not fit to record.\n' "$fails" >&2
		exit 1
	fi
	printf 'golden: recordings updated\n'
	exit 0
fi

if [ "$fails" -ne 0 ]; then
	printf '\n%d journey(s) changed.\n' "$fails" >&2
	printf 'If that was the point, re-record with:\n' >&2
	printf '    tests/golden.sh <sst> --update\n' >&2
	printf 'and say in the commit what changed and why.\n' >&2
	exit 1
fi

printf 'golden OK\n'
