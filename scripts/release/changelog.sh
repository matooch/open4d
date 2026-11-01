#!/bin/sh
# Generate release notes from git history, no external service.
#   sh scripts/release/changelog.sh [TAG] > CHANGELOG.md
# TAG defaults to HEAD. The range is "previous v* tag .. TAG"; with no previous tag it is the
# whole history up to TAG. Commits are grouped by the conventional-ish prefix of the summary
# line when present, otherwise listed under "Changes". Merge commits are skipped.
set -eu

TAG=${1:-HEAD}
PREV=""
if [ "$TAG" = HEAD ]; then
  PREV=$(git describe --tags --abbrev=0 --match 'v*' 2>/dev/null || true)
else
  PREV=$(git describe --tags --abbrev=0 --match 'v*' "$TAG^" 2>/dev/null || true)
fi

if [ -n "$PREV" ]; then RANGE="$PREV..$TAG"; else RANGE="$TAG"; fi

echo "## ${TAG}"
echo
if [ -n "$PREV" ]; then echo "Changes since ${PREV}."; else echo "First release: all changes."; fi
echo

TAB=$(printf '\t')
# One line per commit: short hash <TAB> subject.
LOG=$(git log --no-merges --pretty=format:"%h${TAB}%s" "$RANGE")
if [ -z "$LOG" ]; then echo "No changes."; exit 0; fi

section() { # title, grep-regex for the subject
  title=$1; re=$2
  out=$(printf '%s\n' "$LOG" | awk -F"$TAB" -v re="$re" '$2 ~ re { printf "- %s (%s)\n", $2, $1 }')
  if [ -n "$out" ]; then
    echo "### $title"; echo; echo "$out"; echo
  fi
}

# Brief-style summaries end in "(brief NNN)"; other lines fall under Changes.
section "Fixes" '^([Ff]ix|[Bb]ugfix)'
section "Features and changes" '^([^Ff]|F[^i]|Fi[^x])'
