# Makemake
[![downloads][version-badge]][aur]

![makemake][icon]

Create and load folder templates.

## Usage
Show help:
```
makemake -h
```

Create template named `vscm` from the current directory:
```
makemake -c vscm
```

Load template named `vscm` to the current directory:
```
makemake vscm
```

And much more! See help to learn more.

## Templates
If you want to use or just look at some templates see
[BonnyAD9/makemake-templates][templates]

## How to get it
- From the [AUR][aur]
- Install with this long command:
```sh
wget -nv -O - https://raw.githubusercontent.com/BonnyAD9/makemake-rs/master/useful_stuff/makemakeup.sh | sh && sudo cp /tmp/makemake/target/release/makemake /usr/bin/makemake && sudo cp /tmp/makemake/useful_stuff/man-page/makemake.7 /usr/share/man/man7/makemake.7
```

## Links
- **Author:** [BonnyAD9][author]
- **GitHub repository:** [BonnyAD9/makemake-rs](repo)
- **Package:** [AUR][aur]
- **My website:** [bonnyad9.github.io][my-web]

[icon]: assets/svg/repeat.svg
[templates]: https://github.com/BonnyAD9/makemake-templates
[aur]: https://aur.archlinux.org/packages/makemake
[author]: https://github.com/BonnyAD9
[repo]: https://github.com/BonnyAD9/makemake-rs
[my-web]: https://bonnyad9.github.io/
[version-badge]: https://img.shields.io/aur/version/makemake
