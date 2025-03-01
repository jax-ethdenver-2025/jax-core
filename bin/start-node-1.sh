#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545
NODE1_REMOTE_PORT=8080
NODE1_ENDPOINT_PORT=3001

# Test account (from Anvil's default accounts)
NODE1_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

# Exit if anvil is not running
if ! nc -z localhost "$ANVIL_PORT" >/dev/null 2>&1; then
  echo "Anvil is not running on port $ANVIL_PORT. Exiting."
  exit 1
fi

# Clean up existing data
rm -rf ./data
mkdir -p ./data

# this should source the factory address
source .env.local

export JAX_CONFIG_PATH=./data/jax-node1

alias jax="cargo run --bin jax -- "

cargo run --bin jax -- init \
  --overwrite \
  --factory-address $FACTORY_ADDRESS \
  --eth-signer $NODE1_KEY \
  --http-port $NODE1_REMOTE_PORT \
  --iroh-port $NODE1_ENDPOINT_PORT

cargo run --bin jax -- node
