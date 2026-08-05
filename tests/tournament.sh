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
# The last two are appended rather than placed, and appending is what
# keeps every seed before them on the route it already played. They are
# here for the galaxy checks further down, which the first twelve leave
# half asleep:
#
#   14  opens in quadrant 3 - 5, which holds a starbase. Every other
#       opening quadrant here holds none, so without it the starbases
#       term of the long-range digits is multiplied by nothing and the
#       middle digit of sst.doc:438 is asserted against a constant
#       zero. Changing that term to any other weight passed the whole
#       battery before this seed was added.
#   3   opens in quadrant 1 - 1, the galaxy's corner, where five of the
#       nine scanned cells are outside it. The others are inland or on
#       one edge, so the corner -- both bounds clamped at once -- went
#       unscanned.
#
# A game costs about
# four seconds, nearly all of it the busy-wait in prouts() that types
# the game's dramatic lines out slowly, so fourteen of them take about
# seventy seconds.
SEEDS="7 4 12 6 16 9 19 20 11 21 1 23 14 3"

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
# Both routes scan short-range and then long-range before anything
# else. The two are free of time and energy (sst.doc:352, 461), so no
# turn passes between them: they describe the same quadrant, on a turn
# too early for anything to have damaged the sensors that draw them.
# That pairing is what lets the galaxy checks below read the same
# quadrant twice and compare the two accounts.
#
# Free of time is not free of effect, though. A long-range scan is how
# the star chart learns a neighbourhood, so `chart` later in these
# routes now prints numbers where it used to print dots. Nothing here
# asserts on the chart, and more of it is known than before rather than
# less -- but a chart-shaped surprise further down is this line's
# doing, not the game's.
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
lrscan
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
lrscan
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
any_scanpair=0
any_barrier=0

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

	# --- the galaxy the manual describes ---------------------------
	# `grep -o` appears twice below. It is not in POSIX, and this file
	# is careful about that elsewhere -- but GNU, BSD and busybox all
	# have it, which is every grep the suite runs under, and `head -c`
	# above already leans the same way.
	#
	# Every expected value below comes from sst.doc and none from the
	# game's own source. That distinction is the whole point: a number
	# taken out of setup.c and checked against setup.c agrees with
	# itself whatever either of them says, which is what the note at
	# the top of tests/test_rules.c warns against. Where the manual
	# gives no number, nothing is asserted.

	# sst.doc:173, "in the this galaxy there are from two to five
	# starbases". The briefing states a count and then lists that many
	# quadrants, so the two can disagree with each other as well as
	# with the manual, and both are worth asking about.
	bases=$(sed -n 's/^\([0-9][0-9]*\) starbases in .*/\1/p' "$out" | head -1)
	if [ -z "$bases" ]; then
		fail "seed $seed: the briefing printed no starbase count"
	else
		if [ "$bases" -lt 2 ] || [ "$bases" -gt 5 ]; then
			fail "seed $seed: $bases starbases, where sst.doc:173 promises two to five"
		fi
		listed=$(sed -n 's/^[0-9][0-9]* starbases in //p' "$out" | head -1 |
			grep -oE '[0-9]+ - [0-9]+' | wc -l | tr -d ' ')
		[ "$listed" = "$bases" ] ||
			fail "seed $seed: the briefing counts $bases starbases and then lists $listed"
	fi

	# sst.doc:161, the Super-commander "is reserved for the Good,
	# Expert, and Emeritus games". These are emeritus, so he is
	# promised rather than merely permitted, and the briefing is where
	# the promise is made. The other half of that gate -- that a novice
	# game has none -- is asked once after the battery.
	#
	# What this cannot see is sst.doc:162, "there is just one
	# Super-commander in a game": the briefing says "one" from a bare
	# `if (d.nscrem)`, so it would say it just the same if there were
	# two. Counting them needs a seam into the game state that no
	# journey has, and it is left unchecked rather than pretended at.
	scs=$(grep -c 'one (GULP) Super-Commander' "$out")
	[ "$scs" = 1 ] ||
		fail "seed $seed: an emeritus briefing announced the Super-commander $scs times, where sst.doc:161-162 reserves him for this game and allows just one"

	# sst.doc:123-125, "eight rows of eight quadrants each". Asked of
	# every coordinate the game printed, wherever the journey went,
	# rather than of the starting one alone.
	# The pattern takes a sign, or an underflowed coordinate is not
	# matched at all and the count stays comfortably at zero -- which
	# is the failure this is most interested in.
	strays=$(grep -oE 'Quadrant -?[0-9]+ - -?[0-9]+' "$out" |
		awk '$2 < 1 || $2 > 8 || $4 < 1 || $4 > 8 { c++ } END { print c+0 }')
	[ "$strays" = 0 ] ||
		fail "seed $seed: $strays quadrant coordinates fall outside the 8 by 8 galaxy of sst.doc:123-125"

	# sst.doc:130-131, quadrants are "ten rows of ten sectors each".
	# The short-range scan is that grid drawn out, so it has ten rows
	# under a header numbering ten columns. Read from the first scan
	# of the game, before anything can damage the sensors and mask it.
	# Counting stops at the first line that is not a grid row, which
	# is what confines it to this scan. Without that it counts
	# row-shaped lines out of every later scan too, and a game drawing
	# nine rows a dozen times over still reaches ten -- an assertion
	# that reads like this one and cannot fail. A build whose srscan
	# drew nine rows is what showed it up.
	#
	# There is no guard here for a pager prompt landing mid-grid:
	# pause() writes its prompt with proutn and wipes it with carriage
	# returns, so it would arrive as a prefix of a row rather than as a
	# line of its own, and skipping "that line" would throw away a row
	# and undercount. It cannot happen on turn one anyway -- chew()
	# zeroes the line count and the scan is eleven lines -- and if it
	# ever did, the row count below is what would say so.
	srows=$(awk '/^    1 2 3 4 5 6 7 8 9 10$/ && !s { s=1; next }
		s && /^[ 0-9][0-9]  / { n++; next }
		s { exit }
		END { print n+0 }' "$out")
	[ "$srows" = 10 ] ||
		fail "seed $seed: the first short-range scan draws $srows rows, where sst.doc:130-131 says ten"

	# sst.doc:436-439 gives the long-range scan's digits: thousands a
	# supernova, hundreds Klingons, tens starbases, ones stars. The
	# scan prints the galaxy's own words (lrscan() in reports.c), so
	# this reads the packing itself -- the thing #58 set out to check
	# -- against the manual's description of what it means, rather
	# than against the code that writes it.
	#
	# The short-range scan of the same quadrant is the other account:
	# it draws what is there, and the manual says which of it the
	# long-range scan may count. Klingons and command ships alike
	# (sst.doc:442-444); Romulans not at all (sst.doc:448-449), planets
	# not at all (sst.doc:453-455) -- so an R or a P in the grid must
	# leave the digits alone.
	#
	# How much the two halves are worth differs, and it is worth being
	# straight about it. newqad() fills the quadrant by decoding the
	# same galaxy word that the long-range scan prints, so for
	# Klingons, starbases and stars this weighs an encoding against
	# its own decoding, with the manual supplying the weights -- it
	# would catch a digit that moved or a weight that changed, not a
	# miscount shared by both. The exclusions are the independent
	# half: Romulans and planets come from d.newstuf, by a path the
	# galaxy word knows nothing about, so their staying out of the
	# digits is a genuine second opinion.
	#
	# The Super-commander is counted with the Klingons, and that one is
	# this test's reading rather than the manual's word: sst.doc:159-160
	# introduces him as a "special commander", and sst.doc:442-444 says
	# the scanner does not tell a Klingon from a command ship, which
	# leaves him on the Klingon side of the only distinction the manual
	# draws. Read as it is, not as the manual settling the question.
	# None of the opening quadrants holds one, so the S in the count
	# below is defensive and untested -- what exercises it is a galaxy
	# that opens on him, which no seed here does.
	#
	# A Tholian is the same kind of reading and does get exercised:
	# seeds 20 and 1 open with a T in the grid, and counting it into
	# no digit at all is this test's choice, since the manual never
	# mentions Tholians. It follows the same rule as the rest -- the
	# long-range scan counts what sst.doc:437-439 names, and nothing
	# it does not.
	#
	# Skipped rather than guessed at when either scan is unavailable:
	# damaged short-range sensors mask the grid with '-', and damaged
	# long-range ones print no numbers to read. Neither should happen
	# on the first turn, which is why the battery asks below that this
	# has not quietly stopped running.
	grid=$(awk '/^    1 2 3 4 5 6 7 8 9 10$/ && !s { s=1; next }
		s && /^[ 0-9][0-9]  / { g = g substr($0, 5, 19); next }
		s { exit }
		END { print g }' "$out")
	# Read from the turn that follows the short-range grid, and no
	# further. The two scans are consecutive commands, so the heading
	# belonging with this grid arrives before the next prompt; giving
	# up at the second one is what stops the search from running on
	# and finding a docked scan of somewhere else, hours of game time
	# later, whenever the opening long-range scan did not print. That
	# stale pairing reads as the galaxy's arithmetic being wrong, so
	# it is worth refusing rather than reporting.
	lrsection=$(awk '/^    1 2 3 4 5 6 7 8 9 10$/ && st == 0 { st = 1; next }
		st == 1 && /^[ 0-9][0-9]  / { next }
		st == 1 { st = 2 }
		st == 2 && /COMMAND>/ { if (++prompts > 1) exit; next }
		st == 2 && /[Ll]ong-range scan for Quadrant/ { print; st = 3; next }
		st == 3 && n < 3 { n++; print; if (n == 3) exit }' "$out")
	lrhead=$(printf '%s\n' "$lrsection" | sed -n '1p')
	lrblock=$(printf '%s\n' "$lrsection" | sed -n '2,4p')
	centre=$(printf '%s\n' "$lrblock" | awk 'NR == 2 { print $2 }')
	# Which quadrant each scan is of. The short-range one says so in
	# the status column beside its grid; the long-range one in its
	# heading, read from the token after "Quadrant" rather than from a
	# fixed field, because docked the line reads "Starbase's
	# long-range scan for ..." and every field shifts along.
	srquad=$(awk '/^    1 2 3 4 5 6 7 8 9 10$/ && !s { s=1; next }
		s && /Position/ { print; exit }
		s && !/^[ 0-9][0-9]  / { exit }' "$out" |
		tr ',' ' ' |
		awk '{ for (i = 1; i < NF - 2; i++) if ($i == "Position") print $(i+1) "-" $(i+3) }')
	lrquad=$(printf '%s\n' "$lrhead" | awk '{ for (i = 1; i < NF - 2; i++)
		if ($i == "Quadrant") print $(i+1) "-" $(i+3) }')
	# A masked grid is any '-' in it, and a centre that is not a plain
	# number means the block read was not the one wanted. Both are
	# asked with `case` rather than with tr: a set of one dash is
	# exactly the argument some tr implementations read as an option.
	case "$grid" in
		*-*) masked=1 ;;
		*) masked=0 ;;
	esac
	case "$centre" in
		'' | *[!0-9]*) centre='' ;;
	esac
	# And the two scans must name the same quadrant. The search above
	# is already bounded to this turn, so this is the cheap check that
	# it found the heading belonging to this grid rather than nothing
	# at all -- an opening long-range scan can be missing, its sensors
	# damaged by an attack on the way in, which a start inside the
	# Romulan Neutral Zone can arrange. Belt to that bound's braces,
	# and worth keeping: between them, an unpaired grid is skipped
	# instead of being compared against somebody else's quadrant and
	# reported as the galaxy's arithmetic being wrong.
	if [ -n "$grid" ] && [ -n "$centre" ] && [ "$masked" = 0 ] &&
	   [ -n "$srquad" ] && [ "$srquad" = "$lrquad" ]; then
		# sst.doc:433-434, the scan covers "your quadrant and all
		# adjacent quadrants": three rows of three. Asked first, and
		# the two checks below are held behind it, because both read
		# cells out of that block by position: a block of another
		# shape makes them describe something else, and the digit
		# arithmetic is a confusing way to be told the block was
		# malformed.
		shape=$(printf '%s\n' "$lrblock" |
			awk 'NF != 3 { bad = 1 } END { print NR ":" bad+0 }')
		if [ "$shape" != "3:0" ]; then
			fail "seed $seed: the long-range scan is not the three by three block of sst.doc:433-434 (rows:malformed = $shape)"
		else
			klingons=$(printf '%s' "$grid" | tr -cd 'KCS' | wc -c | tr -d ' ')
			starbases=$(printf '%s' "$grid" | tr -cd 'B' | wc -c | tr -d ' ')
			stars=$(printf '%s' "$grid" | tr -cd '*' | wc -c | tr -d ' ')
			want=$((klingons * 100 + starbases * 10 + stars))
			if [ "$centre" -ne "$want" ]; then
				fail "seed $seed: the long-range scan reads $centre where the quadrant holds K=$klingons B=$starbases stars=$stars -- sst.doc:436-439 makes that $want"
			fi

			# sst.doc:457-459, the -1s "indicate the negative
			# energy barrier at the edge of the galaxy, which you
			# are not permitted to cross". How many there should
			# be follows from where the ship is, without needing
			# to know which way round the block is printed: the
			# scan covers the quadrant and its neighbours, so a
			# row or column at the galaxy's edge loses three of
			# the nine cells, and a corner loses five. This is the
			# only check here that reads the bounds arithmetic in
			# lrscan() rather than the contents of a quadrant.
			qx=${lrquad%-*}
			qy=${lrquad#*-}
			# Each half on its own: asking of the two stuck
			# together lets a number and an empty string through
			# as a number, which is the shape a malformed heading
			# would take.
			case "$qx:$qy" in
				*[!0-9]:* | *:*[!0-9]* | :* | *:) ;;
				*)
					case "$qx" in 1 | 8) ax=2 ;; *) ax=3 ;; esac
					case "$qy" in 1 | 8) ay=2 ;; *) ay=3 ;; esac
					offgalaxy=$((9 - ax * ay))
					# An inland quadrant asks that there
					# are no barrier cells, which is worth
					# asking but is also what this check
					# looks like when it has stopped
					# meaning anything. The battery wants
					# one game that really stood at an
					# edge.
					[ "$offgalaxy" = 0 ] || any_barrier=1
					barriers=$(printf '%s\n' "$lrblock" |
						tr ' ' '\n' | grep -c '^-1$')
					[ "$barriers" = "$offgalaxy" ] ||
						fail "seed $seed: the long-range scan of quadrant $qx - $qy shows $barriers barrier cells where the 8 by 8 galaxy leaves $offgalaxy outside it (sst.doc:457-459)"
					;;
			esac
		fi
		any_scanpair=1
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

