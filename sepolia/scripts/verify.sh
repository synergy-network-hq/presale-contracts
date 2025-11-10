#!/bin/bash

echo "🔍 Starting contract verification on Sepolia Etherscan..."
echo ""

# Check if deployment file exists
if [ ! -f "deployment-sepolia.json" ]; then
    echo "❌ deployment-sepolia.json not found"
    echo "Please deploy contracts first using: npm run deploy"
    exit 1
fi

# Read deployment info
DEPLOYER=$(jq -r '.deployer' deployment-sepolia.json)
TREASURY=$(jq -r '.treasury' deployment-sepolia.json)
SIGNER=$(jq -r '.signer' deployment-sepolia.json)
TOKEN=$(jq -r '.contracts.SNRGToken' deployment-sepolia.json)
RESCUE=$(jq -r '.contracts.SelfRescueRegistry' deployment-sepolia.json)
STAKING=$(jq -r '.contracts.SNRGStaking' deployment-sepolia.json)
SWAP=$(jq -r '.contracts.SNRGSwap' deployment-sepolia.json)
PRESALE=$(jq -r '.contracts.SNRGPresale' deployment-sepolia.json)

echo "📋 Verifying contracts deployed by: $DEPLOYER"
echo "   Treasury: $TREASURY"
echo "   Signer: $SIGNER"
echo ""

# Verify SNRGToken
echo "1️⃣  Verifying SNRGToken..."
echo "   Address: $TOKEN"
npx hardhat verify --network sepolia $TOKEN "$TREASURY"
echo ""
sleep 3

# Verify SelfRescueRegistry
echo "2️⃣  Verifying SelfRescueRegistry..."
echo "   Address: $RESCUE"
npx hardhat verify --network sepolia $RESCUE "$DEPLOYER" "$TOKEN"
echo ""
sleep 3

# Verify SNRGStaking
echo "3️⃣  Verifying SNRGStaking..."
echo "   Address: $STAKING"
npx hardhat verify --network sepolia $STAKING "$TREASURY" "$TOKEN" "$DEPLOYER"
echo ""
sleep 3

# Verify SNRGSwap
echo "4️⃣  Verifying SNRGSwap..."
echo "   Address: $SWAP"
npx hardhat verify --network sepolia $SWAP "$TOKEN" "$DEPLOYER"
echo ""
sleep 3

# Verify SNRGPresale
echo "5️⃣  Verifying SNRGPresale..."
echo "   Address: $PRESALE"
npx hardhat verify --network sepolia $PRESALE "$TOKEN" "$TREASURY" "$DEPLOYER" "$SIGNER"
echo ""

echo "═══════════════════════════════════════════════════════"
echo "🎉 VERIFICATION COMPLETE!"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "📝 Verified Contract Links:"
echo "   SNRGToken:           https://sepolia.etherscan.io/address/$TOKEN#code"
echo "   SelfRescueRegistry:  https://sepolia.etherscan.io/address/$RESCUE#code"
echo "   SNRGStaking:         https://sepolia.etherscan.io/address/$STAKING#code"
echo "   SNRGSwap:            https://sepolia.etherscan.io/address/$SWAP#code"
echo "   SNRGPresale:         https://sepolia.etherscan.io/address/$PRESALE#code"
echo ""
echo "💡 The ASCII art in your contracts should now display properly on Etherscan!"
echo "═══════════════════════════════════════════════════════"
