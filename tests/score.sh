#!/bin/sh
# Checks that the score sheet's columns line up.
#
# score() in finish.c prints one line per thing the player did, each a
# format string ending in %5d, and every one of them is meant to put its
# number in the same column. Nothing enforced that. The plural Romulan
# line spent years a space short, so a winning player's surrendered-ship
# count sat one column left of everything under it (#62) -- on the score
# sheet a player is most likely to read, since surrendered Romulans only
# count on a win.
#
# It went unnoticed because no test could see it. The recordings in
# tests/golden are the only place a score sheet is written down, and
# until a game was won there was none with a Romulan line in it at all.
#
# So this reads the format strings themselves rather than any output,
# which is the only way to reach most of them. A journey prints the line
# its game happens to earn: golden records two singular forms between its
# eight fixtures, and the other ten appear nowhere. "1 Romulan ship
# surrendered" -- the singular half of the line #62 was about -- needs
# exactly one Romulan alive at the end, where a novice game starts with
# two or three, so no journey reaches it without hunting Romulans first.
# Twelve lines have two forms and three have one, which is the 27 counted
# below, and #62 asked for both forms.
#
# What this cannot see, and golden does: the killed penalty, a bare
# literal with no conversion in it to measure. tests/golden/ending.txt
# records it -- and only that fixture can, since the line prints when the
# ship is destroyed and the won journey ends alive.
#
# The winning bonus is checked below, because golden reaches only one
# fifth of it: no journey wins above Novice, so four of the five skill
# labels appear in no recording at all.
#
# And none of this sees the compiled game. If proutf ever stopped
# honouring these widths the recordings would notice and this would not.
#
# Usage: score.sh /path/to/repo/root

set -u

ROOT="${1:-.}"
FINISH="$ROOT/finish.c"

# Where the number ends on every line of the sheet. Not a number the
# code declares anywhere -- it is the shape all but one of the lines
# already had, and the one that the recordings show.
WIDTH=47

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

if [ ! -f "$FINISH" ]; then
	echo "FAIL: $FINISH not found" >&2
	exit 1
fi

# Every string literal that ends in a %5d field, with the conversions
# replaced by the width they print: %6d and %6.2f are six columns, %5d
# is five. A ternary puts two literals on one line, so the line is split
# on quotes and every other field taken.
# A literal with no letter in it is a fragment of a line built from
# several proutn calls -- the winning bonus ends in one -- and has no
# column of its own to check. Those are golden's to watch, not this
# script's.
report=$(awk -v want="$WIDTH" '
	/^void score\(/ { inscore = 1 }
	inscore && /^}/ { inscore = 0 }
	inscore && /%5d\\n/ {
		n = split($0, part, "\"")
		for (i = 2; i <= n; i += 2) {
			lit = part[i]
			if (lit !~ /%5d\\n/) continue
			s = lit
			gsub(/\\n/, "", s)
			gsub(/%6\.2f/, "######", s)
			gsub(/%6d/, "######", s)
			gsub(/%5d/, "#####", s)
			# Asked once nothing is left but the label and its
			# padding. Before the substitutions, the n of \n and
			# the d of %5d both read as a label.
			if (s !~ /[A-Za-z]/) continue
			if (length(s) != want)
				printf "bad:%d:%d:%s\n", FNR, length(s), lit
			seen++
		}
	}
	END { printf "count:%d\n", seen+0 }
' "$FINISH")

count=$(printf '%s\n' "$report" | sed -n 's/^count:\([0-9]*\)$/\1/p')
printf '%s\n' "$report" | sed -n 's/^bad://p' | while IFS=: read -r line width lit; do
	printf 'FAIL: finish.c:%s puts its number at column %s, not %s -- "%s"\n' \
		"$line" "$width" "$WIDTH" "$lit" >&2
done
# That loop is a subshell and cannot raise the count, so it is counted
# here instead.
bad=$(printf '%s\n' "$report" | grep -c '^bad:' || true)
fails=$((fails + bad))

# The winning bonus is the one line built rather than printed: a prefix,
# one of five skill labels, and a tail carrying the number. It lands in
# the same column only if the three add up, and only the Novice label is
# ever recorded -- no journey wins at Fair or above -- so the other four
# are checked here or nowhere. A player who wins at Emeritus is the one
# most likely to study the sheet.
bonus=$(awk -v want="$WIDTH" '
	/proutn\("Bonus for winning /  { n = split($0, p, "\""); prefix = length(p[2]) }
	/proutn\("[A-Z][a-z]* game *"\)/ {
		n = split($0, p, "\"")
		for (i = 2; i <= n; i += 2)
			if (p[i] ~ /game/) { labels[++nl] = p[i] }
	}
	/proutf\(" *%5d\\n"/ {
		n = split($0, p, "\"")
		s = p[2]
		gsub(/\\n/, "", s)
		gsub(/%5d/, "#####", s)
		tail = length(s)
	}
	END {
		printf "parts:%d:%d:%d\n", prefix+0, nl+0, tail+0
		for (i = 1; i <= nl; i++)
			if (prefix + length(labels[i]) + tail != want)
				printf "bonus:%s:%d\n", labels[i], \
					prefix + length(labels[i]) + tail
	}
' "$FINISH")

parts=$(printf '%s\n' "$bonus" | sed -n 's/^parts://p')
prefixlen=$(printf '%s' "$parts" | cut -d: -f1)
nlabels=$(printf '%s' "$parts" | cut -d: -f2)
taillen=$(printf '%s' "$parts" | cut -d: -f3)
if [ "$prefixlen" = 0 ] || [ "$taillen" = 0 ] || [ "$nlabels" != 5 ]; then
	fail "the winning bonus line did not parse (prefix $prefixlen, $nlabels labels, tail $taillen) -- has it changed shape?"
fi
printf '%s\n' "$bonus" | sed -n 's/^bonus://p' | while IFS=: read -r label got; do
	printf 'FAIL: the winning bonus reads "%s" at column %s, not %s\n' \
		"$label" "$got" "$WIDTH" >&2
done
badbonus=$(printf '%s\n' "$bonus" | grep -c '^bonus:' || true)
fails=$((fails + badbonus))

# A count guards the guard: if the shape of these lines ever changes so
# that the pattern stops matching, every line would "align" by not being
# looked at, and this would pass while checking nothing.
#
# Exact, not a floor. The number is deterministic -- this reads source
# text, so the feature flags do not change it -- and a floor with slack
# in it lets that many checks fall out silently. A floor of 24 was tried
# first and a reviewer walked one straight through it: wrapping a
# ternary across two source lines, which is what tidying these 120-column
# lines would do, dropped a format out of the scan and still printed OK.
# Adding a score line costs one number here, which is the right price.
EXPECTLINES=27
if [ "${count:-0}" -ne "$EXPECTLINES" ]; then
	fail "matched ${count:-0} score lines, expected exactly $EXPECTLINES -- has score() changed shape?"
fi

if [ "$fails" -ne 0 ]; then
	printf '\n%d score-sheet column problem(s).\n' "$fails" >&2
	exit 1
fi

printf 'score OK (%s lines, column %s)\n' "$count" "$WIDTH"