# --- the other half of the Super-commander's skill gate ---------------
# sst.doc:161 reserves him for the Good, Expert and Emeritus games.
# The battery above is emeritus and asks that he is announced; this
# asks that a novice game is left alone, which is the half that a
# battery of hard games cannot see. One game, and a short one, because
# only the briefing is being read.
#
# "YOU'LL NEED IT", not the "(GULP) Super-Commander" line the battery
# looks for: that line lives inside the branch setup.c takes for every
# skill but novice, so a novice game cannot print it however many
# Super-commanders it has, and a check for it would pass while seeing
# nothing. The taunt after "Good Luck!" is outside the branch and is
# the one thing a novice game shows for `d.nscrem`. A binary built
# with the skill gate removed is what found that out.
novice="$work/novice.txt"
(cd "$work" && printf 'tournament 7\nshort\nnovice\npw\nquit\nn\nn\nn\n' |
	run "$SST" 2>&1) | head -c "$MAXBYTES" > "$novice"
# "Good Luck!" ends every briefing, long or short, where the wording
# above it does not: a novice game is told about "a deadly Klingon
# invasion force" and never about "Klingons", so the obvious grep for
# the plural passes on a battery game and fails on this one.
if ! grep -q 'Good Luck' "$novice"; then
	fail "novice briefing: the game did not get as far as briefing the player"
