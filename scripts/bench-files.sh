#!/bin/sh
# bench-files.sh <compiler> <dir> <command> <rounds>: runs `<compiler> <command> <file>` on
# every .envo file under dir, rounds times, and prints the total wall time in ms (the best
# round). Process start-up is included, as it is for anyone running the compiler.
set -u
c=$1; dir=$2; cmd=$3; rounds=$4
files=$(cd "$dir" && find . -name '*.envol' | LC_ALL=C sort)
best=0
r=0
while [ "$r" -lt "$rounds" ]; do
    t0=$(date +%s%N)
    for f in $files; do (cd "$dir" && "$c" "$cmd" "$f") > /dev/null 2>&1; done
    t1=$(date +%s%N)
    ms=$(( (t1 - t0) / 1000000 ))
    if [ "$best" -eq 0 ] || [ "$ms" -lt "$best" ]; then best=$ms; fi
    r=$((r + 1))
done
echo "$best"
