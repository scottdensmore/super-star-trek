#!/bin/sh
# Checks that a source file's line endings cannot move without someone
# deciding to move them.
#
# Eleven files here are wholly CRLF -- ai.c battle.c events.c finish.c
# moving.c planets.c reports.c setup.c sst.c sst.doc sst.h -- and every
# newer file is LF, so the tree is mixed by vintage rather than by
# accident. A wholesale flip in any of them passes every other check
# this repository has. The compiler does not care: #180 reports .text
# byte-identical in both ci- presets, zero warnings under -Werror, with
# every line of sst.c converted, and ctest, the journeys, the golden
# recordings and analyze all passing. The only thing that showed it was
# a diffstat. (#180's own figures are from its tree, not reproduced
# here. Against sst.c as it stands -- 1191 lines -- a full flip renders
# as 2382 changed lines where a real edit would be a handful.)
#
# That is the shape worth guarding: not a build that breaks, but a diff
# that looks like a rewrite -- or worse, a real change buried in one,
# because the genuine hunks are indistinguishable from the noise once
# the whole file has moved.
#
# Two ways it gets in, and they need different guards.
#
# A clone that normalizes. With no `text` attribute git falls back to
# core.autocrlf, which is true by default in Git for Windows: all eleven
# flip on checkout and flip back on commit. Measured on a clone of
# 13a7fb8 with core.autocrlf=true -- `git add --renormalize .` rewrites
# exactly those eleven, 9,751 insertions and 9,751 deletions, which is
# every line of all eleven and 19,502 lines to read in a diff.
# .gitattributes is what stops that, and rule 1 checks it is still in
# force rather than checking the symptom. The globs include `*.sh`, so
# the guard covers all ten scripts -- itself among them, once it is
# tracked; `git ls-files` cannot see it before that. Without that,
# `*.sh -text` is asserted by nothing: delete the line and everything
# stays green until a CRLF checkout kills this script at
# `set: Illegal option -`, which ctest reports as a broken test rather
# than as an unprotected tree.
#
# An editing tool in text mode. Python's open(p).read() and .write()
# strip CRLF silently, and so does any editor set to normalize on save.
# .gitattributes cannot help here -- it governs git's own conversion on
# checkout and commit, not what another program writes into the file.
# That is how it actually happened, on the branch for #174, and it was
# caught by a human reading a diffstat. Rules 2 and 3 are what turn that
# into a failing test.
#
# Usage: lineendings.sh <source-dir>

# -f as well as -eu: the expansions below are deliberately unquoted so
# they split on newlines, and without -f each field is then matched as a
# pattern against the *current* directory -- which is why this only bites
# when the script is run from a tree holding a matching name, as a person
# does and ctest does not.
#
# Measured, on a fixture holding `weird[1].c` (CRLF, listed) beside a
# decoy `weird1.c` (LF), run from inside it: without -f both the tracked
# path and the CRLF list expand to `weird1.c`, so the loop reads that one
# file twice and reports `weird1.c was CRLF and is now LF` -- twice, about
# a file that never moved, while the file that is CRLF goes unexamined.
# A loud wrong answer rather than a silent one, and either is a guard
# reporting on a file it never opened. With -f the same fixture passes
# with both files correctly classified. The flag costs one character.
set -euf

src="${1:?usage: lineendings.sh <source-dir>}"

# Split on newlines only, everywhere below. The default IFS would break a
# tracked path containing a space into two names that match nothing, and a
# check that silently examines nothing is the failure this script is about.
# It is set once, before anything splits a list, because the CRLF list is
# newline-separated too.
IFS='
'

skip() {
	printf 'SKIP: %s, so line endings went unchecked\n' "$1" >&2
	exit 77
}

command -v git >/dev/null 2>&1 || skip "no git on PATH"
git -C "$src" rev-parse --is-inside-work-tree >/dev/null 2>&1 ||
	skip "$src is not a git work tree"

# The files that are CRLF today. Explicit, and deliberately so: rules 1
# and 2 are self-maintaining, but nothing can *derive* which ending a
# file is supposed to have -- that is a fact about its history. A new
# file is LF unless it is named here, so adding one needs no edit; a
# file legitimately converted needs one line removed, in the commit that
# converts it, which is exactly the review this guards.
# One name per line: the loops below split on newlines only, so a
# space-separated list here would compare whole lines and match nothing.
crlf_files='ai.c
battle.c
events.c
finish.c
moving.c
planets.c
reports.c
setup.c
sst.c
sst.doc
sst.h'

