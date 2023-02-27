# makemake-rs
Create and load folder templates, now in rust.

This is overhaul of [BonnyAD9/MakeMake](https://github.com/BonnyAD9/MakeMake) in rust. This doesn't have all the features that the original has yet,
and it will not be compatible with both the original templates and the original config file.

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

And more! See help to learn more.

## Comparison to the old makemake
### Features
- [X] Copying files
- [X] Creating directories
- [ ] Variables in files
- [ ] Variables in names
- [ ] Literals in files
- [ ] Literals in names
- [ ] Simple branching in files
- [ ] Simple branching in names

### Improvements
- [X] Speed
- [X] Memory usage
- [X] Doesn't crash 2 times when creating the first template
- [X] Whitespace in expressions has no meaning other than separation (there are no expressions yet :)
- [X] Loading template source (source is the same as the template :)

## Links
- **Author:** [BonnyAD9](https://github.com/BonnyAD9)
- **GitHub repository:** [BonnyAD9/makemake-rs](https://github.com/BonnyAD9/makemake-rs)
- **My website:** [bonnyad9.github.io](https://bonnyad9.github.io/)