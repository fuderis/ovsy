set -e

echo "Building Ovsy core.."
cargo build --release

echo "Building Music agent.."
cd agents/music && cargo build --release

echo "Installing to /opt/ovsy/"
cd ../..
sudo mkdir -p /opt/ovsy
sudo chown $USER:$USER /opt/ovsy

# core:
cp target/release/ovsy /opt/ovsy/
chmod 755 /opt/ovsy/ovsy

# music:
mkdir -p /opt/ovsy/agents/music/target/release
cp agents/music/Ovsy.toml /opt/ovsy/agents/music/
cp agents/music/target/release/music /opt/ovsy/agents/music/target/release/
chmod 755 /opt/ovsy/agents/music/target/release/music

echo "✅ Installed!"
