#!/bin/sh
# Checks that AGENTS.md stays the single source of truth for project
# instructions, and that CLAUDE.md stays a pointer to it.
#
# Rules drift into CLAUDE.md easily and silently: Claude Code's `#`
# shortcut appends there, `/init` rewrites the file wholesale, and
# tools that target CLAUDE.md by name edit it directly. None of them
# know AGENTS.md exists. Without a check, the result is two sources of
# truth that slowly disagree.
#
# The AGENTS.md checks matter just as much: CLAUDE.md pulls it in with
# an `@AGENTS.md` import, and if that file is renamed, emptied, or
# deleted the import resolves to nothing -- no error, no warning, just
# an agent session with no rules at all.
#
# Usage: instructions.sh /path/to/repo/root

set -u

ROOT="${1:-.}"
while [ "${ROOT%/}" != "$ROOT" ] && [ "$ROOT" != "/" ]; do
	ROOT="${ROOT%/}"
done
CLAUDE="$ROOT/CLAUDE.md"
AGENTS="$ROOT/AGENTS.md"

# The pointer needs a title, a sentence or two, and the import. Kept
# tight on purpose: headroom here is room for a rule to hide by being
# folded into an existing line rather than added as a new one, which
# the line budget below cannot see. A very short phrase can still fit;
# that one is left to review.
MAXBYTES=220
# CLAUDE.md is meant to be frozen, so this is the exact number of
# non-blank lines it has today, not a generous budget: a rule slipped in
# as plain prose adds a line and nothing else would. Changing the
# pointer's wording means changing this number on purpose. The prose is
# wrapped at 72 columns to match the rest of the repo -- rewrapping it
# much narrower will need a third line and trip this.
MAXLINES=4
# Enough that a gutted or stub AGENTS.md fails, well under its real size.
MINAGENTS=2000

fails=0
fail() {
	printf 'FAIL: %s\n' "$1" >&2
	fails=$((fails + 1))
}

if [ ! -f "$AGENTS" ]; then
	fail "AGENTS.md is missing -- CLAUDE.md's @AGENTS.md import would silently resolve to nothing"
fi
if [ ! -f "$CLAUDE" ]; then
	fail "CLAUDE.md is missing -- Claude Code sessions would load no project rules"
fi
if [ "$fails" -ne 0 ]; then
	exit 1
fi

agentsbytes=$(wc -c < "$AGENTS" | tr -d ' ')
claudebytes=$(wc -c < "$CLAUDE" | tr -d ' ')

# AGENTS.md holds the real content.
if [ "$agentsbytes" -lt "$MINAGENTS" ]; then
	fail "AGENTS.md is only $agentsbytes bytes; expected at least $MINAGENTS -- has its content been moved or lost?"
fi
if ! grep -q '^## Development workflow' "$AGENTS"; then
	fail "AGENTS.md has no '## Development workflow' section -- the workflow every agent follows should live there"
fi

