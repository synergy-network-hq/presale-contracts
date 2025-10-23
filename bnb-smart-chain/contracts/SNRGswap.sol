// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

/* *
*  /$$$$$$                                                             
* /$$__  $$                                                            
*| $$  \__/ /$$   /$$ /$$$$$$$   /$$$$$$   /$$$$$$   /$$$$$$  /$$   /$$
*|  $$$$$$ | $$  | $$| $$__  $$ /$$__  $$ /$$__  $$ /$$__  $$| $$  | $$
* \____  $$| $$  | $$| $$  \ $$| $$$$$$$$| $$  \__/| $$  \ $$| $$  | $$
* /$$  \ $$| $$  | $$| $$  | $$| $$_____/| $$      | $$  | $$| $$  | $$
*|  $$$$$$/|  $$$$$$$| $$  | $$|  $$$$$$$| $$      |  $$$$$$$|  $$$$$$$
* \______/  \____  $$|__/  |__/ \_______/|__/       \____  $$ \____  $$
*           /$$  | $$                               /$$  \ $$ /$$  | $$
*          |  $$$$$$/                              |  $$$$$$/|  $$$$$$/
*           \______/                                \______/  \______/ 
*  /$$$$$$                                                             
* /$$__  $$                                                            
*| $$  \__/ /$$  /$$  /$$  /$$$$$$   /$$$$$$                           
*|  $$$$$$ | $$ | $$ | $$ |____  $$ /$$__  $$                          
* \____  $$| $$ | $$ | $$  /$$$$$$$| $$  \ $$                          
* /$$  \ $$| $$ | $$ | $$ /$$__  $$| $$  | $$                          
*|  $$$$$$/|  $$$$$/$$$$/|  $$$$$$$| $$$$$$$/                          
* \______/  \_____/\___/  \_______/| $$____/                           
*                                  | $$                                
*                                  | $$                                
*                                  |__/                                
* */

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

interface IBurnable is IERC20 {
    function burnFrom(address account, uint256 amount) external;
    function allowance(address owner, address spender) external view returns (uint256);
}

contract SNRGSwap is Ownable, ReentrancyGuard {
    IBurnable public immutable snrg;
    bool public finalized;
    bytes32 public merkleRoot;

    mapping(address => uint256) public burned;
    event Burned(address indexed user, uint256 amount);
    event Finalized(bytes32 merkleRoot);

    constructor(address _snrg, address owner_) Ownable(owner_) {
        require(_snrg != address(0), "snrg=0");
        snrg = IBurnable(_snrg);
    }

    function burnForReceipt(uint256 amount) external nonReentrant {
        require(!finalized, "finalized");
        require(amount > 0, "amount=0");
        require(snrg.allowance(msg.sender, address(this)) >= amount, "approve");
        snrg.burnFrom(msg.sender, amount);
        burned[msg.sender] += amount;
        emit Burned(msg.sender, amount);
    }

    function finalize(bytes32 _merkleRoot) external onlyOwner {
        require(!finalized, "already");
        finalized = true;
        merkleRoot = _merkleRoot;
        emit Finalized(_merkleRoot);
    }
}