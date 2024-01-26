# CHANGELOG

## v1.1.0
- Significantly speed up loading templates
- Unset variables no longer expand to `' '` but `''`
- If now checks whether variable is defined instead of whether the value is
  empty
- Builtin os variables now expand to the name of the os instead of to `'true'`

## v1.1.1
- Prepare for AUR release

## v1.1.0
### New features:
- Internal variables (`_<OS>` to crate cross platform templates e.g. `_LINUX`,
  `_` that is always undefined)
- Set the default prompt answer trough cli (`-p yes|no|ask`,
  `--prompt-answer yes|no|ask`, `-py`, `-pn`, `-pa`)
- Files can be conditionaly ignored by having blank name

### Improvements:
- Error is thrown when user tries to load template that could look like a flag
- `makemake.json` doesn't have to specify the `files` and `vars` fields

### Bug fixes
- Defining multiple variables trough CLI would define only one of the variables

## v1.0.0
Makemake now has all the features that the original makemake had!!
- Conditions in files
- Conditions in filenames

## v0.3.0
- Expressions in file names
- Load template from source
- Load template source to folder

## v0.2.0
- Variables in files
- Literals in files
- Create template from folder
- Load template to folder
- Remove template
- Ignore files

## v0.1.0
The first release!!
- create and load static templates
- list templates
