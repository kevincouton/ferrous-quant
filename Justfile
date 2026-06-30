# Ferrous Quant Task Runner
# Requires: cargo install just

# Default: show help
_default:
    @just --list

# Build the entire workspace
build:
    cargo build --workspace

# Build release binaries
build-release:
    cargo build --workspace --release

# Run all tests
test:
    cargo test --workspace

# Run tests with coverage
 coverage:
    cargo llvm-cov --workspace --html

# Run benchmarks
bench:
    cargo bench --workspace

# Run e2e tests (requires IB Gateway)
e2e:
    cargo test --workspace --features e2e

# Check formatting and linting
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Fix formatting and apply clippy suggestions
fix:
    cargo fmt --all
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty

# Generate documentation
docs:
    cargo doc --workspace --no-deps --open

# Clean build artifacts
clean:
    cargo clean

# Start local infrastructure with Podman
podman-up:
    podman-compose -f deploy/compose.yml up -d

# Stop local infrastructure
podman-down:
    podman-compose -f deploy/compose.yml down

# View logs
podman-logs:
    podman-compose -f deploy/compose.yml logs -f

# Build Docker image
podman-build:
    podman build -f deploy/Dockerfile -t ferrous-quant:latest .

# Run the CLI in a container
podman-run-cli *args:
    podman run --rm -it ferrous-quant:latest {{args}}

# Run book chapter examples
book-chapter chapter:
    cargo run --bin ch0{{chapter}}_design_patterns
    @echo "Run with: cargo run --bin ch0{{chapter}}_*"
