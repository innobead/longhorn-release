fix:
    cargo clippy --fix --allow-dirty --allow-staged

fmt:
    cargo fmt --all

build: fix fmt
    cargo build

release: fix fmt
    cargo build --release

test: build
    cargo test

clean:
    cargo clean
    rm -rf ./.release
