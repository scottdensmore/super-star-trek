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
