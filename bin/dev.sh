#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545
NODE1_REMOTE_PORT=8080
NODE1_ENDPOINT_PORT=3001
NODE2_REMOTE_PORT=8081
NODE2_ENDPOINT_PORT=3002

# Test accounts (from Anvil's default accounts)
NODE1_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
NODE2_KEY="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"

# Helper function to wait for service to be ready
wait_for_port() {
    local port=$1
    while ! nc -z localhost $port; do
        sleep 1
    done
}

# Kill existing session if it exists
tmux kill-session -t jax 2>/dev/null || true

# Create new tmux session
tmux new-session -d -s jax -n 'anvil'

# Window 0: Anvil
tmux send-keys 'anvil --port '$ANVIL_PORT C-m
wait_for_port $ANVIL_PORT
./bin/deploy-anvil.sh

# Clean up existing data
rm -rf ./data
mkdir -p ./data

# Initialize nodes first

# this should source the factory address
source .env.local

JAX_CONFIG_PATH=./data/jax-node1 cargo run --bin jax -- init \
    --overwrite \
    --factory-address $FACTORY_ADDRESS \
    --eth-signer $NODE1_KEY \
    --http-port $NODE1_REMOTE_PORT \
    --iroh-port $NODE1_ENDPOINT_PORT

JAX_CONFIG_PATH=./data/jax-node2 cargo run --bin jax -- init \
    --overwrite \
    --factory-address $FACTORY_ADDRESS \
    --eth-signer $NODE2_KEY \
    --http-port $NODE2_REMOTE_PORT \
    --iroh-port $NODE2_ENDPOINT_PORT

# Window 1: Node 1 + Interactive Terminal
tmux new-window -n 'node1'
tmux split-window -h
tmux select-pane -t 0
tmux send-keys 'JAX_CONFIG_PATH=./data/jax-node1 cargo run --bin jax -- node' C-m
tmux select-pane -t 1
tmux send-keys 'export JAX_CONFIG_PATH=./data/jax-node1' C-m
tmux send-keys 'alias jax="JAX_CONFIG_PATH=./data/jax-node1 cargo run --bin jax -- "' C-m

# Window 2: Node 2 + Interactive Terminal
tmux new-window -n 'node2'
tmux split-window -h
tmux select-pane -t 0
tmux send-keys 'JAX_CONFIG_PATH=./data/jax-node2 cargo run --bin jax -- node' C-m
tmux select-pane -t 1
tmux send-keys 'export JAX_CONFIG_PATH=./data/jax-node2' C-m
tmux send-keys 'alias jax="JAX_CONFIG_PATH=./data/jax-node2 cargo run --bin jax -- "' C-m

# Select window 1 (node1) and attach
tmux select-window -t 1
tmux attach-session -t jax
