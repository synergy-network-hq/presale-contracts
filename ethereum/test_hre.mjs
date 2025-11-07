import hre from "hardhat";
await import("@nomicfoundation/hardhat-ethers");
console.log("After import, hre.ethers:", hre.ethers);
console.log("hre tasks:", Object.keys(hre.tasks));
