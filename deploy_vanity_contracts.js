// ========================
// Synergy Network Vanity Deployment Script
// CREATE2 + Vanity Address Ending in "SNRG"
// Solidity Version: 0.8.30
// Networks: Ethereum, BSC, Polygon
// Explorer Verification: Etherscan, BscScan, PolygonScan
// ========================

const fs = require('fs');
const path = require('path');
const solc = require('solc');
const { ethers } = require('ethers');
require('dotenv').config();

// RPC URLs & Keys
const ETH_RPC = process.env.ETH_RPC_URL;
const BSC_RPC = process.env.BSC_RPC_URL;
const POLYGON_RPC = process.env.POLYGON_RPC_URL;
const PRIVATE_KEY = process.env.PRIVATE_KEY;

// Explorer API Keys
const ETHERSCAN_KEY = process.env.ETHERSCAN_API_KEY;
const BSCSCAN_KEY = process.env.BSCSCAN_API_KEY;
const POLYGONSCAN_KEY = process.env.POLYGONSCAN_API_KEY;

// Contracts to deploy
const CONTRACTS = [
  'SNRGpresale.sol',
  'SNRGselfRescueRegistry.sol',
  'SNRGstaking.sol',
  'SNRGswap.sol',
  'SNRGtoken.sol'
];

// CREATE2 Deployer Address (must be deployed once on each chain)
const CREATE2_DEPLOYER = "0x0000000000FFe8B47B3e2130213B802212439497";

// ===== Load & Compile Solidity Contract =====
function compileContract(fileName) {
  const filePath = path.resolve(__dirname, 'contracts', fileName);
  const source = fs.readFileSync(filePath, 'utf8');

  const input = {
    language: 'Solidity',
    sources: {
      [fileName]: { content: source }
    },
    settings: {
      optimizer: { enabled: true, runs: 200 },
      outputSelection: {
        '*': {
          '*': ['abi', 'evm.bytecode']
        }
      }
    }
  };

  const output = JSON.parse(solc.compile(JSON.stringify(input)));
  const contractName = fileName.replace('.sol', '');

  const compiled = output.contracts[fileName][contractName];
  return {
    abi: compiled.abi,
    bytecode: compiled.evm.bytecode.object
  };
}

// ===== CREATE2 Address Computation =====
function computeCreate2Address(factoryAddress, saltHex, bytecode) {
  const create2Addr = ethers.utils.getCreate2Address(
    factoryAddress,
    saltHex,
    ethers.utils.keccak256(bytecode)
  );
  return create2Addr;
}

// ===== Brute Force Salt for Vanity Suffix =====
async function findVanitySalt(factory, bytecode) {
  console.log("Searching for vanity address ending in 'snrg'...");

  for (let i = 0; i < 50000000; i++) {
    const salt = ethers.utils.hexZeroPad(ethers.utils.hexlify(i), 32);
    const addr = computeCreate2Address(factory, salt, bytecode);

    if (addr.toLowerCase().endsWith('snrg')) {
      console.log(`FOUND VANITY ADDRESS: ${addr}`);
      console.log(`SALT USED: ${salt}`);
      return { salt, address: addr };
    }

    if (i % 100000 === 0) console.log(`Checked ${i} salts...`);
  }

  throw new Error('Vanity address not found within search range');
}

// ===== Deploy via CREATE2 Factory =====
async function deployWithCreate2(providerUrl, bytecode, salt) {
  const provider = new ethers.providers.JsonRpcProvider(providerUrl);
  const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

  const deployer = new ethers.Contract(
    CREATE2_DEPLOYER,
    ["function deploy(bytes32 salt, bytes memory code) public returns (address)"],
    wallet
  );

  const tx = await deployer.deploy(salt, bytecode);
  console.log('Deploy TX Hash:', tx.hash);
  await tx.wait();

  return tx.hash;
}

// ===== Verify on Explorer =====
async function verifyContract(network, address, sourceFile, constructorArgs) {
  const apiKeyMap = {
    ethereum: ETHERSCAN_KEY,
    bsc: BSCSCAN_KEY,
    polygon: POLYGONSCAN_KEY
  };

  const urls = {
    ethereum: 'https://api.etherscan.io/api',
    bsc: 'https://api.bscscan.com/api',
    polygon: 'https://api.polygonscan.com/api'
  };

  const apiKey = apiKeyMap[network];
  const url = urls[network];

  console.log(`Verifying on ${network}...`);

  const params = new URLSearchParams({
    module: 'contract',
    action: 'verifysourcecode',
    apikey: apiKey,
    contractaddress: address,
    sourceCode: fs.readFileSync(path.join('contracts', sourceFile), 'utf8'),
    codeformat: 'solidity-single-file',
    contractname: sourceFile.replace('.sol', ''),
    compilerversion: 'v0.8.30+commit...',
    optimizationUsed: 1,
    runs: 200,
    constructorArguements: constructorArgs || ''
  });

  const response = await fetch(url, { method: 'POST', body: params });
  const text = await response.text();

  console.log('Verify Response:', text);
}

// ===== Main Deployment Wrapper =====
async function main() {
  for (const file of CONTRACTS) {
    console.log("=========================");
    console.log(`DEPLOYING ${file}`);
    console.log("=========================");

    const compiled = compileContract(file);
    const bytecode = '0x' + compiled.bytecode;

    const vanity = await findVanitySalt(CREATE2_DEPLOYER, bytecode);

    console.log("Deploying to Ethereum...");
    await deployWithCreate2(ETH_RPC, bytecode, vanity.salt);

    console.log("Deploying to BSC...");
    await deployWithCreate2(BSC_RPC, bytecode, vanity.salt);

    console.log("Deploying to Polygon...");
    await deployWithCreate2(POLYGON_RPC, bytecode, vanity.salt);

    // Save deployment log
    const logPath = path.join(__dirname, 'deploy_logs.json');
    const logs = fs.existsSync(logPath) ? JSON.parse(fs.readFileSync(logPath)) : {};
    logs[file] = vanity;
    fs.writeFileSync(logPath, JSON.stringify(logs, null, 2));
  }

  console.log("ALL CONTRACTS DEPLOYED SUCCESSFULLY.");
}

main().catch(err => console.error(err));
