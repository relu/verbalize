#!/usr/bin/env bash
# Corpus coverage check over the fetched samples in .cache/.
#
# `audit` is a hard gate: it fails when a unit is read as an SI composition
# (`Grad` = `G` + `rad`) that the corpus also uses as an ordinary word.
# `survey` is printed for the record (its unhandled shapes are dominated by
# leading-zero runs, which read digit-by-digit by design).
#
# Fetch first, per language and source; this script does not use the network:
#
#   python3 tools/corpus/fetch.py --lang de --articles 2000
#   python3 tools/corpus/fetch.py --lang de --source opus --opus-corpus ECDC
#   tools/corpus/check.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$root"
cli=(cargo run -q -p verbalize-cli --)

status=0
for lang in de en ro; do
    dir="$here/.cache/$lang"
    if [ ! -d "$dir" ]; then
        echo "skip $lang: no corpus in $dir"
        continue
    fi
    echo "== $lang: audit (must be empty)"
    if ! "${cli[@]}" audit --lang "$lang" --threshold 1 "$dir"; then
        status=1
    fi
    echo "== $lang: survey unhandled"
    "${cli[@]}" survey --lang "$lang" "$dir" | sed -n '/# unhandled/,$p'
done
exit $status
