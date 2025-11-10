#!/bin/bash

# Exit on error
set -e

echo "🔍 Starting Etherscan verification for Sepolia contracts..."
echo ""

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "❌ .env file not found"
    exit 1
fi

# Check API key
if [ -z "$ETHERSCAN_API_KEY" ]; then
    echo "❌ ETHERSCAN_API_KEY not set in .env file"
    exit 1
fi

# Check deployment file
if [ ! -f "deployment-sepolia.json" ]; then
    echo "❌ deployment-sepolia.json not found"
    exit 1
fi

# Read deployment info
DEPLOYER=$(cat deployment-sepolia.json | grep -o '"deployer": "[^"]*"' | cut -d'"' -f4)
TREASURY=$(cat deployment-sepolia.json | grep -o '"treasury": "[^"]*"' | cut -d'"' -f4)
SIGNER=$(cat deployment-sepolia.json | grep -o '"signer": "[^"]*"' | cut -d'"' -f4)
TOKEN=$(cat deployment-sepolia.json | grep -o '"SNRGToken": "[^"]*"' | cut -d'"' -f4)
RESCUE=$(cat deployment-sepolia.json | grep -o '"SelfRescueRegistry": "[^"]*"' | cut -d'"' -f4)
STAKING=$(cat deployment-sepolia.json | grep -o '"SNRGStaking": "[^"]*"' | cut -d'"' -f4)
SWAP=$(cat deployment-sepolia.json | grep -o '"SNRGSwap": "[^"]*"' | cut -d'"' -f4)
PRESALE=$(cat deployment-sepolia.json | grep -o '"SNRGPresale": "[^"]*"' | cut -d'"' -f4)

echo "📋 Configuration:"
echo "   Deployer: $DEPLOYER"
echo "   Treasury: $TREASURY"
echo "   Signer: $SIGNER"
echo ""

# Function to encode constructor arguments
encode_args() {
    local contract=$1
    shift
    local args="$@"

    # Use cast from foundry if available, otherwise provide instructions
    if command -v cast &> /dev/null; then
        echo $(cast abi-encode "constructor($args)" "$@")
    else
        echo ""
    fi
}

echo "1️⃣  Verifying SNRGToken at $TOKEN"
npx hardhat verify --network sepolia "$TOKEN" "$TREASURY" 2>&1 || echo "   Note: May already be verified or requires manual verification"
echo ""
sleep 3

echo "2️⃣  Verifying SelfRescueRegistry at $RESCUE"
npx hardhat verify --network sepolia "$RESCUE" "$DEPLOYER" "$TOKEN" 2>&1 || echo "   Note: May already be verified or requires manual verification"
echo ""
sleep 3

echo "3️⃣  Verifying SNRGStaking at $STAKING"
npx hardhat verify --network sepolia "$STAKING" "$TREASURY" "$TOKEN" "$DEPLOYER" 2>&1 || echo "   Note: May already be verified or requires manual verification"
echo ""
sleep 3

echo "4️⃣  Verifying SNRGSwap at $SWAP"
npx hardhat verify --network sepolia "$SWAP" "$TOKEN" "$DEPLOYER" 2>&1 || echo "   Note: May already be verified or requires manual verification"
echo ""
sleep 3

echo "5️⃣  Verifying SNRGPresale at $PRESALE"
npx hardhat verify --network sepolia "$PRESALE" "$TOKEN" "$TREASURY" "$DEPLOYER" "$SIGNER" 2>&1 || echo "   Note: May already be verified or requires manual verification"
echo ""

echo "═══════════════════════════════════════════════════════"
echo "🎉 VERIFICATION PROCESS COMPLETE!"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "📝 Check verification status at:"
echo "   SNRGToken:           https://sepolia.etherscan.io/address/$TOKEN#code"
echo "   SelfRescueRegistry:  https://sepolia.etherscan.io/address/$RESCUE#code"
echo "   SNRGStaking:         https://sepolia.etherscan.io/address/$STAKING#code"
echo "   SNRGSwap:            https://sepolia.etherscan.io/address/$SWAP#code"
echo "   SNRGPresale:         https://sepolia.etherscan.io/address/$PRESALE#code"
echo ""
echo "💡 If verification failed, the contracts may already be verified"
echo "   or may require manual verification at https://sepolia.etherscan.io/verifyContract"
echo "═══════════════════════════════════════════════════════"
