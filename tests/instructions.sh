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
#        instructions.sh /path/to/repo/root --body '<section name>'
#
# The second form runs no checks. It prints the size of one AGENTS.md
# section, in the units the section floors below are compared against,
# and is there so re-aiming a floor does not mean copying the awk out
# of this file. See the comment above check_section.

set -u

ROOT="${1:-.}"
while [ "${ROOT%/}" != "$ROOT" ] && [ "$ROOT" != "/" ]; do
	ROOT="${ROOT%/}"
done
CLAUDE="$ROOT/CLAUDE.md"
AGENTS="$ROOT/AGENTS.md"

# The one copy of the body measurement: check_section compares against
# it, and --body prints it. Defined up here rather than beside its
# caller so --body can answer before any check runs -- the moment you
# most want to measure a section is just after its floor fired, and a
# form that had to reach the bottom of the script would be unusable
# exactly then.
#
# shellcheck disable=SC2016  # $0 belongs to awk here, not to the shell
AGENTS_BODY_AWK='
	index($0, h n) == 1 { inb = 1; next }
	inb && index($0, h) == 1 { inb = 0 }
	inb { b += length($0) + 1 }
	END { print b + 0 }'

if [ "${2:-}" = "--body" ]; then
	# The name is required, and refusing an empty one is the point: with
	# n="" the pattern matches every `### ` heading, so the run measures
	# the whole file and returns a large, plausible number for a command
	# that did nothing you meant. Same hazard the guards further down
	# exist for, reached by a typo instead of a broken awk.
	if [ $# -lt 3 ] || [ -z "$3" ]; then
		printf 'usage: %s <repo-root> --body <section name>\n' "$0" >&2
		exit 2
	fi
	if [ ! -f "$AGENTS" ]; then
		printf 'no such file: %s\n' "$AGENTS" >&2
		exit 2
	fi
	# Rejected on the measurement rather than on a separate `grep`,
	# which would have been a second matcher to disagree with the
	# first: `grep` takes a BRE, so `The C.dex review` would have
	# satisfied it and then measured nothing, handing back 0 as though
	# it were an answer.
	if ! measured=$(awk -v h="### " -v n="$3" "$AGENTS_BODY_AWK" "$AGENTS"); then
		printf 'could not measure "### %s": awk exited non-zero\n' "$3" >&2
		exit 2
	fi
	if [ "$measured" = 0 ]; then
		printf 'AGENTS.md has no "### %s" section, or it is empty\n' "$3" >&2
		exit 1
	fi
	printf '%s\n' "$measured"
	exit 0
fi

# Only reachable if the branch above did not dispatch. check_section
# re-invokes this script with --body, so a break that stops the
# dispatch firing -- deleting its `exit`, mistyping the flag -- would
# have each child run the checks, re-invoke, and fork again without
# bound: a chain still growing at hundreds of processes, orphaned to
# init, outliving the ctest timeout that reaps the top of it. Refusing
# here turns the one break this check cannot otherwise catch into a
# message. The marker is set on the child in check_section.
if [ "${INSTRUCTIONS_BODY_CHILD:-}" = 1 ]; then
	printf '%s: --body did not dispatch; refusing to recurse\n' "$0" >&2
	exit 2
fi

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
# A section the numbered steps hand work to needs three things, and
# each fails silently without the others: a heading, a pointer above it,
# and something under the heading. A heading nothing references is a
# section no reader is sent to; a pointer with no heading under it sends
# them nowhere; and a heading and a pointer with a stub between them
# satisfy the first two while answering nothing. So all three are
# checked, for every section a step defers to by name.
#
# Names are interpolated into a grep BRE below and into awk's literal
# index() everywhere else. Neither name holds a regex metacharacter, so
# the two agree today; one added later would make the heading check
# something other than the literal it looks like, the same hazard the
# comment on STANDARD above describes.
#
# The pointer is matched in quotes because the heading itself is
# unquoted, so this finds the reference and not the thing referred to.
# Before the heading, specifically: a mention below it is a
# back-reference and cannot send anyone anywhere, since the reader is
# already there. Counting quoted mentions was enough while "Answering a
# finding" had one pointer and one heading, and stopped being enough the
# moment a later section quoted the name too -- that alone would satisfy
# a count while step 6's pointer was deleted, which is the state this
# exists to catch.
#
# Flattened first, like the count above and for the same reason: the
# pointer is a quoted phrase in wrapped prose, and a rewrap that split
# it across two lines would fail here while the pointer sat there
# intact. What it cannot see is which step the pointer is in, or a
# layout that put the section above the steps -- the second would fail
# it wrongly, and is why the messages say what was checked rather than
# what it means. Nor how many pointers there are: "The Codex review" has
# two, in steps 11 and 12, and either one alone satisfies this. An
# existence check cannot do better, and the case it is here for is the
# last one going, not the first.
#
# Nor a heading that merely starts with the name. All three matches here
# are prefix matches, so renaming the heading to "### The Codex review
# process" satisfies every one of them while both pointers name a
# heading that no longer exists -- the exact state they are here to
# catch. That is issue #105, and it predates this function: the two
# inline checks it was factored out of matched the same way.
#
# The third argument is a floor under the section's body, in the same
# spirit as MINAGENTS above and for the same reason: the heading and the
# pointer are two lines, and a section gutted to a stub satisfies both
# while saying nothing. It is a floor, not a budget -- set well under
# the real size, so ordinary editing never touches it and only a
# deletion does. It cannot tell whether what is there is still the right
# content; nothing here can, and that is what review is for.
#
# Aim each floor at about half its section, and re-aim it when the
# section moves. "The Codex review" measured 7954 in the commit that
# added this check, where a 4000 floor was half of it; the change after
# it alone was enough to leave 4000 under a third, with nothing saying
# so.
#
# No current size is written down here on purpose. Every edit to the
# section changes it, so a figure in this comment is stale before the
# commit that adds it lands -- which happened twice while this check was
# being written, each time caught by a reviewer reading a number that
# reproduced against nothing. Measure it instead:
#
#   tests/instructions.sh . --body 'The Codex review'
#
# prints exactly what the floor is compared against, because it runs
# the same program: AGENTS_BODY_AWK, defined at the top of this file
# and used by both. A recipe that restated the program in a comment
# would drift from it the first time either moved, and nothing would
# say so.
#
# The unit is whatever awk counts, which is bytes in mawk and busybox
# and characters where awk is locale-aware -- this file's emoji and
# em-dashes put a couple of hundred between them. At half a section
# that is thousands from mattering, which is one more reason not to set
# a floor close. It is also why nothing here says "characters".
#
# Half, not more, deliberately. "The Codex review" is the most volatile
# section in the file and it is volatile *downward*: its "still
# unwatched" block is written to be pruned as entries get settled, and
# its measured block churns with it, so settling two entries is
# ordinary editing that could cost a couple of thousand characters. A
# floor set close enough to trip on that is a budget, and a budget gets
# lowered reflexively until it means nothing. The floor is for the
# silent stub.
#
# Nothing prompts anyone to revisit this. The check only speaks when a
# section shrinks, and the condition worth noticing -- a floor left far
# under a section that grew -- is silent by construction. So: if you
# are reading this and the ratio has drifted again, that is the thing
# to fix, and it is not evidence that the check is fine.
#
# Bodies run from the heading to the next `### ` or the end of file. A
# `## ` does not end one, so a new top-level section appended after the
# last `### ` would be counted into it and the floor would stop being
# able to fire. That errs lax rather than false, which is the direction
# a floor should err, but it is why the floors are set against the real
# sizes rather than left to look after themselves.
#
check_section() {
	name="$1"
	why="$2"
	minbody="$3"
	if ! grep -q "^### $name" "$AGENTS"; then
		fail "AGENTS.md has no '### $name' section -- $why"
		return
	fi
	if ! body=$(awk -v h="### " -v n="$name" "$AGENTS_BODY_AWK" "$AGENTS"); then
		fail "could not measure the \"$name\" section of AGENTS.md -- awk exited non-zero, so this check did not run"
		return
	fi
	case "$body" in
	'' | *[!0-9]*)
		fail "could not measure the \"$name\" section of AGENTS.md -- awk printed '$body' instead of a number, so this check did not run"
		return
		;;
	esac
	# The comment above hands a reader `--body` as the way to re-aim a
	# floor, and a recipe that prints something other than what the
	# floor was compared against is worse than no recipe at all. It is
	# also the one path here that nothing else runs -- a human reaches
	# for it at the moment their build is already failing, which is the
	# worst moment to discover it broken. So run it, and hold it to the
	# number this check just used.
	if ! flag=$(INSTRUCTIONS_BODY_CHILD=1 sh "$0" "$ROOT" --body "$name" 2>&1); then
		fail "\`tests/instructions.sh . --body '$name'\` failed, so the measurement recipe the comment beside these floors gives a reader does not work: $flag"
	elif [ "$flag" != "$body" ]; then
		fail "\`tests/instructions.sh . --body '$name'\` prints $flag but this check compared against $body -- the recipe and the floor have drifted apart, and the comment beside them says they cannot"
	fi
	if [ "$body" -lt "$minbody" ]; then
		fail "AGENTS.md's \"$name\" section measures $body, under the $minbody floor -- $why. If you shortened it on purpose, lower the floor in this script on purpose; \`tests/instructions.sh . --body '$name'\` prints the same number"
	fi
	if ! points=$(awk -v s="$name" '
		{ all = all " " $0 }
		END {
			gsub(/[ \t]+/, " ", all)
			h = index(all, "### " s)
			q = index(all, "\"" s "\"")
			print (q > 0 && h > 0 && q < h) ? "yes" : "no"
		}' "$AGENTS"); then
		fail "could not look for the pointer to \"$name\" in AGENTS.md -- awk exited non-zero, so this check did not run"
		return
	fi
	# Answered, not merely exited: an awk that prints nothing and
	# succeeds would otherwise pass this the way one printing a
	# plausible number passed the count above before it was guarded.
	case "$points" in
	yes) ;;
	no)
		fail "nothing in AGENTS.md points forward at \"$name\" -- $why, and nothing above it now sends a reader to it"
		;;
	*)
		fail "could not look for the pointer to \"$name\" in AGENTS.md -- awk printed '$points' instead of yes or no, so this check did not run"
		;;
	esac
}

