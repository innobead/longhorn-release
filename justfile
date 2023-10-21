cargo_opts := env('CARGO_OPTS', '')

# Build
build cargo_cmd_opts='':
    @cargo {{ cargo_opts }} build {{ cargo_cmd_opts }}

# Run renote
renote cargo_cmd_opts='' cmd='help':
    @cargo {{ cargo_opts }} run {{ cargo_cmd_opts }} -- {{ cmd }}

# Run tests
test cargo_cmd_opts='':
    @cargo {{ cargo_opts }} test {{ cargo_cmd_opts }}

# Fix code
fix: fmt
    @cargo {{ cargo_opts }} fix --allow-dirty --allow-staged

# Format & Lint codes
fmt:
    @rustup component add rustfmt clippy
    @cargo {{ cargo_opts }} fmt --all

# Release binaries
release:
    @just build '--release'

# Clean build caches
clean:
    @cargo clean

# Build container image
build-image tag='latest':
    @docker build -t renote:{{ tag }} .

# Run container image
run-image cmd='help' tag='latest':
    docker run --env GITHUB_TOKEN:$GITHUB_TOKEN renote:{{ tag }} {{ cmd }}
