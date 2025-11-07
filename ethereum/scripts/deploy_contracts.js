import hre from "hardhat";
import { ethers } from "ethers";
import "dotenv/config";

async function main() {
  // Get network name and config
  const networkName = hre.network.name;
  const networkConfig = hre.config.networks[networkName];
  
  if (!networkConfig) {
    throw new Error(`Network ${networkName} not found in config`);
  }
  
  // Create provider using the RPC URL from config or env
  const rpcUrl = networkConfig.url || process.env.SEPOLIA_RPC_URL || "https://rpc.sepolia.org";
  const provider = new ethers.JsonRpcProvider(rpcUrl);
  
  // Get private key from config accounts array
  const privateKey = networkConfig.accounts && networkConfig.accounts[0] 
    ? networkConfig.accounts[0] 
    : process.env.PRIVATE_KEY;
  
  if (!privateKey) {
    throw new Error("No private key found in config or environment");
  }
  
  // Create signer
  const signer = new ethers.Wallet(privateKey, provider);
  
  console.log("Deploying contracts with account:", signer.address);
  console.log("Network:", networkName);
  console.log("RPC URL:", rpcUrl);

  const TREASURY = process.env.TREASURY || signer.address;
  const SIGNER = process.env.SIGNER || signer.address;

  // Get contract artifacts
  const TokenArtifact = await hre.artifacts.readArtifact("SNRGToken");
  const RescueArtifact = await hre.artifacts.readArtifact("SelfRescueRegistry");
  const StakingArtifact = await hre.artifacts.readArtifact("SNRGStaking");
  const SwapArtifact = await hre.artifacts.readArtifact("SNRGSwap");
  const PresaleArtifact = await hre.artifacts.readArtifact("SNRGPresale");

  console.log("\nDeploying SNRGToken...");
  const TokenFactory = new ethers.ContractFactory(TokenArtifact.abi, TokenArtifact.bytecode, signer);
  const token = await TokenFactory.deploy(TREASURY);
  await token.waitForDeployment();
  const tokenAddress = await token.getAddress();
  console.log("✓ SNRGToken deployed to:", tokenAddress);

  console.log("\nDeploying SelfRescueRegistry...");
  const RescueFactory = new ethers.ContractFactory(RescueArtifact.abi, RescueArtifact.bytecode, signer);
  const rescue = await RescueFactory.deploy(signer.address);
  await rescue.waitForDeployment();
  const rescueAddress = await rescue.getAddress();
  console.log("✓ SelfRescueRegistry deployed to:", rescueAddress);

  console.log("\nDeploying SNRGStaking...");
  const StakingFactory = new ethers.ContractFactory(StakingArtifact.abi, StakingArtifact.bytecode, signer);
  const staking = await StakingFactory.deploy(TREASURY, signer.address);
  await staking.waitForDeployment();
  const stakingAddress = await staking.getAddress();
  console.log("✓ SNRGStaking deployed to:", stakingAddress);

  console.log("\nDeploying SNRGSwap...");
  const SwapFactory = new ethers.ContractFactory(SwapArtifact.abi, SwapArtifact.bytecode, signer);
  const swap = await SwapFactory.deploy(tokenAddress, signer.address);
  await swap.waitForDeployment();
  const swapAddress = await swap.getAddress();
  console.log("✓ SNRGSwap deployed to:", swapAddress);

  console.log("\nDeploying SNRGPresale...");
  const PresaleFactory = new ethers.ContractFactory(PresaleArtifact.abi, PresaleArtifact.bytecode, signer);
  const presale = await PresaleFactory.deploy(tokenAddress, TREASURY, signer.address, SIGNER);
  await presale.waitForDeployment();
  const presaleAddress = await presale.getAddress();
  console.log("✓ SNRGPresale deployed to:", presaleAddress);

  console.log("\nWiring contracts together...");
  const tx1 = await token.setEndpoints(stakingAddress, swapAddress, rescueAddress);
  await tx1.wait();
  console.log("✓ Token endpoints set");

  const tx2 = await staking.setSnrgToken(tokenAddress);
  await tx2.wait();
  console.log("✓ Staking token set");
  
  const tx3 = await rescue.setToken(tokenAddress);
  await tx3.wait();
  console.log("✓ Rescue registry token set");

  console.log("\n" + "=".repeat(60));
  console.log("DEPLOYMENT SUMMARY");
  console.log("=".repeat(60));
  console.log("Network:", networkName);
  console.log("Deployer:", signer.address);
  console.log("SNRGToken:", tokenAddress);
  console.log("SelfRescueRegistry:", rescueAddress);
  console.log("SNRGStaking:", stakingAddress);
  console.log("SNRGSwap:", swapAddress);
  console.log("SNRGPresale:", presaleAddress);
  console.log("=".repeat(60));
  console.log("\n✅ Deployment complete!");
}

main().catch((error) => {
  console.error("\n❌ Deployment failed:");
  console.error(error);
  process.exitCode = 1;
});
