#!/bin/sh
# Covers the game's handling of sst.doc, which has CRLF line endings.
#
# The manual is read from that file and printed like any other game
# text, so a trailing carriage return travels with it. In the plain
# display that is invisible; under curses the CR sends the cursor back
# to column 0 and the newline then erases the line, which once left
# `help` printing nothing at all. The same applies to input: a script
# written on Windows arrives with carriage returns attached to every
# command.
#
# Every case here ends the game by running out of input, so the pager
# never steals a character from a command still to come. Each plays in
# a directory of its own holding the sst.doc that case needs, since
# `help` reads it from the current directory.
#
# Usage: help.sh /path/to/sst

set -u

SST="${1:-./sst}"
# The cases below run from directories of their own.
case "$SST" in
	/*) ;;
	*) SST="$(pwd)/$SST" ;;
esac
srcdir=$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)

CR=$(printf '\r')	# BSD sed reads a regex \r as a literal r
MAXBYTES=262144
DUMPBYTES=2048

work=$(mktemp -d "${TMPDIR:-/tmp}/sst-help.XXXXXX")
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
		timeout 20 "$@"
	else
		"$@"
	fi
}

out="$work/out.txt"
rc="$work/rc.txt"

# The real manual, and one cut off before its end-of-topic marker.
real="$work/real"
trunc="$work/trunc"
mkdir -p "$real" "$trunc"
if [ ! -f "$srcdir/sst.doc" ]; then
	echo "FAIL: cannot find sst.doc next to $srcdir" >&2
	exit 1
fi
cp "$srcdir/sst.doc" "$real/sst.doc"
printf 'preamble\r\n  Mnemonic:  MOVE\r\nsome help text\r\n' > "$trunc/sst.doc"

# $1 description, $2 input, $3 directory to play in.
play() {
	rm -f "$out" "$rc"
	(
		cd "$3" || exit 1
		printf '%s' "$2" | run "$SST" 2>&1
		echo $? > "$rc"
	) | head -c "$MAXBYTES" > "$out"
	status=$(cat "$rc" 2>/dev/null || echo "no-status")
	bytes=$(wc -c < "$out" | tr -d ' ')
	if [ "$status" != "0" ]; then
		fail "$1: exited with status $status, want 0"
	fi
	if [ "$bytes" -ge "$MAXBYTES" ]; then
		fail "$1: produced at least $MAXBYTES bytes -- looping?"
	fi
}

want() {
	grep -q "$2" "$out" || fail "$1 (expected output to contain: $2)"
}

dump_if_failed() {
	[ "$fails" -eq "$1" ] && return
	printf '  --- last %d bytes ---\n' "$DUMPBYTES" >&2
	tail -c "$DUMPBYTES" "$out" >&2
	printf '\n' >&2
}

# --- the manual actually prints ------------------------------------
before=$fails
play "help" 'regular
short
novice
xyz
help move
' "$real"
want "help printed no documentation" "Full command:"
want "help printed no documentation body" "warp drive"
# The bug this guards: with the carriage returns left on, every one of
# these lines erases itself the moment it is drawn under curses. The
# pager's own "\r<spaces>\r" wipe ends a line too, so drop that shape
# before looking.
if sed "s/$CR *$CR\$//" "$out" | grep -q "$CR\$"; then
	fail "help output still carries sst.doc's carriage returns"
fi
dump_if_failed "$before"

# --- a script written on Windows is playable ------------------------
before=$fails
play "crlf input" "regular$CR
short$CR
novice$CR
xyz$CR
srscan$CR
" "$real"
want "CRLF script: no short-range scan" "1 2 3 4 5 6 7 8 9 10"
if grep -q 'What is' "$out"; then
	fail "CRLF script: the game rejected a command it should have understood"
fi
dump_if_failed "$before"

# --- a truncated manual stops instead of repeating itself -----------
before=$fails
play "truncated manual" 'regular
short
novice
xyz
help move
' "$trunc"
want "truncated manual: printed nothing" "some help text"
want "truncated manual: said nothing about the missing rest" "rest of that entry is missing"
dump_if_failed "$before"

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	exit 1
fi

printf 'help OK\n'
