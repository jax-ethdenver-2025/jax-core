#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545
NODE2_REMOTE_PORT=8081
NODE2_ENDPOINT_PORT=3002

# Test account (from Anvil's default accounts)
NODE2_KEY="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"

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
  --eth-signer $NODE2_KEY \
  --http-port $NODE2_REMOTE_PORT \
  --iroh-port $NODE2_ENDPOINT_PORT

cargo run --bin jax -- node