# --body's guards are the other half of the recipe, and check_section
# only ever drives its success path, so nothing would reach them: an
# edit that moved one past the measurement would go unnoticed, which is
# the hazard the empty-name guard was added for in the first place. The
# marker keeps a broken dispatch from forking here as well.
probe_body() {
	INSTRUCTIONS_BODY_CHILD=1 sh "$0" "$ROOT" --body "$1" >/dev/null 2>&1
	printf '%s' "$?"
}
if [ "$(probe_body '')" != 2 ]; then
	fail "\`--body\` with an empty name did not exit 2 -- with no name the measurement matches every heading and returns the size of the whole file, which is a plausible number for a command that did nothing you meant"
fi
if [ "$(probe_body 'No Such Section')" != 1 ]; then
	fail "\`--body\` with a name matching no heading did not exit 1 -- it should refuse rather than report the 0 it measures"
fi
if [ "$(probe_body 'The C.dex review')" != 1 ]; then
	fail "\`--body\` accepted a name that only matches as a regex -- the measurement is literal, so such a name measures nothing and 0 would be returned as though it were an answer"
fi

# Delete just step 6's clause and the count above still reads 4 and the
# heading is still there: the section goes on existing with nothing
# sending a reader to it, which loses the half of #97 that asked where a
# disagreement is recorded.
# About half its section, as above. It was 1200, over 70% of it, which
# is a budget rather than a floor: cutting the section's longest
# paragraph would have tripped it, and the message would then have
# invited lowering the number, which is how a floor decays. Being the
# stabler of the two sections is a reason the budget would rarely have
# bitten, not a reason it was the right shape.
check_section 'Answering a finding' \
	'it is where the standard steps 6, 7 and 8 state is defined' 850
# Steps 11 and 12 defer the whole of the Codex reviewer's handling to
# this one -- what its reactions mean, how to answer its comments, and
# the merge condition in step 12, which has no other definition
# anywhere.
check_section 'The Codex review' \
	'it is the only place the reviewer that steps 11 and 12 defer to is defined' 6500

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
