#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545
NODE3_REMOTE_PORT=8082
NODE3_ENDPOINT_PORT=3003

# Test account (from Anvil's default accounts)
NODE3_KEY="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"

# Exit if anvil is not running
if ! nc -z localhost "$ANVIL_PORT" >/dev/null 2>&1; then
  echo "Anvil is not running on port $ANVIL_PORT. Exiting."
  exit 1
fi

# # Clean up existing data
rm -rf ./data
mkdir -p ./data

# this should source the factory address
source .env.local

export JAX_CONFIG_PATH=./data/jax-node3

alias jax="cargo run --bin jax -- "

cargo run --bin jax -- init \
  --overwrite \
  --factory-address $FACTORY_ADDRESS \
  --eth-signer $NODE3_KEY \
  --http-port $NODE3_REMOTE_PORT \
  --iroh-port $NODE3_ENDPOINT_PORT

cargo run --bin jax -- node
