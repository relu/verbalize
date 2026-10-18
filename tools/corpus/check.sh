#!/usr/bin/env bash
# Corpus coverage report over the fetched samples in .cache/.
#
# `audit` lists units read as an SI composition (`Grad` = `G` + `rad`) that the
# corpus also uses in prose, ranked by word use. It is a triage list, not a
# verdict: a real unit appears in prose too (`kV`, `mGy`), so the list needs a
# human eye — an all-caps author initial (`Palacios MA`) is not megaampere.
#
# `survey` lists the shapes that fell back to a last-resort reading or were
# left verbatim (dominated by leading-zero runs, which read digit-by-digit by
# design).
#
# With `--strict` any suspect token fails the run (a gate for a curated corpus),
# and so does having no corpus to check at all.
# Fetch first; this script does not use the network:
#
#   python3 tools/corpus/fetch.py --lang de --articles 2000
#   python3 tools/corpus/fetch.py --lang de --source opus --opus-corpus ECDC
#   tools/corpus/check.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$root"
cli=(cargo run -q -p verbalize-cli --)

strict=0
[ "${1:-}" = "--strict" ] && strict=1

status=0
checked=0
for lang in de en ro; do
    dir="$here/.cache/$lang"
    # fetch.py creates the directory before downloading, so it can be empty.
    if [ -z "$(find "$dir" -type f -print -quit 2>/dev/null)" ]; then
        echo "skip $lang: no corpus in $dir"
        continue
    fi
    checked=$((checked + 1))
    echo "== $lang: audit"
    if [ "$strict" = 1 ]; then
        "${cli[@]}" audit --lang "$lang" --threshold 1 "$dir" || status=1
    else
        "${cli[@]}" audit --lang "$lang" "$dir"
    fi
    echo "== $lang: survey unhandled"
    "${cli[@]}" survey --lang "$lang" "$dir" | sed -n '/# unhandled/,$p'
done
if [ "$strict" = 1 ] && [ "$checked" = 0 ]; then
    echo "check.sh: --strict, but no corpus to check; fetch one first (tools/corpus/README.md)" >&2
    status=1
fi
exit $status
