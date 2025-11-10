import hre from "hardhat";
import { ethers } from "ethers";
import "dotenv/config";

async function main() {
  // Get network name - Hardhat sets this when --network is used
  const networkName = 'sepolia'; // We know we're deploying to sepolia
  const networkConfig = hre.config.networks[networkName];
  
  if (!networkConfig) {
    throw new Error(`Network ${networkName} not found in config. Available networks: ${Object.keys(hre.config.networks).join(', ')}`);
  }
  
  // Get RPC URL - prefer env var, then config, then default
  let rpcUrl = process.env.SEPOLIA_RPC_URL;
  if (!rpcUrl && networkConfig.url) {
    rpcUrl = typeof networkConfig.url === 'string' ? networkConfig.url : String(networkConfig.url);
  }
  if (!rpcUrl) {
    rpcUrl = "https://rpc.sepolia.org";
  }
  rpcUrl = String(rpcUrl).trim();
  
  // Get private key from environment (preferred) or config
  let privateKey = process.env.PRIVATE_KEY;
  
  // If not in env, try to get from config (might be an array or function)
  if (!privateKey && networkConfig.accounts) {
    if (Array.isArray(networkConfig.accounts) && networkConfig.accounts.length > 0) {
      privateKey = networkConfig.accounts[0];
    } else if (typeof networkConfig.accounts === 'function') {
      // If accounts is a function, we can't use it directly - need env var
      throw new Error("Accounts function not supported. Please set PRIVATE_KEY in .env file");
    }
  }
  
  if (!privateKey) {
    throw new Error("No private key found. Please set PRIVATE_KEY in .env file");
  }
  
  // Ensure private key is a string
  privateKey = String(privateKey).trim();
  
  // Ensure it has 0x prefix if it doesn't already
  if (!privateKey.startsWith('0x')) {
    privateKey = '0x' + privateKey;
  }
  
  // Create provider and signer
  const provider = new ethers.JsonRpcProvider(rpcUrl);
  const signer = new ethers.Wallet(privateKey, provider);
  
  // Verify connection
  const balance = await provider.getBalance(signer.address);
  console.log("Deploying contracts to Sepolia testnet");
  console.log("Deployer address:", signer.address);
  console.log("Balance:", ethers.formatEther(balance), "ETH");
  console.log("Network:", networkName);

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
