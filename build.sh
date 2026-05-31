#!/bin/bash

# Configuration:
UNDERLINE_COUNT=40
INSTALL_DIR="$HOME/.ovsy"
PORT=7878
BINARIES=("ovsy-cli" "ovsy-server")

# Colors:
NC='\033[0m'         # no color
BOLD='\033[1m'

OK='\033[1;32m'      # bold green
ERR='\033[1;31m'     # bold light red
LINE='\033[0;90m'    # dark gray

underline() {
  echo -en "${LINE}"
  printf '%.0s─' $(seq 1 $UNDERLINE_COUNT)
  echo -e "${NC}"
}

# File extension:
EXE=""
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    EXE=".exe"
fi

# 1. Search for project root directory (up to 3 levels):
for i in {1..3}; do
    if [ -d ".git" ]; then
        break
    fi
    cd ..
done

if [ ! -d ".git" ]; then
    echo -e "${LIGHT_RED}Error: .git directory not found.${NC}"
    exit 1
fi

# 2. Kill existing processes to release file locks:
echo -n "Cleaning port $PORT... "
if [[ -z "$EXE" ]]; then
    fuser -k $PORT/tcp >/dev/null 2>&1
else
    PIDS=$(netstat -aon | grep ":$PORT" | awk '{print $5}' | sort -u)
    for pid in $PIDS; do
        taskkill //F //PID "$pid" >/dev/null 2>&1
    done
fi
echo -e "${CYAN}Done${NC}"

# 3. Build project using Cargo:
echo "Running cargo build..."
underline

if ! cargo build --release; then
    echo "Error: Cargo build failed."
    exit 1
fi

underline

# 4. Deploying
echo "Deploying binaries..."
mkdir -p "$INSTALL_DIR/agents"
mkdir -p "$INSTALL_DIR/bin"

# 4.1. Deploy binaries:
for bin_name in "${BINARIES[@]}"; do
    SRC="target/release/${bin_name}${EXE}"
    DEST_DIR="$INSTALL_DIR/bin"
    DEST_NAME="${bin_name}${EXE}"
    DEST="$DEST_DIR/$DEST_NAME"
    BACKUP="$DEST.old"

    if [ -f "$SRC" ]; then
        # create backup if destination exists:
        if [ -f "$DEST" ]; then
            rm -f "$BACKUP"
            mv "$DEST" "$BACKUP" || echo -e "${ERR}Warning: backup failed for $bin_name${NC}"
        fi

        # copy new binary and set permissions:
        if cp "$SRC" "$DEST"; then
            [ -z "$EXE" ] && chmod 755 "$DEST"
            echo -e "  [${OK}OK${NC}] ${BOLD}$DEST_NAME${NC} ${OK}→${NC} installed"
            rm -f "$BACKUP"
        else
            echo -e "  [${ERR}FAIL${NC}] ${BOLD}$DEST_NAME${NC} ${OK}→${NC} failed to copy"
        fi
    fi
done

# 4.2. Deploy agents:
if [ -d "agents" ]; then
    for agent_dir in agents/*/ ; do
        agent_name=$(basename "$agent_dir")
        
        # skip files:
        [ -d "$agent_dir" ] || continue
        
        BIN_NAME="${name_agent:-${agent_name}-agent}${EXE}"
        SRC_BIN="target/release/${AGENT_BIN_NAME}"
        DEST_DIR="$INSTALL_DIR/agents/$agent_name"
        
        # check binary file:
        if [[ -f "$SRC_BIN" && -f "$SRC_TOML" ]]; then
            mkdir -p "$DEST_DIR"
            
            if [[ -z "$EXE" ]]; then
                pkill -x "$BIN_NAME" >/dev/null 2>&1
            else
                taskkill //F //IM "$BIN_NAME" >/dev/null 2>&1
            fi
            # --------------------------------------

            DEST_BIN="$DEST_DIR/$BIN_NAME"
            if cp "$SRC_BIN" "$DEST_BIN" ; then
                [ -z "$EXE" ] && chmod 755 "$DEST_BIN"
                echo -e "  [${OK}OK${NC}] ${BOLD}$BIN_NAME${NC} ${OK}→${NC} installed"
            else
                echo -e "  [${ERR}FAIL${NC}] Failed to copy binary for agent: $agent_name"
            fi
        fi
    done
fi

underline
echo -e "${OK}Build successful!${NC}"

# 5. Start server if requested:
if [[ "$1" == "--start" || "$1" == "-s" ]]; then
    echo "Starting server..."
    # execute via the newly installed CLI:
    "$INSTALL_DIR/ovsy-cli${EXE}" start
fi
