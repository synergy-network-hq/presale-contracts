import hre from "hardhat";
console.log("network:", hre.network);
console.log("network.name:", hre.network?.name);
console.log("config.networks:", Object.keys(hre.config.networks || {}));
