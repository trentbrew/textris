play:
    cargo run --release

dev:
    cargo watch -x "run --release"

test:
    cargo test

lint:
    cargo clippy
