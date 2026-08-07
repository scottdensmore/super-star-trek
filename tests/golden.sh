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

# Asserts that a recording still contains the lines it exists for.
#
# Everything else here compares a fixture with itself, which cannot
# notice a journey that quietly stopped doing the thing it was written
# to do -- it would simply be re-recorded doing something else, and
# match from then on. The `won` journey is one seed away from being an
# ordinary fight that ends in a quit, so it says out loud what a win
# prints. Checks the fixture, not the run, so that a careless
# --update fails here too.
#
# $1 fixture name, then one or more lines it must contain.
covers() {
	name="$1"
	shift
	f="$golden/$name.txt"
	if [ ! -f "$f" ]; then
		fail "$name: no recording to check for content"
		return
	fi
	missed=0
	for want in "$@"; do
		if ! grep -qF -- "$want" "$f"; then
			fail "$name: the recording no longer contains \"$want\""
			missed=1
		fi
	done
	# Once, not once per line. Worth saying at all because --update
	# has already written the file by the time this runs: the summary
	# below calls it unfit to record, and it is on disk anyway.
	if [ "$missed" -ne 0 ] && [ "$UPDATE" = "--update" ]; then
		printf '  (%s was written anyway -- git checkout %s to undo)\n' \
			"$name.txt" "tests/golden/$name.txt" >&2
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
# again, freeze and thaw, end a game with the score sheet that goes
# with it, and win one.
#
# Still unrecorded, and worth adding when someone finds a seed that
# does it: a device taking damage and being repaired. None of the
# journeys below manages to get the ship hurt, so `damages` says "All
# devices functional." in every one of them.
#
# Still unrecorded on the winning path, all of them dearer than the
# win below: the Commodore Emeritus citation and the plaque behind it,
# which need a win at better than Good; the captured-Klingon transfer
# line; a win that earns no promotion; and the TUI's own end of game,
# since tests/tui.sh never wins one either.

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

# A game that is won. Every other journey here quits or dies, so the
# whole FWON epilogue in finish.c went unrecorded: the Romulan
# surrender, the promotion prose, and the win bonus on the score sheet.
# That was not only a gap in the wording -- #52 and #54 both changed
# arithmetic that runs on no other path, and this suite said nothing
# about either.
#
# Winning means the galaxy's last Klingon dead, which sounds beyond a
# script until you count them: a short novice game has two to four.
# tournament 23 puts all three -- two Klingons and a Commander -- in
# quadrant 5 - 7, which is why the whole game is one move and one
# phaser burst. Change the seed and it stops winning, so the assertion
# below checks the recording still shows a win rather than trusting it.
# The `torpedoes` journey names this seed too, and opens identically --
# same square, same first three starbases -- because those are drawn
# before the answers can matter. Length and skill are typed after the
# seed is set (setup.c:480, 505) and then decide how much the layout
# draws (setup.c:547), so from the Klingon count onward the two part
# company: 3 battle cruisers here against 139 there. A seed names a
# game, not a galaxy.
#
# Two answers at the end, and two is all the game asks for: whether to
# record the score, and whether to play again. The journeys above pass
# a third that is never read. The two pauses on this path eat nothing
# either, which is the part worth knowing: readinput() is fgets, so
# stdio has already drawn the whole pipe into its buffer, and the raw
# read() behind getch() finds nothing left. Give the pause a keystroke
# of its own here and the score question eats it instead, and answers
# slide by one. That holds while a script fits in one stdio buffer,
# which every journey here does several times over; a much longer one
# would leave bytes in the pipe for the pause to eat, and would need
# its answers counted again.
#
# Two things about the recording itself. A supernova announcement
# arrives mid-journey and pauses, so this is the only fixture with a
# real screen-clear escape in it -- read it with `cat -v` or it wipes
# what came before off your terminal. And its score sheet is the only
# one anywhere with a Romulan surrender line, which is how #62 was
# found: the plural format string was a space short, so that line sat a
# column left of every other. Fixed, and this recording is the evidence;
# tests/score.sh now holds every one of those formats to column 47.
play won 'tournament 23
short
novice
pw
srscan
move automatic 5 7 5 5
srscan
shields up
phasers automatic 1500
n
n
'

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

# The epilogue the `won` journey exists to put on the record. The
# promotion line is the one #54 changed and this suite could not see;
# the total is what would notice #52-style arithmetic drift without
# somebody reading the diff by eye.
covers won \
	'Romulan ships surrender to Starfleet Command.' \
	'You have smashed the Klingon invasion fleet and saved' \
	'promotes you one step in rank from "Novice" to "Fair".' \
	'2 Romulan ships surrendered' \
	'Bonus for winning Novice game' \
	'TOTAL SCORE                                 797'

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
