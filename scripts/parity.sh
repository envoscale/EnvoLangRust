#!/bin/sh
# parity.sh <envoc> <envoc-rs> <EnvoLang checkout> <command> [--core]
# Runs both compilers' <command> on every .envol file in the checkout (build/ left out)
# and requires the same stdout, stderr and exit code. Prints the differences, then a count.
set -u
envoc=$1; rs=$2; root=$3; cmd=$4; mode=${5:-}
[ -x "$envoc" ] || { echo "parity.sh: no compiler at $envoc" >&2; exit 1; }
t=$(mktemp -d)
same=0; differ=0
for f in $(cd "$root" && find . -name '*.envol' -not -path './build/*' | LC_ALL=C sort); do
    dir=$(dirname "$f"); base=$(basename "$f")
    (cd "$root/$dir" && "$envoc" $mode "$cmd" "$base") > "$t/o1" 2> "$t/e1"; r1=$?
    (cd "$root/$dir" && "$rs" $mode "$cmd" "$base") > "$t/o2" 2> "$t/e2"; r2=$?
    if [ "$r1" = "$r2" ] && cmp -s "$t/o1" "$t/o2" && cmp -s "$t/e1" "$t/e2"; then same=$((same + 1))
    else
        differ=$((differ + 1))
        if [ "$differ" -le 10 ]; then
            echo "DIFF $f (exit $r1 vs $r2)"
            diff "$t/o1" "$t/o2" | head -n 4
            diff "$t/e1" "$t/e2" | head -n 4
        fi
    fi
done
rm -rf "$t"
echo "$cmd $mode: $same same, $differ different"
[ "$same" -gt 0 ] || { echo "parity.sh: no .envol files under $root" >&2; exit 1; }
[ "$differ" -eq 0 ]
