#!/bin/sh
# Plays real games: movement, combat, docking, and the clock running.
#
# tests/journey.sh deliberately uses only commands that pass no game
# time, because anything that advances the clock can schedule an event,
# and an event pauses for a keystroke. This one does the opposite, and
# the first thing it asserts is that the script survives those pauses:
# a desync shows up as UNRECOGNIZED COMMAND, as "Beg your pardon", or
# as the game running out of input before the journey is over.
#
# It survives them for a reason worth knowing, because it is fragile.
# getch() reads a byte with read(2) rather than through stdio, so it
# would take one from the script -- but the game reaches a pause only
# after fgets() has pulled the whole script into stdin's buffer, and
# read(2) then finds nothing left. That holds only while a journey fits
# in one buffer, which is what the check below is for.
#
# Every game is a `tournament <n>`, which seeds the generator with the
# number, so a failure here is reproducible rather than a flake -- and
# reproducible everywhere, since the generator is the game's own rather
# than the C library's. What is still not certain is what any one
# galaxy contains, so the two kinds of assertion below are kept apart:
#
#   per journey   things true of any game whatever the galaxy holds:
#                 it ends cleanly, it stays in sync, it prints no
#                 developer chatter, it leaks no terminal escapes.
#
#   across the battery
#                 things that need a particular galaxy: that some game
#                 docked, some fought, some destroyed a Klingon, some
#                 filled a screen. Any one seed may legitimately do none
#                 of these -- calling for help can get the ship
#                 dismantled in transit, which ends that game early and
#                 correctly -- so they are asked of the run as a whole.
#
# Usage: tournament.sh /path/to/sst [build-type]

set -u

