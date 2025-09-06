Invoke-Build() {
cargo clippy --all-targets -- -D warnings || exit $?
cargo build -p website-searcher || exit $?
cargo test -p website-searcher --locked || exit $?
cargo test --locked --workspace --exclude website_searcher_core --exclude app || echo "Workspace tests failed (non-blocking)"
cargo audit || exit $?
cargo build -p website-searcher --release || exit $?
cargo build -p app --release || exit $?
}
cargo fmt --all -- --check
if [ $? -eq 0 ]; then
echo "Formatting is correct"
Invoke-Build
else
echo "Formatting is incorrect, fixing..."
cargo fmt --all
Invoke-Build
fi
exit 0