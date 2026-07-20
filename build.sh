#!/usr/bin/env bash

set -Eeuo pipefail

###############################################################################
# Configuration
###############################################################################

INSTALL_DIR="/usr/local/bin"

###############################################################################
# Colors
###############################################################################

NC=$'\033[0m'
BOLD=$'\033[1m'

BLUE=$'\033[38;5;33m'
GREEN=$'\033[38;5;42m'
RED=$'\033[38;5;196m'
GRAY=$'\033[38;5;245m'

###############################################################################
# Output
###############################################################################

section() {
    printf "\n${BLUE}==>${NC} ${BOLD}%s${NC}\n\n" "$1"
}

field() {
    printf "  ${GRAY}%-12s${NC} %s\n" "$1" "$2"
}

item() {
    printf "  %-18s ${GRAY}→${NC} ${BLUE}%s${NC}\n" "$1" "$2"
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

command -v cargo >/dev/null || die "cargo not found"
command -v jq >/dev/null || die "jq not found"

[[ -f Cargo.toml ]] || die "Cargo.toml not found"

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

[[ ${#PACKAGES[@]} -gt 0 ]] || die "No binaries found"

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

field "Type" "$PROJECT_TYPE"
field "Packages" "${#PACKAGES[@]}"
field "Binaries" "$TOTAL_BINARIES"

printf "\n"

for package in "${PACKAGES[@]}"; do
    printf "  ${BOLD}%s${NC}\n" "$package"

    for binary in ${BIN_MAP[$package]}; do
        printf "    └─ %s\n" "$binary"
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

success "Compilation successful"


###############################################################################
# Install
###############################################################################

section "Installing"

if [[ -w "$INSTALL_DIR" ]]; then
    INSTALL=(install)
else
    INSTALL=(sudo install)
fi


for package in "${PACKAGES[@]}"; do

    for binary in ${BIN_MAP[$package]}; do

        SOURCE="target/release/$binary"

        [[ -f "$SOURCE" ]] ||
            die "Binary not found: $SOURCE"

        "${INSTALL[@]}" \
            -m755 \
            "$SOURCE" \
            "$INSTALL_DIR/$binary"

        item "$binary" "$INSTALL_DIR/$binary"

    done

done


###############################################################################
# Done
###############################################################################

printf "\n${GREEN}${BOLD}Done.${NC}\n"
