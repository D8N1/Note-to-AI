#!/bin/bash

echo "Setting up IPFS private swarm..."

# Generate swarm key
ipfs key gen --type=rsa --size=2048 swarm-key

# Initialize private swarm
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["*"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "POST", "GET"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Headers '["Authorization"]'

echo "Swarm setup complete!" 