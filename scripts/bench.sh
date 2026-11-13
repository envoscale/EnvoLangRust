#!/bin/sh
# bench.sh <envoc> <envoc-rs> <EnvoScale checkout> [rounds]
# Joins the compiler's sources ten times into one file (about 240,000 lines), checks both
# compilers print the same for each command, and prints a Markdown table of the best of
# `rounds` wall times.
set -eu
envoc=$1; rs=$2; root=$3; rounds=${4:-5}
out=bench-out
mkdir -p "$out"
big="$out/big.envol"
: > "$big"
for i in 1 2 3 4 5 6 7 8 9 10; do
    for f in $(tr -d '\r' < "$root/compiler/files.txt"); do cat "$root/compiler/$f" >> "$big"; echo >> "$big"; done
done
lines=$(wc -l < "$big" | tr -d ' ')
best() {
    b=0; r=0
    while [ "$r" -lt "$rounds" ]; do
        t0=$(date +%s%N); "$@" > /dev/null; t1=$(date +%s%N)
        ms=$(( (t1 - t0) / 1000000 ))
        if [ "$b" -eq 0 ] || [ "$ms" -lt "$b" ]; then b=$ms; fi
        r=$((r + 1))
    done
    echo "$b"
}
echo "| $lines lines | envoc (Envol) | envoc-rs (Rust) | Rust / Envol |"
echo "|---|---|---|---|"
for cmd in lex parse; do
    "$envoc" "$cmd" "$big" > "$out/a.txt"; "$rs" "$cmd" "$big" > "$out/b.txt"
    cmp -s "$out/a.txt" "$out/b.txt" || { echo "the outputs of $cmd differ" >&2; exit 1; }
    e=$(best "$envoc" "$cmd" "$big"); r=$(best "$rs" "$cmd" "$big")
    echo "| \`$cmd\` | $e ms | $r ms | $(awk "BEGIN { printf \"%.2f\", $r / $e }") |"
done
