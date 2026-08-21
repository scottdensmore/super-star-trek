#!/bin/sh
# Lint .github/workflows, which nothing here used to look at.
#
# Every `run:` body is a shell script, and they are the scripts least
# likely to be exercised before they matter: a step guarded by
# `if: runner.os == 'macOS'` runs only on a runner nobody can reproduce
# locally, so a mistake there arrives as a red leg on somebody else's
# pull request. That is how a call to timeout(1) -- GNU coreutils, absent
# from the macOS images -- reached review on the #186 branch. #106 makes
# the same argument for tests/*.sh; this is #187, one directory over.
#
# actionlint rather than something hand-rolled, and that is the whole
# design decision. The first attempt at this test extracted each `run:`
# body with awk and piped it through shellcheck, on the reasoning that
# actionlint was not installed and a YAML library could not be assumed.
# The verifier took that apart: ordinary YAML broke it six ways --
# a `defaults: run:` mapping key, a body indented other than exactly two
# spaces deeper, `format('{0}-{1}', ...)` defeating the expression
# substitution, a `uses:`-only workflow counted as a failure, an empty
# workflow directory counted as a pass -- and the count check meant to
# stop a silent pass could itself be routed around, because an
# uninitialised awk variable printed an empty string and `[ -ne ]` then
# failed with status 2, which `if` reads as false. It reported
# "workflow OK" over zero scripts.
#
# The answer to that is not a better hand-rolled parser. actionlint
# parses the workflow schema, checks expressions, and runs shellcheck
# over each `run:` body with the GitHub expressions substituted the way
# they have to be -- all the things the awk was approximating, plus
# checks it could not reach at all. Measured: it catches an unquoted
# variable (SC2086), a shell syntax error (SC1072), and `matrix.oss`
# where the matrix defines `os`, which no amount of shell linting would
# have found.
#
# What it still cannot catch, since #187 asked for it and the honest
# answer is worth keeping: that a command exists on one runner image and
# not another. `timeout 300 brew ...` on the macOS leg lints clean. That
# defect is a matter of knowing the image, not the language -- and it is
# the one #187's own worked example is about, so it is filed as #196
# rather than left as a shrug. #187 suggests the cheap form there: a
# workflow calling timeout(1) unguarded, which is what bit #186.
#
# Usage: workflow.sh [path/to/repo/root]
#
# Skips 77 without actionlint, which is how a developer sees it. CI is
# different only where CI installs it -- Linux -- and there a skip would
# mean the coverage quietly vanished. Same shape as tests/analyze.sh,
# which scopes its hard failure the same way and for the same reason.

set -u

here=$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)
root="${1:-$here/..}"
flows="$root/.github/workflows"

if [ ! -d "$flows" ]; then
	echo "FAIL: no $flows to lint" >&2
	exit 1
fi

# Named explicitly rather than letting actionlint find the repository
# itself: it looks for the nearest .git from the working directory, so a
# copy of the tree outside a repository -- which is where this project
# proves its tests can fail -- would otherwise get "no project was
# found" and no linting.
set --
for flow in "$flows"/*.yml "$flows"/*.yaml; do
	[ -f "$flow" ] && set -- "$@" "$flow"
done
if [ "$#" -eq 0 ]; then
	echo "FAIL: no workflow files in $flows to lint" >&2
	exit 1
fi

unavailable() {
	if [ -n "${CI:-}" ] && [ "$(uname -s)" = Linux ]; then
		printf 'FAIL: %s on Linux CI, so the workflows went unlinted\n' \
			"$1" >&2
		exit 1
	fi
	printf 'SKIP: %s, so the workflows went unlinted\n' "$1" >&2
	exit 77
}

if ! command -v actionlint >/dev/null 2>&1; then
	unavailable "no actionlint"
fi

# The canary, and it is the whole reason this is not three lines long.
# actionlint's shellcheck integration is opt-out by absence: where that
# binary is not on PATH it drops every SC check and still exits 0, so a
# runner image without it would lint the schema, report success, and
# leave the run: bodies -- the half this test exists for -- unchecked.
# The first version of this file trusted the exit status and did
# exactly that, and the verifier proved it with the same workflow
# passing and failing depending only on whether shellcheck was on PATH.
#
# (No comment line below may begin with the name of that tool: a line
# starting "# shellch" + "eck" is read as a directive addressed to it,
# and faults the file. Found by linting this very file, which is why
# the name is split there.)
#
# So: hand actionlint a workflow whose only defect is one shellcheck
# would catch, and require it to complain. If it does not, the
# integration is inert and this skips rather than passing -- the same
# shape as tests/analyze.sh's use-after-free canary, which is what
# stops a compiler that takes -fanalyzer and reports nothing from
# looking like a clean analysis.
# The $ below is meant to reach actionlint literally, not to expand.
# shellcheck disable=SC2016
canary='on: push
jobs:
  c:
    runs-on: ubuntu-latest
    steps:
      - run: echo $UNQUOTED/x
'
canary_out=$(printf '%s' "$canary" | actionlint -no-color - 2>&1)
canary_rc=$?
# Three outcomes, told apart rather than lumped together. actionlint
# exits 1 when it has something to report, 0 when it has nothing, and
# 2 or more when it could not do the job at all -- measured against
# 1.7.12: 1 for the canary, 0 for a clean workflow, 3 for a file it
# cannot read. A first version of this checked only whether SC2086
# came back, so an actionlint that had failed outright reported
# "cannot run shellcheck", which on a Linux CI leg is a hard red with
# the wrong cause on it. Raised by the verifier on the #187 branch.
if [ "$canary_rc" -ge 2 ]; then
	printf '%s\n' "$canary_out" >&2
	unavailable "actionlint could not read the canary (status $canary_rc)"
elif [ "$canary_rc" -eq 0 ] ||
     ! printf '%s\n' "$canary_out" | grep -q 'SC2086'; then
	# What is left after that split: an exit 1 carrying some other
	# complaint about the canary -- a future actionlint stricter
	# about a workflow this one accepts, say -- is indistinguishable
	# from the inert case by status, and gets named as inert. So
	# print what it actually said, since on Linux CI this is a hard
	# failure and the message alone would send the reader after the
	# wrong tool.
	# Guarded: the inert case is the common one and exits 0 saying
	# nothing, where an unguarded printf puts a blank line above the
	# message.
	[ -n "$canary_out" ] && printf '%s\n' "$canary_out" >&2
	unavailable "actionlint cannot run shellcheck"
fi

# Proven able to fail, on a copy of the tree outside the repository,
# once per half: `run: echo $HOME/x` appended as a step faulted SC2086,
# and `if: ${{ github.nosuchcontext.x }}` on the job faulted the
# expression check. Both exited 1 here; restoring the file returned it
# to "workflow OK". The canary above was proven the same way, against
# three PATHs -- real actionlint (lints), a shellcheck-less one (skips
# "cannot run shellcheck"), and a stub exiting 3 (skips naming the
# status).
if ! actionlint -no-color "$@"; then
	printf 'FAIL: actionlint faulted %d workflow file(s)\n' "$#" >&2
	exit 1
fi
echo "workflow OK ($# file(s))"