# Steps 6, 7 and 8 hold their review passes to one standard, and say it
# in one form of words so a reader can see that they agree. They did not
# always: two said "address every actionable finding" and the middle one
# said "fix or explicitly resolve", while the first claimed to be
# stating the same rule "as with the two steps below". Nobody noticed
# until a reviewer read all three together.
#
# Counted, not located: hard-coding line numbers would break on every
# edit above them, and nothing here can tell which step a match fell in.
# So this catches the one-place regression -- a step reworded on its own
# -- and not a coordinated edit that reworded a step and added the
# phrase somewhere else to keep the total at four. Issue #65 is where
# that check would live -- as filed it checks numbering, not which step
# a match fell in; the comment thread there asks for the attribution it
# would need.
#
# Counted over the file with its line breaks flattened, because the
# phrase wraps mid-sentence in two of the four places it appears: a
# check a rewrap can break is a check that gets deleted the first time
# someone rewraps. The phrase holds no regex metacharacter, so passing
# it to awk as a pattern matches it literally; one added later would
# quietly turn this into something else.
STANDARD='actionable finding, or resolve it explicitly'
# Three steps state it and one section defines it. Stating it a fifth
# time on purpose means changing this number on purpose.
#
# "The Codex review" binds a fourth pass to the same standard and is
# deliberately not counted: it says so in its own words rather than in
# this phrase, and rewording it into the phrase to make it countable
# would be a bigger edit than the coverage is worth. That is the one
# place the standard can drift where this cannot see it.
STANDARDS=4
# Awk's status and its output are both checked, because either can lie
# on its own. A non-number means it did not run -- unsupported construct,
# missing binary, anything -- and compared directly against STANDARDS
# that is a silent pass rather than a failure: the test errors "Illegal
# number", `if` reads the non-zero status as false, and the script goes
# on to print "instructions OK". A number from a command that then
# failed is the same hole from the other side: a shadowed awk printing
# "4" and exiting 17 satisfied the number check, and did it on a file
# whose step 8 had been reworded back to the old standard -- green,
# with the defect present and the counter never having run.
if ! standards=$(awk -v want="$STANDARD" '
	{ s = s " " $0 }
	END { gsub(/[ \t]+/, " ", s); print gsub(want, "&", s) }' "$AGENTS"); then
	fail "could not count the finding standard in AGENTS.md -- awk exited non-zero, so this check did not run whatever it printed"
else
	case "$standards" in
	'' | *[!0-9]*)
		fail "could not count the finding standard in AGENTS.md -- awk printed '$standards' instead of a number, so this check did not run"
		;;
	*)
		if [ "$standards" -ne "$STANDARDS" ]; then
			fail "AGENTS.md states the finding standard $standards times, expected $STANDARDS -- steps 6, 7 and 8 each state it and 'Answering a finding' defines it. A step reworded on its own no longer visibly agrees with the others; if you added a statement of it on purpose, update STANDARDS in this script"
		fi
		;;
	esac
fi
SECTION='Answering a finding'
if ! grep -q "^### $SECTION" "$AGENTS"; then
	fail "AGENTS.md has no '### $SECTION' section -- step 6 points at it by name, and it is where the standard the three steps state is defined"
fi
# And the pointer to it, which is the one thing tying the numbered steps
# to the definition. Delete just that clause and the count above still
# reads 4 and the heading is still there: the section goes on existing
# with nothing sending a reader to it, which loses the half of #97 that
# asked where a disagreement is recorded. Matched in quotes because the
# heading itself is unquoted, so this finds the reference and not the
# thing referred to.
# Before the heading, specifically. A reference below it is a
# back-reference and cannot send anyone anywhere: the reader is already
# there. Counting quoted mentions was enough while step 6 held the only
# one, and stopped being enough the moment a later section quoted the
# name too -- that alone would satisfy a count while step 6's pointer
# was deleted, which is the state this check exists to catch.
#
# Flattened first, like the count above and for the same reason: the
# pointer is a quoted phrase in wrapped prose, and a rewrap that split
# it across two lines would fail here while the pointer sat there
# intact. What it cannot see is which step the pointer is in, or a
# layout that put the section above the steps -- the second would fail
# it wrongly, and is why the message says what it checked rather than
# what it means.
if ! points=$(awk -v s="$SECTION" '
	{ all = all " " $0 }
	END {
		gsub(/[ \t]+/, " ", all)
		h = index(all, "### " s)
		q = index(all, "\"" s "\"")
		print (q > 0 && h > 0 && q < h) ? "yes" : "no"
	}' "$AGENTS"); then
	fail "could not look for the pointer to \"$SECTION\" in AGENTS.md -- awk exited non-zero, so this check did not run"
else
	# Answered, not merely exited: an awk that prints nothing and
	# succeeds would otherwise pass this the way one printing a
	# plausible number passed the count above before it was guarded.
	case "$points" in
	yes) ;;
	no)
		fail "nothing in AGENTS.md points forward at \"$SECTION\" -- the section defines the standard steps 6, 7 and 8 state, and nothing above it now sends a reader to it"
		;;
	*)
		fail "could not look for the pointer to \"$SECTION\" in AGENTS.md -- awk printed '$points' instead of yes or no, so this check did not run"
		;;
	esac
