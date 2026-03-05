set -e

echo "Building Ovsy core.."
cargo build --release
cd agents

echo "Building Power agent.."
cd power && cargo build --release

echo "Building Music agent.."
cd ../music && cargo build --release

cd ../../

echo "Installing to /opt/ovsy/"
mkdir -p /opt/ovsy

# copy agents:
install_agent() {
  mkdir -p "/opt/ovsy/agents/$1/target/release"
  cp "agents/$1/Ovsy.toml" "/opt/ovsy/agents/$1/"
  cp "agents/$1/target/release/$1" "/opt/ovsy/agents/$1/target/release/"
}

install_agent music
install_agent power

# copy core:
cp target/release/ovsy /opt/ovsy/

echo "✅ Installed!"
