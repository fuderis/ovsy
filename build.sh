#!/usr/bin/env bash

set -Eeuo pipefail

###############################################################################
# Configuration
###############################################################################

INSTALL_DIR="/usr/local/bin"

###############################################################################
# Output Style
###############################################################################

NC=$'\033[0m'
BOLD=$'\033[1m'

BLUE=$'\033[34m'
CYAN=$'\033[1;36m'
GREEN=$'\033[1;32m'
RED=$'\033[1;31m'
GRAY=$'\033[0;37m'

section() {
    printf "\n${CYAN}▶${NC} ${BOLD}%s${NC}\n" "$1"
}

info() {
    printf "  ${GRAY}%-12s${NC} %s\n" "$1" "$2"
}

item() {
    printf "  ${GREEN}✓${NC} %s ${GRAY}→${NC} %s\n" "$1" "${BLUE}$2${NC}"
}

success() {
    printf "  ${GREEN}✓${NC} %s\n" "$1"
}

error() {
    printf "  ${RED}✗${NC} %s\n" "$1" >&2
}

die() {
    error "$1"
    exit 1
}

###############################################################################
# Requirements
###############################################################################

section "Checking requirements"

command -v cargo >/dev/null ||
    die "cargo is not installed"

command -v jq >/dev/null ||
    die "jq is not installed"

[[ -f Cargo.toml ]] ||
    die "Cargo.toml not found"

success "Environment ready"


###############################################################################
# Metadata
###############################################################################

section "Inspecting project"

METADATA="$(cargo metadata --format-version=1 --no-deps)"

mapfile -t PACKAGES < <(
    jq -r '
        .packages[]
        | select(any(.targets[]; .kind | any(. == "bin")))
        | .name
    ' <<< "$METADATA"
)

[[ ${#PACKAGES[@]} -gt 0 ]] ||
    die "No binaries found"

declare -A BIN_MAP

while IFS=$'\t' read -r package binary; do
    BIN_MAP["$package"]+="$binary "
done < <(
    jq -r '
        .packages[]
        |
        .name as $package
        |
        .targets[]
        |
        select(.kind | any(. == "bin"))
        |
        [$package, .name]
        |
        @tsv
    ' <<< "$METADATA"
)

TOTAL_BINARIES=0

for package in "${PACKAGES[@]}"; do
    count=$(wc -w <<< "${BIN_MAP[$package]}")
    TOTAL_BINARIES=$((TOTAL_BINARIES + count))
done

if jq -e '.workspace_members | length > 1' <<< "$METADATA" >/dev/null; then
    PROJECT_TYPE="Workspace"
else
    PROJECT_TYPE="Package"
fi

info "Type" "$PROJECT_TYPE"
info "Packages" "${#PACKAGES[@]}"
info "Binaries" "$TOTAL_BINARIES"

printf "\n"

for package in "${PACKAGES[@]}"; do

    printf "  ${BOLD}%s${NC}\n" "$package"

    for binary in ${BIN_MAP[$package]}; do
        printf "    ${GRAY}└─${NC} %s\n" "$binary"
    done

done


###############################################################################
# Build
###############################################################################

section "Building binaries"

BUILD_CMD=(
    cargo
    build
    --release
)

for package in "${PACKAGES[@]}"; do
    BUILD_CMD+=(
        --package
        "$package"
    )
done

BUILD_CMD+=("$@")
"${BUILD_CMD[@]}"

success "Compilation completed"


###############################################################################
# Install
###############################################################################

section "Installing binaries"

if [[ -w "$INSTALL_DIR" ]]; then
    INSTALL=(install)
else
    INSTALL=(sudo install)
fi

for package in "${PACKAGES[@]}"; do
    for binary in ${BIN_MAP[$package]}; do

        SOURCE="target/release/$binary"

        [[ -f "$SOURCE" ]] ||
            die "Binary missing: $SOURCE"


        "${INSTALL[@]}" \
            -m755 \
            "$SOURCE" \
            "$INSTALL_DIR/$binary"


        item "$binary" "$INSTALL_DIR/$binary"
    done
done


###############################################################################
# Completed
###############################################################################

section "Completed"
success "Installation finished"