if [ $# -lt 1 ]; then
	echo "usage: tournament.sh <sst> [build-type]" >&2
	exit 2
fi
SST="$1"
BUILD_TYPE="${2:-}"
case "$SST" in
	/*) ;;
	*) SST="$(pwd)/$SST" ;;
esac
if [ ! -x "$SST" ]; then
	echo "FAIL: $SST is not executable" >&2
	exit 1
fi

# Enough seeds that the galaxy-dependent assertions below have room to
# land on any platform, few enough to stay quick.
#
# Position in this list picks the route -- they alternate -- so the list
# cannot be sorted or trimmed from the middle without changing which
# game each seed plays. The seeds themselves are chosen, not arbitrary:
# between them they dock, cross quadrants, exchange fire, fire a
# torpedo, lose warp drive and fill a screen, several times over each.
# Since the generator is the game's own now rather than the C library's,
# a seed picked here behaves the same everywhere -- which is what makes
# choosing them worth the trouble.
#
# A game costs about
# four seconds, nearly all of it the busy-wait in prouts() that types
# the game's dramatic lines out slowly, so twelve of them take about a
# minute.
SEEDS="7 4 12 6 16 9 19 20 11 21 1 23"

MAXBYTES=262144
DUMPBYTES=3000
CR=$(printf '\r')	# BSD sed reads a regex \r as a literal r
ESC=$(printf '\033')

work=$(mktemp -d "${TMPDIR:-/tmp}/sst-tournament.XXXXXX")
if [ -z "${work:-}" ] || [ ! -d "$work" ]; then
	echo "FAIL: could not create a temporary directory" >&2
	exit 1
fi
trap 'rm -rf "$work"' EXIT
trap 'rm -rf "$work"; exit 130' INT
trap 'rm -rf "$work"; exit 143' TERM
trap 'rm -rf "$work"; exit 129' HUP
trap 'rm -rf "$work"; exit 141' PIPE

out="$work/out.txt"
rc="$work/rc.txt"

run() {
	if command -v timeout >/dev/null 2>&1; then
		timeout 60 "$@"
	else
		"$@"
	fi
}

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

# Two journeys, because docking and fighting pull against each other.
# Calling for help is the only way to reach a starbase without knowing
# where one is, and it is a gamble: roughly one game in four, the
# starbase fails to rematerialise the ship and that game ends there and
# then. Called early the radio is certainly working and most games
# dock, but the quarter that die never reach a fight; called late most
# games have fought first, and fewer dock. So half the seeds do one and
# half the other.
#
# Every command carries its arguments on its own line -- `shields`
# alone asks a question, and a question the script cannot answer eats
# the rest of it.
#
# Two argument forms are handled carefully for the same reason.
# Phasers are fired once, as early as the route can manage, and for a
# small amount: ask for more energy than the ship can spare and it
# re-prompts "Units to fire=" until it gets an affordable number, which
# would eat the rest of the script. The window is narrow rather than
# closed -- below 200 units the command declines cleanly, above 400 the
# ask is affordable, and only in between does it re-prompt -- and
# raising shields costs an attack round first, so "early" is not the
# same as "untouched". The docking route no longer fires at all, which
# is where the risk was worst.
#
# Movement is manual rather than automatic because automatic needs the
# computer, and a damaged one turns those four numbers into a prompt
# for two others. One of the jumps is deliberately too short to take a
# single step, which is a path of its own: the loop that walks the ship
# across the quadrant never runs, and the enemy-distance sums after it
# used to read whatever the stack held.
#
# Both end in more `n`s than the two the quit path needs. A game that
# ends early -- the ship destroyed in a fight, or lost calling for help
# -- goes straight to the same two questions, and answers them out of
# whatever route lines are left; the spares mean it still finds an
# answer rather than running out of input. While the ship is alive they
# are never read, because the game exits on the second one.
ROUTE_DOCK='srscan
call
dock
status
shields up
lrscan
move manual 1 0
srscan
lrscan
move manual 0 1
srscan
chart
damages
score
quit
n
n
n
n
n
n
'

ROUTE_FIGHT='srscan
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
lrscan
move manual 0.04 0
srscan
move manual -1 0
srscan
photons 1 5 5
lrscan
move manual 0 -1
srscan
photons 1 5 5
call
dock
status
chart
damages
score
quit
n
n
n
n
n
n
'

# The routes have to stay inside stdin's stdio buffer; see the top of
# this file for why. A route that outgrew it would fail as a desync and
# blame the pager.
for r in "$ROUTE_DOCK" "$ROUTE_FIGHT"; do
	n=$(printf 'tournament 99\nlong\nemeritus\npw\n%s' "$r" | wc -c | tr -d ' ')
	if [ "$n" -ge 4096 ]; then
		fail "a journey is $n bytes; past stdin's buffer a pause starts eating it"
	fi
done

# Set by the battery, asked about once at the end.
any_docked=0
any_moved=0
any_fought=0
any_fired=0
any_impulse_hint=0
any_paged=0

saw() { grep -qF "$1" "$out"; }

alternate=0
for seed in $SEEDS; do
	rm -f "$out" "$rc"
	if [ "$alternate" = 0 ]; then
		route="$ROUTE_DOCK"; alternate=1
	else
		route="$ROUTE_FIGHT"; alternate=0
	fi
	# The game writes saved games to the directory it plays in.
	(
		cd "$work" || exit 1
		printf 'tournament %s\nlong\nemeritus\npw\n%s' "$seed" "$route" \
			| run "$SST" 2>&1
		echo $? > "$rc"
	) | head -c "$MAXBYTES" > "$out"

	status=$(cat "$rc" 2>/dev/null || echo "no-status")
	bytes=$(wc -c < "$out" | tr -d ' ')
	before=$fails

	[ "$status" = "0" ] ||
		fail "seed $seed: exited with status $status, want 0"
	[ "$bytes" -lt "$MAXBYTES" ] ||
		fail "seed $seed: produced at least $MAXBYTES bytes -- runaway output?"

	# The whole point. A pause calls getch(), and a getch() that took a
	# byte of the script would leave every command after it shifted by
	# one -- which surfaces as an unknown command, or as the game
	# asking a question nobody is left to answer.
	# Both halves matter: an unknown command name says a whole line
	# went astray, "Beg your pardon" says a command got an argument
	# meant for something else. A prompt that ate a line shows up as
	# the second, which is the quieter and likelier of the two.
	if saw "UNRECOGNIZED COMMAND"; then
		fail "seed $seed: a command was not understood -- the script has desynchronised"
	fi
	pardons=$(grep -c 'Beg your pardon' "$out")
	if [ "$pardons" -gt 0 ]; then
		# Two states derail a script through no fault of the pager:
		# with the computer out, getcd() discards the rest of the
		# move command's line before asking for displacements, and
		# with the sensors out, automatic phaser fire becomes a
		# prompt per enemy. Either way the next line of the script
		# answers a question it was not written for. A script cannot
		# see it coming and cannot answer it; what matters here is
		# that nothing *else* knocks the script out of step, so the
		# known causes are named rather than assumed. It happens in
		# about one game in eighteen.
		# One pardon per named cause, no more: a game that has one of
		# them is not thereby excused every other way of losing step.
		causes=$(grep -cE 'Computer damaged; manual movement only|Manual-fire-must-be-used|Battle computer damaged, manual file only\.' "$out")
		if [ "$pardons" -gt "$causes" ]; then
			fail "seed $seed: a command got the wrong argument -- the script has desynchronised"
		fi
	fi
	if saw "[Transmission ends.]"; then
		fail "seed $seed: the game ran out of input before the journey was over"
	fi
	# Last, not merely present: anything printed after the sign-off is
	# output that escaped the end of the game.
	if [ "$(grep -v '^[[:space:]]*$' "$out" | tail -1)" != \
	     "May the Great Bird of the Galaxy roost upon your home planet." ]; then
		fail "seed $seed: the farewell is not the last thing printed"
	fi

	# Every ending prints a score, whether the journey finished or the
	# ship did not. Without this the checks above are all absences,
	# and a game that died on its second command would satisfy them
	# having done nothing.
	saw "TOTAL SCORE" ||
		fail "seed $seed: no score -- the game did not get far enough to end properly"

	# Developer diagnostics must not print for a player who did not ask.
	# Unlike the plain journey, this one fights, so the combat trace is
	# genuinely reachable here.
	if grep -q 'DEBUG:' "$out"; then
		fail "seed $seed: developer chatter printed for a player who did not ask"
	fi
	if grep -qE 'Hit [0-9][^ ]* energy|Prob = [0-9]+ \(' "$out"; then
		fail "seed $seed: combat diagnostics printed for a player who did not ask"
	fi

	# Refusing to warp says what can still be done. A crippled ship is
	# recoverable -- impulse moves it, a starbase repairs it -- and the
	# refusal on its own read as "you cannot move". Checked wherever it
	# happens; that it happens at all is asked of the battery below,
	# since not every galaxy costs the ship its warp drive.
	if grep -q 'The warp engines are damaged, Sir' "$out" &&
	   ! grep -q 'impulse power' "$out"; then
		fail "seed $seed: warp was refused without mentioning impulse"
	fi
	grep -q 'impulse power' "$out" && any_impulse_hint=1

	# The Super-commander's position is reported at most once per turn.
	# It used to be announced twice, byte for byte, when one warp jump
	# scheduled his move more than once -- which reads as the game
	# stuttering rather than as two reports.
	#
	# Back to back, which is two lines apart: the report and the line
	# introducing it, nothing else between. The same quadrant reported
	# again a turn later is six lines apart at the closest -- a second
	# report, not a stutter -- so two is the whole of the window.
	#
	# Be clear about what this is worth. The battery draws four of
	# these reports, one of them a game with two, so the awk has real
	# lines to compare -- but none of them is a duplicate, so nothing
	# currently makes it fail. Seed 4 did, against a build with the
	# fix reverted, and stopped when the Super-commander's pursuit was
	# corrected. Not because his move is scheduled twice any less
	# often: that is a fixed 0.2777-stardate period and seed 4 still
	# schedules it twice in one pass. It is that the axis bug used to
	# leave him sitting still -- with no x component his own quadrant
	# was the destination, checkdest() accepted it, and the two
	# reports in the pass named the same place. Moving every turn,
	# they name different ones. 190 seeds across both routes produced
	# no duplicate afterwards. The guard in scom() is live code with
	# no other coverage, so the check stays; a seed that exercises it
	# again would be worth swapping in.
	if awk '/Super-commander is in/ {
			if ($0 == prev && NR - prevline <= 2) bad = 1
			prev = $0; prevline = NR
		}
		END { exit !bad }' "$out"; then
		fail "seed $seed: the same intelligence report printed twice running"
	fi

	# Plain mode must stay plain. One escape sequence is the game's
	# own: pause() clears the screen for an announcement, and has
	# always done it by hoping for an ANSI terminal. Anything else is
	# curses output leaking out of the TUI, which is what to catch --
	# so drop the known one first rather than give up the check.
	if sed "s/$ESC\[2J$ESC\[0;0H//g" "$out" | tr -cd '\033' | grep -q .; then
		fail "seed $seed: escape sequences other than the pager's leaked into plain-mode output"
	fi
	if sed "s/$CR *$CR\$//" "$out" | grep -q "$CR\$"; then
		fail "seed $seed: a line ends with a carriage return"
	fi

	saw "Condition     DOCKED" && any_docked=1
	# "Course laid in" is printed before the ship moves, and it can
	# then be stopped inside the quadrant. Entering one is the only
	# line that means a boundary was crossed.
	saw "Entering Quadrant" && any_moved=1
	# Either being shot at or shooting something counts: which of the
	# two a given galaxy produces is its own business.
	saw "unit hit from" && any_fought=1
	# From the score sheet, which every quit prints. The bare
	# "*** ... destroyed" line also covers a starbase lost to the
	# Super-commander and the Enterprise's own end, neither of which
	# means the player fired anything.
	grep -qE 'Klingon (ship|Commander ship|ships|Commander ships) destroyed|Super-Commander ship destroyed' "$out" &&
		any_fought=1
	saw "Torpedo track" && any_fired=1
	# Not "filled a screen" -- a death pause counts too. What it
	# proves is that a pause happened and the script survived it,
	# which is this file's whole premise. Both kinds count: the pager
	# filling the screen, and the announcement pause, which also
	# clears it. Only the terser wording of each appears here, since
	# these games are played at emeritus and pause() words its prompts
	# for skill > fair.
	grep -qE '\[CONTINUE\?\]|\[ANNOUNCEMENT ARRIVING' "$out" && any_paged=1

	if [ "$fails" -ne "$before" ]; then
		printf '  --- seed %s, last %d bytes ---\n' "$seed" "$DUMPBYTES" >&2
		tail -c "$DUMPBYTES" "$out" >&2
		printf '\n' >&2
	fi
done

# --- a damaged save file is refused, not played ----------------------
# Every read from a saved game used to be unchecked, so a file that
# stopped short was loaded into the game state and played anyway --
# with, in the worst shape, a password that had never been read. It now
# says so and asks again, which is how a file that was not there at all
# already behaved.
save="$work/damaged"
mkdir -p "$save"
(cd "$save" && printf 'tournament 7\nshort\nnovice\npw\nfreeze sav\nquit\nn\nn\n' |
	run "$SST" >/dev/null 2>&1)
if [ ! -s "$save/sav.trk" ]; then
	fail "damaged save: could not write a save file to damage"
else
	# Everything but the password: the shape a freeze interrupted
	# part-way through leaves behind, and the one that used to be
	# played without complaint.
	whole=$(wc -c < "$save/sav.trk" | tr -d ' ')
	head -c $((whole - 8)) "$save/sav.trk" > "$save/short.trk"
	(cd "$save" && printf 'frozen short\nregular\nshort\nnovice\npw\nquit\nn\nn\nn\n' |
		run "$SST" 2>&1) | head -c "$MAXBYTES" > "$out"
	grep -q "Can't read game file" "$out" ||
		fail "damaged save: a file that stops short was accepted"
	# And nothing of it is left behind: the length and skill questions
	# come back, so the game the player goes on to ask for is the one
	# they get.
	grep -q 'Short, Medium, or Long' "$out" ||
		fail "damaged save: the refused file's settings were kept"
	grep -q 'Novice, Fair, Good, Expert' "$out" ||
		fail "damaged save: the refused file's skill was kept"
	grep -q 'COMMAND>' "$out" ||
		fail "damaged save: the game did not carry on and start another"
fi

# Asked of the battery, not of any one game: any single galaxy may
# contain no base within reach or no enemy worth shooting, but that
# none of a dozen did would mean the journey stopped exercising what it
# is here for.
[ "$any_docked" = 1 ] || fail "no game docked at a starbase"
[ "$any_moved" = 1 ] || fail "no game moved between quadrants"
[ "$any_fought" = 1 ] || fail "no game exchanged fire with an enemy"
[ "$any_fired" = 1 ] || fail "no game fired a torpedo"
[ "$any_impulse_hint" = 1 ] || fail "no game lost warp drive and was told about impulse"
[ "$any_paged" = 1 ] || fail "no game paused for a keystroke"

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'tournament OK (%s seeds%s)\n' "$(echo "$SEEDS" | wc -w | tr -d ' ')" \
	"${BUILD_TYPE:+, $BUILD_TYPE}"
