#!/bin/sh
# Static analysis over the game's C sources.
#
# This is the check .claude/agents/verifier.md's step 2 asks for. It used
# to ask for cppcheck, clang-tidy or scan-build "if installed", and none
# of the three was installed on the development machine or in CI -- so
# the leg never ran anywhere, the verifier reported the gap honestly on
# every pass, and the gap stayed open. #147.
#
# GCC's own analyzer is what the project actually has: no install, and
# already the compiler CI builds with on Linux. It is a different check
# from the -Wall -Wextra -Wmissing-prototypes the ci-* presets make
# fatal -- those are what the compiler notices while generating code,
# where this walks paths and tracks what has been written to.
#
# Usage: analyze.sh "<compile flags>" "<compiler>" "<sources;...>"
# Exits 77 (ctest SKIP_RETURN_CODE) where no analyzer-capable compiler
# is available, except on Linux CI, where it fails instead.
#
# Proved to fail two ways, each on a copy of the tree outside the
# repository: a use-after-free planted in sst.c, caught as
# -Werror=use-after-free, and setup.c put back to the form that reads
# an ix only the loop above it sets, caught as
# -Werror=analyzer-use-of-uninitialized-value -- which is the analyzer
# leg specifically, since the plain warning set does not see it.
#
# What it does not prove is how far the analyzer reaches. gcc's reach
# is uneven and it says nothing where it gives up. Measured by planting
# the same uninitialized read in five functions:
# caught in setup.c's newqad() and battle.c's phasers(), missed in
# sst.c's cramen(), tui.c's tui_shutdown() and battle.c's cloak(). So
# it is per function rather than per file -- battle.c catches one of
# its own and misses another -- and a passing run says which files were
# compiled, not which were understood.
#
# Where it stops is a budget. Measured with gcc 15.2.0 against
# --param=analyzer-bb-explosion-factor, whose default is 5: cloak()
# comes back at 7 and not at 6, cramen() not until 30. The dial is not
# raised here, and the reason is not that the tree would redden -- at 7
# and at 12 it still reports nothing at all. It is that the cost is
# time, 16 seconds over the sources at the default against 27 at 12,
# and that deeper analysis can surface more findings of the newqad()
# kind, which on a -Werror check cost a rewrite or a suppression each.
# None have appeared yet. #171 has the rest.
#
# An analyzer gone inert is the third case and it does not fail here.
# The canary is the probe below, so a compiler that takes -fanalyzer
# and reports nothing is passed over rather than failing the run: with
# no other candidate the script skips, and on Linux CI that skip is
# itself a failure. Proved by compiling the canary with -fsyntax-only,
# which analyses nothing.
#
# Both of those need ctest to see them properly: exiting 77 is only a
# skip if SKIP_RETURN_CODE says so, and it did not at first -- the
# proofs above ran this script directly and could not see that. Run
# with no analyzer-capable compiler, ctest reports Skipped; with CI=1
# on Linux it reports Failed.

set -u

# The sources live beside this script's parent, and that is how the
# other tests here find what they read.
srcdir=$(unset CDPATH; cd -- "$(dirname -- "$0")/.." && pwd)

# Handed the build's own flags rather than restating them here, which
# is what CMakeLists.txt passes and why. -Werror is added whatever the
# preset says: this test is fatal on a diagnostic even where the build
# it was configured from is not.
flags="${1:-}"
buildcc="${2:-}"
sources="${3:-}"
if [ -z "$flags" ] || [ -z "$sources" ]; then
	echo "usage: analyze.sh \"<flags>\" \"<compiler>\" \"<sources;...>\"" >&2
	exit 2
fi

# A template, and the status checked. Bare `mktemp -d` is a GNU
# extension: BSD mktemp, which is what macOS has, prints usage and
# exits 1. Unchecked that leaves $work empty, every path below absolute,
# the probe unable to write, and the script exiting 77 -- a green macOS
# job with nothing analysed, which is the shape of hole #147 is about,
# rebuilt inside the fix for it.
work=$(mktemp -d "${TMPDIR:-/tmp}/sst-analyze.XXXXXX") || exit 1
[ -n "$work" ] && [ -d "$work" ] || exit 1

# One trap per signal, each exiting. A handler that only cleans up
# returns to where the signal landed, so an interrupted run would carry
# on compiling into a directory it had just deleted and report thirteen
# analysis failures that were nothing of the kind. The one-trap-per-
# signal shape is tests/tui.sh's, though its note beside it is about a
# closed pipe under dash rather than about this.
trap 'rm -rf "$work"' EXIT
trap 'rm -rf "$work"; exit 129' HUP
trap 'rm -rf "$work"; exit 130' INT
trap 'rm -rf "$work"; exit 143' TERM
trap 'rm -rf "$work"; exit 141' PIPE

