// SPDX-License-Identifier: MIT
pragma solidity ^0.8.30;

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
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

interface IBurnable is IERC20 {
    function burnFrom(address account, uint256 amount) external;
    function allowance(address owner, address spender) external view returns (uint256);
}

/**
 * @title SNRGSwap
 * @author DevPup
 * @notice Token swap contract that burns old tokens for migration to new token
 * @dev Allows users to burn tokens and receive a receipt (recorded burn amount) for claiming new tokens via merkle proof
 */
contract SNRGSwap is Ownable2Step, ReentrancyGuard, Pausable {
    /// @notice The SNRG token to be burned
    IBurnable public immutable SNRG;
    
    /// @notice Whether the burn period has been finalized
    bool public finalized;
    
    /// @notice Merkle root for verifying claims in new token contract
    bytes32 public merkleRoot;

    /// @notice Mapping of burned amounts per user address
    mapping(address => uint256) public burned;
    
    event Burned(address indexed user, uint256 amount);
    event Finalized(bytes32 merkleRoot);

    error AlreadyFinalizedError();
    error ZeroAddress();
    error ZeroAmount();
    error AlreadyFinalized();
    error ZeroMerkleRoot();

    /**
     * @notice Constructor
     * @dev Initializes swap contract with immutable SNRG token address
     * @param _SNRG SNRG token address to be burned
     * @param owner_ Owner address for administrative functions
     */
    constructor(address _SNRG, address owner_) Ownable(owner_) {
        if (_SNRG == address(0)) revert ZeroAddress();
        if (owner_ == address(0)) revert ZeroAddress();
        SNRG = IBurnable(_SNRG);
    }

    /**
     * @notice Burn tokens to receive migration receipt
     * @dev FIX L-05: Removed redundant allowance check - burnFrom will revert if insufficient
     * @param amount Amount of tokens to burn
     */
    function burnForReceipt(uint256 amount) external nonReentrant whenNotPaused {
        if (finalized) revert AlreadyFinalizedError();
        if (amount == 0) revert ZeroAmount();
        
        // FIX L-05: Removed redundant allowance check
        // The burnFrom call will revert if allowance is insufficient
        // This saves gas and simplifies the code
        
        // Update state before external call to prevent reentrancy
        burned[msg.sender] += amount;
        SNRG.burnFrom(msg.sender, amount);
        
        emit Burned(msg.sender, amount);
    }

    /**
     * @notice Finalize the burn period and set merkle root for claims
     * @dev Only owner can finalize, can only be called once
     * @param _merkleRoot Merkle root for verifying new token claims
     */
    function finalize(bytes32 _merkleRoot) external onlyOwner {
        if (finalized) revert AlreadyFinalized();
        if (_merkleRoot == bytes32(0)) revert ZeroMerkleRoot();
        
        finalized = true;
        merkleRoot = _merkleRoot;
        
        emit Finalized(_merkleRoot);
    }
    
    /**
     * @notice Get burned amount for a user
     * @dev View function to check how many tokens a user has burned
     * @param user Address to query
     * @return uint256 Amount of tokens burned by user
     */
    function getBurnedAmount(address user) external view returns (uint256) {
        return burned[user];
    }
    
    /**
     * @notice Pause the contract
     * @dev Only owner can pause, prevents burning
     */
    function pause() external onlyOwner {
        _pause();
    }
    
    /**
     * @notice Unpause the contract
     * @dev Only owner can unpause, allows burning
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}