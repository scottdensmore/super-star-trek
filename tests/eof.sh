#!/bin/sh
# The input stream running out has to end the session.
#
# Without this the game never learns that no answer is coming: it
# re-prompts, reads nothing, prompts again, and floods stdout with
# gigabytes in seconds. Anyone scripting the game or driving it from a
# harness hits that instead of a clean exit, and it is the reason the
# journey test needs a byte ceiling at all.
#
# Usage: eof.sh /path/to/sst

set -u

SST="${1:-./sst}"
# Generous next to a real game's few KB, tiny next to a runaway loop,
# which reaches this in well under a second.
MAXBYTES=262144
DUMPBYTES=2048	# enough of a failing capture to see what went wrong

work=$(mktemp -d "${TMPDIR:-/tmp}/sst-eof.XXXXXX")
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
		timeout 15 "$@"
	else
		"$@"
	fi
}

# $1 description, $2 input, rest are the game's arguments.
check_eof() {
	what="$1"
	input="$2"
	shift 2
	before=$fails

	out="$work/out.txt"
	rc="$work/rc.txt"
	# Fresh each time: a case whose subshell dies before recording its
	# status would otherwise inherit the previous case's success.
	rm -f "$rc" "$out"
	(
		printf '%s' "$input" | run "$SST" "$@" 2>&1
		echo $? > "$rc"
	) | head -c "$MAXBYTES" > "$out"

	status=$(cat "$rc" 2>/dev/null || echo "no-status")
	bytes=$(wc -c < "$out" | tr -d ' ')

	if [ "$status" != "0" ]; then
		fail "$what: exited with status $status, want 0 (124 means it hung)"
	fi
	if [ "$bytes" -ge "$MAXBYTES" ]; then
		fail "$what: produced at least $MAXBYTES bytes -- still looping on empty input?"
	fi
	if ! grep -q "Great Bird of the Galaxy" "$out"; then
		fail "$what: did not sign off; the session should end the way quitting does"
	fi
	# journey.sh detects a script that stops short by looking for this
	# marker's absence, which only means anything while something
	# checks it still gets printed.
	if ! grep -q "Transmission ends" "$out"; then
		fail "$what: no end-of-input marker; journey.sh's guard depends on it"
	fi

	if [ "$fails" -ne "$before" ]; then
		printf '  --- last %d bytes of "%s" ---\n' "$DUMPBYTES" "$what" >&2
		tail -c "$DUMPBYTES" "$out" >&2
		printf '\n' >&2
	fi
}

# Input stops at each of the points the game waits for an answer.
check_eof "eof at the first prompt" ''
check_eof "eof during setup" 'regular
'
check_eof "eof at the command prompt" 'regular
short
novice
xyz
'
check_eof "eof after a command" 'regular
short
novice
xyz
srscan
'
# Asking to play again is its own prompt, read by a different path.
check_eof "eof at the play-again prompt" 'regular
short
novice
xyz
quit
'
# Asking for -t without a terminal falls back to the plain display, so
# this covers the fallback path rather than curses input. Ctrl-D inside
# a real full-screen session needs a pty to test -- see issue #13.
check_eof "eof at the command prompt, -t falls back" 'regular
short
novice
xyz
' -t

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'eof OK\n'