# What a working analyzer has to do, in one file: accept -fanalyzer, and
# actually report on it. The second half is the point -- see the note on
# -fsyntax-only below -- and it is asked of each candidate rather than
# once of the winner, so a compiler that takes the flag and analyses
# nothing is passed over rather than failing the run.
cat > "$work/canary.c" <<'CANARY'
#include <stdlib.h>
int canary(void);
int canary(void) {
	int *p = malloc(sizeof(int));
	free(p);
	return *p;			/* use after free */
}
CANARY
[ -s "$work/canary.c" ] || exit 1

# Asking the version string is not enough: macOS ships clang as `gcc`,
# which takes -fanalyzer only as far as an error. Compiling is the
# question worth asking.
#
# Darwin tries only CC and the compiler the build was configured with.
# The runners build with clang but often carry a Homebrew gcc-N, which
# would otherwise be found here and turned loose on the macOS SDK under
# -Werror -- a red job from a Darwin-only warning or an Xcode bump, on
# a check Linux has already delivered. Naming CC, or configuring the
# tree with such a compiler, asks for it there deliberately.
#
# The gcc-N list needs bumping as versions land; without it a machine
# whose only GCC is gcc-16 skips quietly.
candidates="${CC:-} $buildcc gcc gcc-15 gcc-14 gcc-13"
if [ "$(uname -s)" = Darwin ]; then
	# Restricted to CC and the compiler the build was configured
	# with, rather than falling through to any gcc on the path: with
	# CC=clang exported and a Homebrew gcc-N installed, that would
	# find the gcc and turn it loose on the macOS SDK under -Werror.
	# The build's own compiler stays in because a tree configured
	# with GCC on Darwin has already accepted that; AppleClang fails
	# the canary and skips.
	candidates="${CC:-} $buildcc"
fi
cc=""
for candidate in $candidates; do
	command -v "$candidate" >/dev/null 2>&1 || continue
	"$candidate" -fanalyzer -c -o "$work/canary.o" "$work/canary.c" \
	    2>"$work/canary.txt" || true
	if grep -q 'Wanalyzer-use-after-free' "$work/canary.txt"; then
		cc="$candidate"
		break
	fi
done

if [ -z "$cc" ]; then
	# Linux CI builds with GCC, so a skip there would mean the analyzer
	# had gone missing from the one place it is guaranteed. macOS CI
	# builds with clang and is expected to skip.
	if [ -n "${CI:-}" ] && [ "$(uname -s)" = Linux ]; then
		echo "FAIL: no compiler with -fanalyzer on Linux CI, so nothing was analysed" >&2
		exit 1
	fi
	echo "SKIP: no compiler whose -fanalyzer reports anything" >&2
	exit 77
fi

# The sst target's own sources, as CMake lists them -- osx.c included,
# and tests/test_tuifmt.c and tests/test_rules.c not, those being test
# harnesses rather than the game. Taken from the build rather than
# globbed here so a source added later cannot go unanalysed quietly.
#
# -Werror, so any diagnostic fails rather than a pattern deciding which
# count. Grepping for -Wanalyzer looked right and was not: a planted
# use-after-free in sst.c came back as plain -Wuse-after-free, which
# -Wall raises before the analyzer gets to it, and the run reported
# clean. The ci-* presets already make -Wall -Wextra fatal, so anything
# new that stops the compile here is either an analyzer finding or
# something those builds would have caught anyway.
# Split the semicolon list CMake passes into positional parameters and
# put IFS back before anything else is expanded -- left set, it splits
# $flags into a single argument and every compile fails on an
# unrecognized option that is the whole flag string.
oldifs=$IFS
IFS=';'
# shellcheck disable=SC2086  # splitting on ';' is the point here
set -- $sources
IFS=$oldifs

status=0
for src in "$@"; do
	case "$src" in /*) ;; *) src="$srcdir/$src" ;; esac
	out="$work/$(basename "$src").txt"
	# shellcheck disable=SC2086  # $flags is a flag list, not one word
	if ! "$cc" -fanalyzer $flags -Werror -c -o "$work/obj.o" "$src" \
	     2>"$out"; then
		echo "FAIL: $(basename "$src") did not analyse clean" >&2
		cat "$out" >&2
		status=1
	fi
done

if [ "$status" -ne 0 ]; then
	exit 1
fi

printf 'analyze OK (%s)\n' "$cc"
