#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
ENV_FILE=".env.local"

# Exit if anvil is not running
if ! nc -z localhost "$ANVIL_PORT" >/dev/null 2>&1; then
    echo "Anvil is not running on port $ANVIL_PORT. Exiting."
    exit 1
fi

# Deploy contracts
cd contracts
forge script script/Factory.s.sol:FactoryScript --fork-url http://localhost:$ANVIL_PORT --broadcast --private-key $PRIVATE_KEY -vvv

# Read the latest run JSON file which contains the deployment information
LATEST_RUN=$(ls -t broadcast/Factory.s.sol/31337/run-latest.json)

# Extract contract addresses from the JSON file
FACTORY_ADDRESS=$(jq -r '.transactions[] | select(.contractName == "Factory") | .contractAddress' "$LATEST_RUN")

# Check if the address is found and set it, otherwise exit with an error
if [ -z "$FACTORY_ADDRESS" ] || [ "$FACTORY_ADDRESS" == "null" ]; then
    echo "Error: No contract with contractName 'Factory' found."
    exit 1
fi

# Update or create .env.local with the new addresses
cd ..
touch $ENV_FILE

echo "Deployed contracts:"
echo "Factory: $FACTORY_ADDRESS"

# Append new variables
echo "FACTORY_ADDRESS=$FACTORY_ADDRESS" >$ENV_FILE

# Kill anvil if we started it
if [ "$KILL_ANVIL" = true ]; then
    echo "Stopping anvil (PID: $ANVIL_PID)..."
    kill $ANVIL_PID
fi

echo "Deployment complete!"
echo "Factory: $FACTORY_ADDRESS"
