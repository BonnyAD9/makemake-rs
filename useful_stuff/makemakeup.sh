#!/usr/bin/sh

# install script for makemake
# wget -O - https://raw.githubusercontent.com/BonnyAD9/makemake-rs/master/useful_stuff/makemakeup.sh | sh && sudo cp /tmp/makemake/target/release/makemake /usr/bin/makemake && sudo cp /tmp/makemake/useful_stuff/man-page/makemake.7 /usr/share/man/man7/makemake.7

check_exists() {
    if type $1 1>/dev/null 2> /dev/null; then
        true
    else
        echo "Missing the program '$1'."
        exit 1
    fi
}

check_exists wget
check_exists cargo

VERSION=2.1.0
PKG="https://github.com/BonnyAD9/makemake-rs/archive/refs/tags/v$VERSION.tar.gz"

cd /tmp

if wget -O - "$PKG" | tar xz; then
    true
else
    exit 1
fi

mv "makemake-rs-$VERSION" makemake
cd makemake

if cargo build -r; then
    true
else
    exit 1
fi

echo sudo cp /tmp/makemake/target/release/makemake /usr/bin/makemake '&& \'
echo sudo cp /tmp/makemake/useful_stuff/man-page/makemake.7 /usr/share/man/man7/makemake.7
