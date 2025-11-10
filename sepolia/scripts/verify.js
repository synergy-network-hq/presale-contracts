import { run } from "hardhat";
import "dotenv/config";
import fs from "fs";

async function main() {
  console.log("🔍 Starting contract verification on Sepolia Etherscan...\n");

  // Check if Etherscan API key is set
  if (!process.env.ETHERSCAN_API_KEY) {
    console.error("❌ ETHERSCAN_API_KEY not found in .env file");
    console.error("Please get an API key from https://etherscan.io/myapikey");
    console.error("and add it to your .env file as ETHERSCAN_API_KEY=your_key_here\n");
    process.exit(1);
  }

  // Read deployment info
  if (!fs.existsSync("deployment-sepolia.json")) {
    console.error("❌ deployment-sepolia.json not found");
    console.error("Please deploy contracts first using: npm run deploy\n");
    process.exit(1);
  }

  const deploymentInfo = JSON.parse(fs.readFileSync("deployment-sepolia.json", "utf8"));
  const { contracts, deployer, treasury, signer } = deploymentInfo;

  console.log("📋 Verifying contracts deployed by:", deployer);
  console.log("   Treasury:", treasury);
  console.log("   Signer:", signer);
  console.log("");

  // Verify each contract with constructor arguments
  const verifications = [
    {
      name: "SNRGToken",
      address: contracts.SNRGToken,
      constructorArguments: [treasury],
    },
    {
      name: "SelfRescueRegistry",
      address: contracts.SelfRescueRegistry,
      constructorArguments: [deployer, contracts.SNRGToken],
    },
    {
      name: "SNRGStaking",
      address: contracts.SNRGStaking,
      constructorArguments: [treasury, contracts.SNRGToken, deployer],
    },
    {
      name: "SNRGSwap",
      address: contracts.SNRGSwap,
      constructorArguments: [contracts.SNRGToken, deployer],
    },
    {
      name: "SNRGPresale",
      address: contracts.SNRGPresale,
      constructorArguments: [contracts.SNRGToken, treasury, deployer, signer],
    },
  ];

  for (let i = 0; i < verifications.length; i++) {
    const { name, address, constructorArguments } = verifications[i];

    console.log(`${i + 1}️⃣  Verifying ${name}...`);
    console.log(`   Address: ${address}`);

    try {
      await run("verify:verify", {
        address: address,
        constructorArguments: constructorArguments,
      });
      console.log(`   ✅ ${name} verified successfully!`);
    } catch (error) {
      if (error.message.includes("Already Verified")) {
        console.log(`   ℹ️  ${name} is already verified`);
      } else {
        console.error(`   ❌ ${name} verification failed:`);
        console.error(`   ${error.message}`);
      }
    }
    console.log("");

    // Add a small delay between verifications to avoid rate limiting
    if (i < verifications.length - 1) {
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
  }

  console.log("═══════════════════════════════════════════════════════");
  console.log("🎉 VERIFICATION COMPLETE!");
  console.log("═══════════════════════════════════════════════════════");
  console.log("");
  console.log("📝 Verified Contract Links:");
  console.log(`   SNRGToken:           https://sepolia.etherscan.io/address/${contracts.SNRGToken}#code`);
  console.log(`   SelfRescueRegistry:  https://sepolia.etherscan.io/address/${contracts.SelfRescueRegistry}#code`);
  console.log(`   SNRGStaking:         https://sepolia.etherscan.io/address/${contracts.SNRGStaking}#code`);
  console.log(`   SNRGSwap:            https://sepolia.etherscan.io/address/${contracts.SNRGSwap}#code`);
  console.log(`   SNRGPresale:         https://sepolia.etherscan.io/address/${contracts.SNRGPresale}#code`);
  console.log("");
  console.log("💡 The ASCII art in your contracts should now display properly on Etherscan!");
  console.log("═══════════════════════════════════════════════════════");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("❌ Verification failed:");
    console.error(error);
    process.exit(1);
  });
