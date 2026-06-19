#!/bin/bash

# Configuration:
UNDERLINE_COUNT=40
INSTALL_DIR="$HOME/.ovsy"
PORT=7878
BINARIES=("ovsy-core" "ovsy-cli")

# Colors:
NC='\033[0m'
BOLD='\033[1m'

GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
LINE='\033[0;90m'

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
echo -e "${BLUE}==>${NC} Cleaning port ${BLUE}$PORT${NC}... "
if [[ -z "$EXE" ]]; then
    fuser -k $PORT/tcp >/dev/null 2>&1
else
    PIDS=$(netstat -aon | grep ":$PORT" | awk '{print $5}' | sort -u)
    for pid in $PIDS; do
        taskkill //F //PID "$pid" >/dev/null 2>&1
    done
fi

# 3. Build project using Cargo:
echo -e "${BLUE}==>${NC} Running cargo build:"

if ! cargo build --release; then
    echo "Error: Cargo build failed."
    exit 1
fi

# 4. Deploying
underline
echo -e "${BLUE}==>${NC} Deploying binaries and agents:"

mkdir -p "$INSTALL_DIR/agents"
mkdir -p "$INSTALL_DIR/bin"

# 4.1. Deploy core binaries:
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
            mv "$DEST" "$BACKUP" || echo -e "  [${RED}Warning${NC}] backup failed for $bin_name"
        fi

        # copy new binary and set permissions:
        if cp "$SRC" "$DEST"; then
            [ -z "$EXE" ] && chmod 755 "$DEST"
            echo -e "  [${GREEN}OK${NC}] ${BOLD}$DEST_NAME${NC} ${GREEN}→${NC} installed (Core)"
            rm -f "$BACKUP"
        else
            echo -e "  [${RED}FAIL${NC}] ${BOLD}$DEST_NAME${NC} ${RED}→${NC} failed to copy"
        fi
    fi
done

# 4.2. Deploy agents:
if [ -d "agents" ]; then
    for agent_dir in agents/*/ ; do
        [ -d "$agent_dir" ] || continue
        
        agent_name=$(basename "$agent_dir")
        
        BIN_NAME="${agent_name}-agent${EXE}"
        SRC_BIN="target/release/${BIN_NAME}"
        DEST_DIR="$INSTALL_DIR/agents/$agent_name"
        
        if [[ -f "$SRC_BIN" ]]; then
            mkdir -p "$DEST_DIR"
            
            if [[ -z "$EXE" ]]; then
                pkill -x "$BIN_NAME" >/dev/null 2>&1
            else
                taskkill //F //IM "$BIN_NAME" >/dev/null 2>&1
            fi

            DEST_BIN="$DEST_DIR/$BIN_NAME"
            if cp "$SRC_BIN" "$DEST_BIN" ; then
                [ -z "$EXE" ] && chmod 755 "$DEST_BIN"
                echo -e "  [${GREEN}OK${NC}] ${BOLD}$BIN_NAME${NC} ${GREEN}→${NC} installed (Agent)"
            else
                echo -e "  [${RED}FAIL${NC}] ${BOLD}$BIN_NAME${NC} ${RED}→${NC} failed to copy agent"
            fi
        fi
    done
fi

# 5. Register in PATH:
underline
echo -e "${BLUE}==>${NC} Registering in PATH:"

LOCAL_BIN_DIR="$HOME/.local/bin"
if [[ -z "$EXE" ]]; then
    # -------------------------------------------------------------------------
    # UNIX (Linux / macOS)
    # -------------------------------------------------------------------------
    mkdir -p "$LOCAL_BIN_DIR"
    ln -sf "$INSTALL_DIR/bin/ovsy-cli" "$LOCAL_BIN_DIR/ovsy"

    BINARY_PATH_="$INSTALL_DIR/bin/ovsy-cli"
    SYMLINK_PATH_="$LOCAL_BIN_DIR/ovsy"
    BINARY_PATH=$(echo "$BINARY_PATH_" | sed "s|$HOME|~|g")
    SYMLINK_PATH=$(echo "$SYMLINK_PATH_" | sed "s|$HOME|~|g")
    echo -e "  [${GREEN}OK${NC}] ${BOLD}Symlink created${NC} $BINARY_PATH ${GREEN}->${NC} $SYMLINK_PATH"

    if [[ ":$PATH:" != *":$LOCAL_BIN_DIR:"* ]]; then
        echo -e "${RED}Warning: $LOCAL_BIN_DIR is not in your PATH.${NC}"
        
        CURRENT_SHELL=$(basename "$SHELL")
        RC_FILE="$HOME/.bashrc"
        [[ "$CURRENT_SHELL" == "zsh" ]] && RC_FILE="$HOME/.zshrc"
        
        echo -e "To fix this, add the following line to your ${BOLD}$RC_FILE${NC}:"
        echo -e "  ${LINE}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
    fi
else
    # -------------------------------------------------------------------------
    # WINDOWS (Git Bash / MSYS / Cygwin)
    # -------------------------------------------------------------------------
    mkdir -p "$LOCAL_BIN_DIR"
    SHIM_FILE="$LOCAL_BIN_DIR/ovsy"
    
    echo "#!/bin/sh" > "$SHIM_FILE"
    echo "\"$INSTALL_DIR/bin/ovsy-cli.exe\" \"\$@\"" >> "$SHIM_FILE"
    chmod +x "$SHIM_FILE"
    echo -e "  [${GREEN}OK${NC}] Created command shim in $SHIM_FILE"

    WIN_INSTALL_DIR=$(cd "$INSTALL_DIR/bin" && pwd -W 2>/dev/null || cygpath -w "$INSTALL_DIR/bin")
    
    USER_PATH=$(reg query "HKCU\Environment" /v PATH 2>/dev/null | awk -F'    ' '/PATH/{print $4}' | sed 's/\r//')
    
    if [[ ";$USER_PATH;" != *";$WIN_INSTALL_DIR;"* ]]; then
        echo "Adding $WIN_INSTALL_DIR to Windows User PATH..."
        if setx PATH "$USER_PATH;$WIN_INSTALL_DIR" >/dev/null 2>&1; then
            echo -e "  [${GREEN}OK${NC}] Path added to Windows Registry. Restart your terminal to apply."
        else
            echo -e "  [${RED}FAIL${NC}] Failed to update Windows PATH automatically."
        fi
    else
        echo -e "  [${GREEN}OK${NC}] Path already exists in Windows Registry."
    fi
fi

underline
echo -e "${GREEN}Build successful!${NC}"

# 6. Start server if requested:
if [[ "$1" == "--start" || "$1" == "-s" ]]; then
    echo "Starting server..."
    "$INSTALL_DIR/ovsy-cli${EXE}" start
fi
