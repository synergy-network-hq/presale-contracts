const { ethers } = require("hardhat");

/**
 * This script deploys the Timelock contract on a given EVM network.  It should
 * be executed separately from the main deployment since timelock settings may
 * vary between networks.  The multisig address and minimum delay are
 * configured via environment variables.
 */
async function main() {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying timelock with account:", deployer.address);
  const MIN_DELAY = process.env.MIN_DELAY || 3600; // default 1 hour
  const MULTISIG = process.env.MULTISIG || deployer.address;
  const Timelock = await ethers.getContractFactory("Timelock");
  const timelock = await Timelock.deploy(MIN_DELAY, MULTISIG);
  await timelock.deployed();
  console.log("Timelock deployed to:", timelock.address);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});