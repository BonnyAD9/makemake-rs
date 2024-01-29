cd "$(dirname "$0")"

cd ..
cargo build -r
cd -

./test_parenthesis/test.sh
