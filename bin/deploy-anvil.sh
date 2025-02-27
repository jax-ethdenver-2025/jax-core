#!/bin/bash
set -e

# Configuration
ANVIL_PORT=8545
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
ENV_FILE=".env.local"

# Ensure anvil is running
if ! nc -z localhost $ANVIL_PORT; then
    echo "Anvil is not running on port $ANVIL_PORT. Starting anvil..."
    anvil --port $ANVIL_PORT &
    ANVIL_PID=$!
    
    # Give anvil time to start
    sleep 2
    
    echo "Anvil started with PID: $ANVIL_PID"
    KILL_ANVIL=true
else
    echo "Anvil already running on port $ANVIL_PORT"
    KILL_ANVIL=false
fi

# Deploy contracts
cd contracts
forge script script/Factory.s.sol:FactoryScript --fork-url http://localhost:$ANVIL_PORT --broadcast --private-key $PRIVATE_KEY -vvv

# Read the latest run JSON file which contains the deployment information
LATEST_RUN=$(ls -t broadcast/Factory.s.sol/31337/run-latest.json)

# Extract contract addresses from the JSON file
if [ -f "$LATEST_RUN" ]; then
    # The contracts are deployed in this order: RewardPool, JaxToken, Factory
    JAXTOKEN_ADDRESS=$(cat $LATEST_RUN | jq -r '.transactions[0].contractAddress')
    REWARDPOOL_ADDRESS=$(cat $LATEST_RUN | jq -r '.transactions[1].contractAddress')
    FACTORY_ADDRESS=$(cat $LATEST_RUN | jq -r '.transactions[2].contractAddress')
    
else
    echo "Error: Could not find deployment JSON file"
    exit 1
fi

# # Create addresses.json
# echo "{
#   \"Factory\": \"$FACTORY_ADDRESS\",
#   \"JaxToken\": \"$JAXTOKEN_ADDRESS\",
#   \"RewardPool\": \"$REWARDPOOL_ADDRESS\"
# }" > ../addresses.json

# echo "Contract addresses saved to addresses.json:"
# cat ../addresses.json | jq

# Update or create .env.local with the new addresses
cd ..
touch $ENV_FILE

# Append new variables
echo "FACTORY_ADDRESS=$FACTORY_ADDRESS" > $ENV_FILE

# Kill anvil if we started it
if [ "$KILL_ANVIL" = true ]; then
    echo "Stopping anvil (PID: $ANVIL_PID)..."
    kill $ANVIL_PID
fi

echo "Deployment complete!"
echo "Factory: $FACTORY_ADDRESS"
