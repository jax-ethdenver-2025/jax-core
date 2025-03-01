#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545

# Exit if anvil is not running
if ! nc -z localhost "$ANVIL_PORT" >/dev/null 2>&1; then
  echo "Anvil is not running on port $ANVIL_PORT. Exiting."
  exit 1
fi

# Test account (from Anvil's default accounts)
NODE_KEYS=(
  "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
  "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
  "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
  # todo: add more keys to account for more nodes
)

# get the node index from the command line
NODE_INDEX=$1

NODE_KEY=${NODE_KEYS[$NODE_INDEX]}
NODE_REMOTE_PORT=$((8080 + NODE_INDEX))
NODE_ENDPOINT_PORT=$((3001 + NODE_INDEX))

# check if the overwrite flag is set
if [ "$2" = "--overwrite" ]; then
  OVERWRITE=true
else
  OVERWRITE=false
fi

export JAX_CONFIG_PATH="./data/jax-node$NODE_INDEX"

function init_node() {
 # this should source the factory address
  source .env.local

  # check if the factory address is set
  if [ -z "$FACTORY_ADDRESS" ]; then
    echo "FACTORY_ADDRESS is not set. Exiting."
    exit 1
  fi

  cargo run --bin jax -- init \
    --overwrite \
    --factory-address $FACTORY_ADDRESS \
    --eth-signer $NODE_KEY \
    --http-port $NODE_REMOTE_PORT \
    --iroh-port $NODE_ENDPOINT_PORT 
}

# check if the config path exists
if [ ! -d "$JAX_CONFIG_PATH" ]; then
  init_node
elif [ "$OVERWRITE" = true ]; then
  init_node
fi

alias jax="cargo run --bin jax -- "

cargo run --bin jax -- node
