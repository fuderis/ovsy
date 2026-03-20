#!/usr/bin/env bash
# Exit on error, undefined variables, and pipe failures
set -euo pipefail

# Installation directory (can be overridden by the first argument):
INSTALL_DIR="${1:-/opt/ovsy}"
BUILD_MODE="release"

echo "🚀 Starting Ovsy build & install..."

# 1. Build Ovsy Core (requires a Rust compiler):
echo "📦 Building Core..."
cargo build --release

# 2. Prepare directory structure:
echo "📂 Preparing installation directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR/agents"

# Install core binary using 'install' to set executable permissions (755):
install -Dm755 "target/release/ovsy" "$INSTALL_DIR/ovsy"

# 3. Iterate through /agents dir:
for agent_dir in agents/*/; do
    # get the directory name without trailing slash:
    agent_name=$(basename "$agent_dir")
    
    # check for the mandatory Ovsy.toml manifest:
    if [ ! -f "${agent_dir}Ovsy.toml" ]; then
        echo "⚠️  Skipping $agent_name: Ovsy.toml not found."
        continue
    fi

    echo "🛠️  Processing agent: $agent_name"

    # determine build logic based on files present in the agent's folder:
    # Rust
    if [ -f "${agent_dir}Cargo.toml" ]; then
        echo "   Detected Rust agent. Compiling..."
        
        # build inside the agent directory:
        (cd "$agent_dir" && cargo build --release)
        
        mkdir -p "$INSTALL_DIR/agents/$agent_name"
        cp "${agent_dir}Ovsy.toml" "$INSTALL_DIR/agents/$agent_name/"
        
        # we look for any binary in target/release and install it as {name}-agent:
        AGENT_BIN_DIR="${agent_dir}target/release"
        
        # 1. try to find {name}-agent:
        if [ -f "$AGENT_BIN_DIR/${agent_name}-agent" ]; then
             install -m755 "$AGENT_BIN_DIR/${agent_name}-agent" "$INSTALL_DIR/agents/$agent_name/${agent_name}-agent"
        # 2. if not found, take the binary named after the folder/package and rename it during install:
        elif [ -f "$AGENT_BIN_DIR/${agent_name}" ]; then
             echo "   Renaming $agent_name to ${agent_name}-agent during installation..."
             install -m755 "$AGENT_BIN_DIR/${agent_name}" "$INSTALL_DIR/agents/$agent_name/${agent_name}-agent"
        else
             echo "❌ Error: Could not find any suitable binary in $AGENT_BIN_DIR"
             exit 1
        fi

    # Python
    elif [ -f "${agent_dir}requirements.txt" ]; then
        echo "   Detected Python agent. Deploying sources..."
        mkdir -p "$INSTALL_DIR/agents/$agent_name"
        cp -r "$agent_dir"* "$INSTALL_DIR/agents/$agent_name/"
        
    # Unknown
    else
        echo "   Unknown agent type for $agent_name. Copying all files..."
        mkdir -p "$INSTALL_DIR/agents/$agent_name"
        cp -r "$agent_dir"* "$INSTALL_DIR/agents/$agent_name/"
    fi
done

echo "✅ All components installed successfully to $INSTALL_DIR"
