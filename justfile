fix:
    cargo clippy --fix --allow-dirty --allow-staged

build:
    cargo fmt --all
    cargo build

test:
    cargo test

clean:
    cargo clean
    rm -rf ./.release
