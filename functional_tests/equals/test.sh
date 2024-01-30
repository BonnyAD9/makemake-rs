#!/usr/bin/sh

cd "$(dirname "$0")"

makemake=../../target/release/makemake

rm -r res 2>/dev/null

$makemake -py -c test -d template
$makemake test -d res -Da=hello -Db
$makemake -r test

if diff expected res/file; then
    echo success
    exit 0
else
    echo failure
    exit 1
fi
