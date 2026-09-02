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

# Assemble an archive of prebuilt lints for one whisker
package-lints rev whisker:
    #!/usr/bin/env -S bash -euo pipefail
    # Whisker looks for an archive named for the commit it pins and for the
    # tag of the binary asking. `whisker abi` prints that tag, so the name
    # comes from the whisker that will load these libraries rather than from
    # anything written here.
    rev="{{ rev }}"
    whisker="{{ whisker }}"

    # Whisker refuses a library built by another rustc, so the libraries are
    # built with the toolchain this repository pins, which is whisker's.
    cargo build --release --locked --workspace

    name="${rev}-$("${whisker}" abi)"
    rm -rf "dist/${name}"
    mkdir -p "dist/${name}"

    # A publisher ships one library per rule, flat, and whisker unpacks only
    # the regular files at the archive's root.
    shopt -s nullglob
    libraries=(target/release/*.so target/release/*.dylib)
    if [ "${#libraries[@]}" -eq 0 ]; then
        echo "the workspace built no dynamic library" >&2
        exit 1
    fi
    cp "${libraries[@]}" "dist/${name}/"

    # The names go in bare. Whisker unpacks only the entries at the archive's
    # root, and `tar -C dir .` writes them all with a `./` prefix, which is a
    # second path component and so is skipped.
    names=()
    for library in "${libraries[@]}"; do
        names+=("$(basename "${library}")")
    done

    tar -czf "dist/${name}.tar.gz" -C "dist/${name}" "${names[@]}"
    rm -rf "dist/${name}"
    (cd dist && shasum -a 256 "${name}.tar.gz" > "${name}.tar.gz.sha256")

# Print the whisker revision every rule is built against
pin:
    @grep -m1 'rev = ' Cargo.toml | sed 's/.*rev = "\([0-9a-f]*\)".*/\1/'