fi

# CLAUDE.md points at it and holds nothing else.
if ! grep -q '^@AGENTS\.md[[:space:]]*$' "$CLAUDE"; then
	fail "CLAUDE.md has no '@AGENTS.md' import line -- Claude Code would not load the shared rules"
fi
# Blank out the one allowed import and see if any other remains. An
# import works mid-line too, so matching only at the start of a line
# would miss `... see @RULES.md:` -- four lines, small, and a second
# source of truth in every session. Only whole tokens are blanked, or
# `@AGENTS.md.bak` would be read as the allowed import plus nothing --
# and since `.` has to count as part of a token for that, sentence
# punctuation after the import is absorbed too.
if sed -E 's#@AGENTS\.md([.,;:!?]?)([^A-Za-z0-9._/-]|$)#\1\2#g' "$CLAUDE" \
		| grep -q '@[A-Za-z0-9._/-]'; then
	fail "CLAUDE.md imports a file other than AGENTS.md -- rules should reach agents through AGENTS.md alone"
fi
if [ "$claudebytes" -gt "$MAXBYTES" ]; then
	fail "CLAUDE.md is $claudebytes bytes, over the $MAXBYTES limit -- rules belong in AGENTS.md, not here"
fi

claudelines=$(grep -cv '^[[:space:]]*$' "$CLAUDE")
if [ "$claudelines" -gt "$MAXLINES" ]; then
	fail "CLAUDE.md has $claudelines non-blank lines, expected at most $MAXLINES -- if you added a rule, move it to AGENTS.md; if you reworded the pointer, update MAXLINES in this script"
fi

# Rules arrive as list items or new sections. The pointer needs neither,
# so their presence below the title means content landed in the wrong file.
if tail -n +2 "$CLAUDE" | grep -qE '^[[:space:]]*(#|[-*+][[:space:]]|[0-9]+\.[[:space:]])'; then
	fail "CLAUDE.md contains headings or list items below the title -- move that content to AGENTS.md"
fi

# Claude Code also loads CLAUDE.md files from subdirectories (when working
# under them) and from .claude/. Any of those is a second shared source of
# truth, so only the root pointer may exist. A personal, unshared
# CLAUDE.local.md at the root is deliberately left alone -- .gitignore
# keeps it from becoming a shared one.
#
# Matched case-insensitively: macOS filesystems are case-insensitive by
# default, so a lone claude.md there would load like the real thing.
# The prune list mirrors .gitignore's build trees rather than any
# directory merely starting with "build".
extra=$(find "$ROOT" \( -name .git -o -name build -o -name 'build-*' \) -prune -o \
	-iname 'CLAUDE*.md' \
	! -ipath "$ROOT/CLAUDE.md" \
	! -ipath "$ROOT/CLAUDE.local.md" \
	-print 2>/dev/null)
if [ -n "$extra" ]; then
	fail "these files also carry instructions; fold them into AGENTS.md:"
	echo "$extra" | sed 's/^/         /' >&2
fi

if [ "$fails" -ne 0 ]; then
	printf '\n%d check(s) failed.\n' "$fails" >&2
	printf 'AGENTS.md is the source of truth for every coding agent; CLAUDE.md only imports it.\n' >&2
	printf 'Put project rules in AGENTS.md and leave CLAUDE.md as the pointer.\n' >&2
	exit 1
fi

printf 'instructions OK (CLAUDE.md %s bytes, AGENTS.md %s bytes)\n' "$claudebytes" "$agentsbytes"