elif [ "$(grep -v '^[[:space:]]*$' "$novice" | tail -1)" != \
       "May the Great Bird of the Galaxy roost upon your home planet." ]; then
	# Cheap, and it is what tells a desynchronised run from a game
	# that simply had nothing to report: "Good Luck" is printed on
	# the way past either way.
	fail "novice briefing: the game did not end cleanly"
else
	grep -q "YOU'LL NEED IT" "$novice" || grep -q 'Super-Commander' "$novice" &&
		fail "novice briefing: a Super-commander, where sst.doc:161 reserves him for Good and above"

	# The same promise as the battery's, read off the other briefing.
	# A novice game words it "You will have N supporting starbases."
	# (setup.c takes a different branch for it), so the emeritus check
	# above never sees this wording at all -- and sst.doc:173 makes no
	# distinction between the two.
	nbases=$(sed -n 's/^You will have \([0-9][0-9]*\) supporting starbases\..*/\1/p' \
		"$novice" | head -1)
	if [ -z "$nbases" ]; then
		fail "novice briefing: no starbase count"
	elif [ "$nbases" -lt 2 ] || [ "$nbases" -gt 5 ]; then
		fail "novice briefing: $nbases starbases, where sst.doc:173 promises two to five"
	fi
fi

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
# The long-range digits are checked only where both scans came out
# clean, so this is what stops that check from skipping every game and
# reporting nothing. Every seed manages it today -- the pair is drawn
# on the first two commands, before a shot is fired -- but a galaxy
# that opens under attack could legitimately cost one its sensors, so
# the battery is asked for one rather than for all fourteen.
[ "$any_scanpair" = 1 ] ||
	fail "no game produced a short- and long-range scan of the same quadrant"
# And one of them stood at the galaxy's edge, where the barrier check
# has something to count. Every seed away from an edge asks only that
# no barrier cell appears, which passes just as well when the check has
# quietly stopped working. Three of these seeds open on an edge today
# -- 21, 23, and 3, which is the corner and so the only one that puts
# both bounds to work at once. A reshuffled list that lost them would
# take the check with it, and nothing else would say so.
[ "$any_barrier" = 1 ] ||
	fail "no game scanned from the galaxy's edge, so the barrier check counted nothing"

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'tournament OK (%s seeds%s)\n' "$(echo "$SEEDS" | wc -w | tr -d ' ')" \
	"${BUILD_TYPE:+, $BUILD_TYPE}"
