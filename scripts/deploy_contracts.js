const { ethers } = require("hardhat");

/**
 * This deployment script deploys the core Synergy smart contracts to the
 * selected EVM network using Hardhat.  It excludes the Timelock contract,
 * which is deployed separately.  Environment variables or command line
 * arguments should be used to provide the treasury and signer addresses.
 */
async function main() {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying contracts with account:", deployer.address);

  // Configure constructor parameters.  You should replace these values
  // appropriately for each deployment.  The TREASURY address receives the
  // initial token supply and fees.  The SIGNER address authorises presale
  // purchases.  The MULTISIG address will own the timelock (not deployed here).
  const TREASURY = process.env.TREASURY || deployer.address;
  const SIGNER = process.env.SIGNER || deployer.address;

  // Deploy SNRG token
  const Token = await ethers.getContractFactory("SNRGToken");
  const token = await Token.deploy(TREASURY);
  await token.deployed();
  console.log("SNRGToken deployed to:", token.address);

  // Deploy SelfRescueRegistry and set the owner to the deployer
  const RescueRegistry = await ethers.getContractFactory("SelfRescueRegistry");
  const rescue = await RescueRegistry.deploy(deployer.address);
  await rescue.deployed();
  console.log("SelfRescueRegistry deployed to:", rescue.address);

  // Deploy staking contract; pass treasury and owner
  const Staking = await ethers.getContractFactory("SNRGStaking");
  const staking = await Staking.deploy(TREASURY, deployer.address);
  await staking.deployed();
  console.log("SNRGStaking deployed to:", staking.address);

  // Deploy swap contract; pass token address and owner
  const Swap = await ethers.getContractFactory("SNRGSwap");
  const swap = await Swap.deploy(token.address, deployer.address);
  await swap.deployed();
  console.log("SNRGSwap deployed to:", swap.address);

  // Deploy presale contract; pass snrg token, treasury, owner and signer
  const Presale = await ethers.getContractFactory("SNRGPresale");
  const presale = await Presale.deploy(token.address, TREASURY, deployer.address, SIGNER);
  await presale.deployed();
  console.log("SNRGPresale deployed to:", presale.address);

  // Wire contracts together.  Set the staking, swap and rescueRegistry on the token
  const tx1 = await token.setEndpoints(staking.address, swap.address, rescue.address);
  await tx1.wait();
  console.log("Token endpoints set");

  // Set the SNRG token address on the staking and rescue registry
  const tx2 = await staking.setSnrgToken(token.address);
  await tx2.wait();
  console.log("Staking token set");
  const tx3 = await rescue.setToken(token.address);
  await tx3.wait();
  console.log("Rescue registry token set");

  // Add supported payment tokens to the presale if desired.  Example:
  // const USDC_ADDRESS = process.env.USDC_ADDRESS;
  // await presale.setSupportedToken(USDC_ADDRESS, true);

  console.log("Deployment complete.");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});