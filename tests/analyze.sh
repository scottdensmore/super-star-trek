#!/bin/sh
# Static analysis over the game's C sources.
#
# This is the `analyze` test registered in CMakeLists.txt. Its predecessor
# depended on cppcheck, clang-tidy or scan-build being installed, and none
# was available on the development machine or in CI, so the leg never ran.
# Issue #147 replaced that optional check with an enforced one.
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
# A third, for the depth below: the same read planted at the top of
# sst.c's cramen(), on a copy outside the repository, which this script
# passed at gcc's default depth and fails at the one it now asks for.
# The fallback has its own, driven by a shim on the path that forwards
# to gcc and rejects that --param the way a gcc too old for it would:
# the run notes it, reports the default depth on the line it prints,
# and goes green -- the weaker check it should be, said out loud rather
# than passed off as this one. With CI=1 the same shim exits 1 instead,
# which is the gate below.
#
# How far the analyzer reaches is a budget, and at gcc's default it did
# not reach far enough. Measured by planting this read at the top of
# five functions and compiling with the flags below:
#
#	{ volatile int c = 0; int u; if (c) u = 1; if (u == 42) c = 2; }
#
# Only -fanalyzer sees it: -Wall -Wextra at -O0 say nothing about it,
# so a catch is the analyzer leg and not the plain warning set. Two
# shorter candidates were no use, since the analyzer does not flag a
# read into a volatile at all and an unconditional read is flagged by
# plain -Wuninitialized as well. At the default the read is caught in
# setup.c's newqad() and battle.c's phasers() and missed in sst.c's
# cramen(), tui.c's tui_shutdown() and battle.c's cloak(). Per function
# rather than per file: battle.c catches one of its own, misses another.
#
# --param=analyzer-bb-explosion-factor is the dial, default 5, and it
# is raised to 30 below. Each of the three comes back at its own value,
# measured with gcc 15.2.0 by planting the read again at each: cloak()
# at 7 and not at 6, tui_shutdown() at 8, cramen() at 25 and not at 24.
# Those cliffs belong to that exact read and not to the function: a
# shorter one planted in cramen() came back at 10 instead of 25,
# because what is planted decides how much budget is left to walk the
# rest. Both ends hold whichever was used -- missed at 5, caught at 30.
# 30 clears the deepest cliff measured with room over it, and is not a
# value anything in the tree needs today.
#
# What raising it costs is time and nothing else so far. Over the
# thirteen sources on this machine, each figure more than one run:
# 15-17s at the default, 31-36s at 30, about 46s at 60, with zero
# diagnostics at every one of them -- and zero under the Release
# configuration's flags too, which analyse different code for want of
# -DDEBUG. So the reason to stop at 30 is not that the tree
# reddens beyond it; it is that no function measured here needs more,
# and every ctest run pays the difference. The CMakeLists.txt timeout
# was raised with it.
#
# Those are this machine's numbers on this gcc, and the compiler CI
# runs is neither. Checked against it rather than hoped for: on the
# gcc 13.3.0 that ubuntu-24.04 carries, in a container of that image,
# the --param is accepted, the tree is clean at 30 under both
# configurations' flags, and the read planted in cramen() is missed at
# the default and caught at 30 there too. CI's own analyze leg runs
# faster than this one by a factor the compile times do not explain,
# so take the timings above as an upper bound for it rather than a
# prediction.
#
# The cost that has not been paid yet is the other kind: deeper analysis
# can surface more findings of the newqad() kind, which on a -Werror
# check cost a rewrite or a suppression each. None have appeared, here
# or at 60.
#
# What a passing run still does not say is where the analyzer gave up
# anyway. -Wanalyzer-too-complex reports that and is off by default;
# turned on at the default factor it fires 169 times in battle.c, 142
# in setup.c, 46 in sst.c and 0 in five of the thirteen. It cannot be
# made fatal for that reason, and it does not fall with the factor
# raised -- at 30 sst.c gives 298, because the analyzer explores
# further before it stops. #171 has that half, which this does not
# close.
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

# The depth the comment above measured, asked of the compiler rather
# than assumed of it. A gcc that does not know this --param exits on it
# before compiling anything, so every source would fail for a reason
# that has nothing to do with the game -- and the gcc CI runs is not
# the one this was written against. Compiling the canary with it is the
# same question the candidate loop just asked, so a compiler that takes
# the option and stops reporting is caught here too.
factor=30
depth="--param=analyzer-bb-explosion-factor=$factor"
"$cc" -fanalyzer "$depth" -c -o "$work/canary.o" "$work/canary.c" \
    2>"$work/depth.txt" || true
if ! grep -q 'Wanalyzer-use-after-free' "$work/depth.txt"; then
	# Gated on Linux CI for the same reason the missing-compiler case
	# above is, and more sharply: there it is this check quietly
	# reverting to the reach #171 exists to fix, with cramen(), cloak()
	# and tui_shutdown() unanalysed again.
	#
	# Elsewhere it falls back and says so, on the line it prints as
	# well as in the note. Both reach a reader only where the output
	# does -- running this script directly, or ctest -V -- because
	# ctest prints nothing at all for a test that passes and
	# --output-on-failure waits for a failure that will not come. So
	# the asymmetry is not that the fallback is loud off CI; it is that
	# off CI the weaker check is the honest answer for a compiler that
	# cannot do better, and on CI it is a regression in a check the
	# compiler there can do.
	if [ -n "${CI:-}" ] && [ "$(uname -s)" = Linux ]; then
		echo "FAIL: $cc does not analyse with $depth, so Linux CI would have run at gcc's default depth" >&2
		exit 1
	fi
	echo "NOTE: $cc does not analyse with $depth; using its default depth" >&2
	factor=default
	depth=""
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
	# Both expansions are unquoted, for two different reasons: $flags is
	# a flag list rather than one word, and $depth is empty on the
	# fallback path, where quoting it would hand gcc one empty argument
	# instead of none -- which it reads as a linker input it cannot
	# find, failing all thirteen sources for a reason that has nothing
	# to do with the game.
	# shellcheck disable=SC2086
	if ! "$cc" -fanalyzer $depth $flags -Werror -c -o "$work/obj.o" "$src" \
	     2>"$out"; then
		echo "FAIL: $(basename "$src") did not analyse clean" >&2
		cat "$out" >&2
		status=1
	fi
done

if [ "$status" -ne 0 ]; then
	exit 1
fi

# The depth is on the line a passing run prints because a passing run
# is where it matters: the same output from a compiler that fell back
# to the default would be a weaker check reading as this one.
printf 'analyze OK (%s, bb-explosion-factor %s)\n' "$cc" "$factor"