# `sst.doc` rather than `*.doc`, matching .gitattributes: that file
# declines `*.doc` because the next one is likely a Word file wanting
# `binary`, and such a file would fail rule 2 here as "mixed" on its
# own bytes. A text .doc added later goes in both places.
# One pattern per line, unquoted: IFS is newline so they split into four
# words, and `set -f` above stops the shell expanding them before git
# sees them. Quoting them here would make the quotes literal and match
# nothing -- which the reach checks below catch loudly, but only after.
globs='*.c
*.h
sst.doc
*.sh'

is_crlf_file() {
	for known in $crlf_files; do
		[ "$1" = "$known" ] && return 0
	done
	return 1
}

failures=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	failures=$((failures + 1))
}

checked=0
seen=''

for f in $(git -C "$src" ls-files -- $globs); do
	path="$src/$f"
	[ -f "$path" ] || continue
	checked=$((checked + 1))
	seen="$seen
$f"

	# Rule 1: git must not be guessing. `-text` means it converts
	# nothing in either direction, so a clone on any platform gets the
	# bytes that were committed. Checked through check-attr rather than
	# by reading .gitattributes, so any spelling that produces the right
	# attribute passes and any that does not fails.
	attr=$(git -C "$src" check-attr text -- "$f" | sed 's/.*: //')
	if [ "$attr" != "unset" ]; then
		fail "$f has text=$attr; it needs -text, or core.autocrlf will convert it"
	fi

	cr=$(tr -cd '\r' < "$path" | wc -c | tr -d ' ')
	lf=$(tr -cd '\n' < "$path" | wc -c | tr -d ' ')

	# Rule 2: no file may be half-converted. A tool that rewrote part of
	# a file leaves this, and it is the one state no vintage explains.
	if [ "$cr" -ne 0 ] && [ "$cr" -ne "$lf" ]; then
		fail "$f is mixed: $cr CR against $lf LF"
		continue
	fi

	# Rule 3: and the ending has to be the one it has always had.
	if is_crlf_file "$f"; then
		if [ "$cr" -eq 0 ]; then
			fail "$f was CRLF and is now LF -- a whole-file flip, not an edit"
		fi
	elif [ "$cr" -ne 0 ]; then
		fail "$f was LF and is now CRLF -- a whole-file flip, not an edit"
	fi
done

# A guard that silently checked nothing looks exactly like a green one,
# so assert the reach directly: every file the list names must actually
# have been examined above.
#
# This began as `checked -lt 15` and that number had no headroom -- the
# tree holds 15 *.c, 4 *.h, 1 *.doc and 10 *.sh, so losing every glob but
# `*.c` still leaves checked at exactly 15 while sst.h and sst.doc, two of
# the eleven, go unchecked. The guard would have sat silent through its own
# failure mode, and widening the globs later did not save it -- the same
# number still lands on the boundary. Naming the files instead catches a
# lost glob that drops one of the eleven, and also catches a stale entry:
# rename or delete one and it is no longer seen, where a count would
# never notice.
for known in $crlf_files; do
	found=0
	for f in $seen; do
		[ "$f" = "$known" ] && found=1 && break
	done
	if [ "$found" -eq 0 ]; then
		fail "$known is listed as CRLF but was never checked; a glob or the file is gone"
	fi
done

# One file per glob that must have been examined, so dropping a glob is
# caught. Written out rather than derived from $globs: a check that
# iterates the list it is checking narrows with it -- measured, removing
# `*.sh` from $globs then skipped its own assertion and passed at 20
# files, with `*.sh -text` asserted by nothing again. That is the
# regression this file exists to prevent, and it survived the first
# attempt at this check.
#
# The completeness loop above already covers .c, .h and sst.doc through
# crlf_files; they are named again here so each glob has a sentinel of
# its own and a legitimate conversion cannot quietly remove one.
# `tui.c` is here for a different reason from the rest: it is LF and not
# in crlf_files, so it is what keeps rule 3's `was LF and is now CRLF`
# arm guarding something. A counting check stood here instead and was
# dead code -- with `tests/lineendings.sh` in this list, completeness
# passing already implies checked > listed, so it could never fire.
required_seen='sst.c
sst.h
sst.doc
tui.c
tests/lineendings.sh'

for known in $required_seen; do
	found=0
	for f in $seen; do
		[ "$f" = "$known" ] && found=1 && break
	done
	if [ "$found" -eq 0 ]; then
		fail "$known was never examined; the glob covering it was dropped, and everything it covered is now unchecked"
	fi
done

if [ "$failures" -ne 0 ]; then
	printf '\n%s check(s) failed across %s files.\n' "$failures" "$checked" >&2
	printf 'If a conversion was deliberate, change crlf_files in this script\n' >&2
	printf 'in the same commit, so the diff says so.\n' >&2
	exit 1
fi

printf 'line endings: %s files, all as recorded\n' "$checked"
