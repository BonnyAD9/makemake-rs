cd "$(dirname "$0")"

cd ..
cargo build -r
cd -

printf 'parenthesis: '
./parenthesis/test.sh
printf 'equals:      '
./equals/test.sh
printf 'null check:  '
./null-check/test.sh
printf 'call:        '
./call/test.sh
