import { ethers } from "hardhat";
import "dotenv/config";

async function main() {
  console.log("🚀 Starting Sepolia contract deployment...\n");

  // Get deployer info
  const [deployer] = await ethers.getSigners();
  console.log("📝 Deploying contracts with account:", deployer.address);

  const balance = await ethers.provider.getBalance(deployer.address);
  console.log("💰 Account balance:", ethers.formatEther(balance), "ETH\n");

  // Get addresses from env
  const TREASURY = process.env.TREASURY || deployer.address;
  const SIGNER = process.env.SIGNER || deployer.address;

  console.log("📋 Configuration:");
  console.log("   Treasury:", TREASURY);
  console.log("   Signer:", SIGNER);
  console.log("   Deployer:", deployer.address);
  console.log("");

  // Deploy contracts in the correct order
  const deployedContracts = {};

  // 1. Deploy SNRGToken
  console.log("1️⃣  Deploying SNRGToken...");
  const SNRGToken = await ethers.getContractFactory("SNRGToken");
  const snrgToken = await SNRGToken.deploy(TREASURY);
  await snrgToken.waitForDeployment();
  const snrgTokenAddress = await snrgToken.getAddress();
  deployedContracts.SNRGToken = snrgTokenAddress;
  console.log("   ✅ SNRGToken deployed to:", snrgTokenAddress);
  console.log("");

  // 2. Deploy SelfRescueRegistry
  console.log("2️⃣  Deploying SelfRescueRegistry...");
  const SelfRescueRegistry = await ethers.getContractFactory("SelfRescueRegistry");
  const selfRescueRegistry = await SelfRescueRegistry.deploy(deployer.address, snrgTokenAddress);
  await selfRescueRegistry.waitForDeployment();
  const selfRescueRegistryAddress = await selfRescueRegistry.getAddress();
  deployedContracts.SelfRescueRegistry = selfRescueRegistryAddress;
  console.log("   ✅ SelfRescueRegistry deployed to:", selfRescueRegistryAddress);
  console.log("");

  // 3. Deploy SNRGStaking
  console.log("3️⃣  Deploying SNRGStaking...");
  const SNRGStaking = await ethers.getContractFactory("SNRGStaking");
  const snrgStaking = await SNRGStaking.deploy(TREASURY, snrgTokenAddress, deployer.address);
  await snrgStaking.waitForDeployment();
  const snrgStakingAddress = await snrgStaking.getAddress();
  deployedContracts.SNRGStaking = snrgStakingAddress;
  console.log("   ✅ SNRGStaking deployed to:", snrgStakingAddress);
  console.log("");

  // 4. Deploy SNRGSwap
  console.log("4️⃣  Deploying SNRGSwap...");
  const SNRGSwap = await ethers.getContractFactory("SNRGSwap");
  const snrgSwap = await SNRGSwap.deploy(snrgTokenAddress, deployer.address);
  await snrgSwap.waitForDeployment();
  const snrgSwapAddress = await snrgSwap.getAddress();
  deployedContracts.SNRGSwap = snrgSwapAddress;
  console.log("   ✅ SNRGSwap deployed to:", snrgSwapAddress);
  console.log("");

  // 5. Deploy SNRGPresale
  console.log("5️⃣  Deploying SNRGPresale...");
  const SNRGPresale = await ethers.getContractFactory("SNRGPresale");
  const snrgPresale = await SNRGPresale.deploy(
    snrgTokenAddress,
    TREASURY,
    deployer.address,
    SIGNER
  );
  await snrgPresale.waitForDeployment();
  const snrgPresaleAddress = await snrgPresale.getAddress();
  deployedContracts.SNRGPresale = snrgPresaleAddress;
  console.log("   ✅ SNRGPresale deployed to:", snrgPresaleAddress);
  console.log("");

  // Configure SNRGToken endpoints
  console.log("⚙️  Configuring SNRGToken endpoints...");
  const setEndpointsTx = await snrgToken.setEndpoints(
    snrgStakingAddress,
    snrgSwapAddress,
    snrgPresaleAddress,
    selfRescueRegistryAddress
  );
  await setEndpointsTx.wait();
  console.log("   ✅ Endpoints configured successfully");
  console.log("");

  // Print deployment summary
  console.log("═══════════════════════════════════════════════════════");
  console.log("🎉 DEPLOYMENT COMPLETE!");
  console.log("═══════════════════════════════════════════════════════");
  console.log("");
  console.log("📝 Deployed Contract Addresses:");
  console.log("   SNRGToken:           ", deployedContracts.SNRGToken);
  console.log("   SelfRescueRegistry:  ", deployedContracts.SelfRescueRegistry);
  console.log("   SNRGStaking:         ", deployedContracts.SNRGStaking);
  console.log("   SNRGSwap:            ", deployedContracts.SNRGSwap);
  console.log("   SNRGPresale:         ", deployedContracts.SNRGPresale);
  console.log("");
  console.log("💡 Configuration:");
  console.log("   Treasury:            ", TREASURY);
  console.log("   Signer:              ", SIGNER);
  console.log("");
  console.log("📋 Next Steps:");
  console.log("   1. Verify contracts on Etherscan");
  console.log("   2. Fund SNRGStaking with rewards (if needed)");
  console.log("   3. Set up presale supported tokens (if needed)");
  console.log("   4. Open the presale (if ready)");
  console.log("");
  console.log("═══════════════════════════════════════════════════════");

  // Save deployment info to file
  const fs = await import("fs");
  const deploymentInfo = {
    network: "sepolia",
    chainId: 11155111,
    deployer: deployer.address,
    treasury: TREASURY,
    signer: SIGNER,
    timestamp: new Date().toISOString(),
    contracts: deployedContracts
  };

  fs.writeFileSync(
    "deployment-sepolia.json",
    JSON.stringify(deploymentInfo, null, 2)
  );
  console.log("💾 Deployment info saved to deployment-sepolia.json");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("❌ Deployment failed:");
    console.error(error);
    process.exit(1);
  });
