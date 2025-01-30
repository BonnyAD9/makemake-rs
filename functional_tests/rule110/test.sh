#!/usr/bin/sh

cd "$(dirname "$0")"

makemake=../../target/release/makemake

$makemake -py -c test -d template
$makemake test -d res
$makemake -r test

if diff expected res/file; then
    echo success
    rm -r res
    exit 0
else
    echo failure
    rm -r res
    exit 1
fi
