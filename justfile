# Run every check that CI runs
pre-commit: format-check lint test

# Format the sources
format:
    cargo fmt

# Check that the sources are formatted
format-check:
    cargo fmt --check

# Lint the sources
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run every rule's tests
test:
    cargo nextest run --workspace --all-features --config-file .config/nextest.toml

# Check this repository with the rules in it
#
# The whisker binary must come from the revision `Cargo.toml` pins, because a
# plugin built against one whisker is refused by any other.
check-self whisker="whisker":
    {{ whisker }} check .

# Print the whisker revision every rule is built against
pin:
    @grep -m1 'rev = ' Cargo.toml | sed 's/.*rev = "\([0-9a-f]*\)".*/\1/'
