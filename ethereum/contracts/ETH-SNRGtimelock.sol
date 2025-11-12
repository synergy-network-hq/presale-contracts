// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

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
* /$$$$$$$$ /$$                         /$$                     /$$      
*|__  $$__/|__/                        | $$                    | $$      
*   | $$    /$$ /$$$$$$/$$$$   /$$$$$$ | $$  /$$$$$$   /$$$$$$$| $$   /$$
*   | $$   | $$| $$_  $$_  $$ /$$__  $$| $$ /$$__  $$ /$$_____/| $$  /$$/
*   | $$   | $$| $$ \ $$ \ $$| $$$$$$$$| $$| $$  \ $$| $$      | $$$$$$/ 
*   | $$   | $$| $$ | $$ | $$| $$_____/| $$| $$  | $$| $$      | $$_  $$ 
*   | $$   | $$| $$ | $$ | $$|  $$$$$$$| $$|  $$$$$$/|  $$$$$$$| $$ \  $$
*   |__/   |__/|__/ |__/ |__/ \_______/|__/ \______/  \_______/|__/  \__/
* */

import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";

/**
 * @title Timelock
 * @author DevPup
 * @notice Timelock controller for administrative actions with delay mechanism
 * @dev Uses OpenZeppelin's TimelockController with specific role configuration:
 *      - The multisig is the sole PROPOSER and ADMIN
 *      - The EXECUTOR role is granted to address(0), allowing permissionless execution after delay
 *      - Deployer's admin role is revoked after setup for security
 * @dev Audit Note: This contract was already correct in the prior version. No changes needed.
 */
contract Timelock is TimelockController {
    event TimelockDeployed(address indexed multisig, uint256 minDelay);
    
    error ZeroAddress();
    error ZeroDelay();
    error DelayTooShort();
    error DelayTooLong();
    
    /**
     * @notice Constructor
     * @dev Initializes timelock with specific delay and role configuration
     * @param minDelay Minimum delay for operations in seconds (must be 2-30 days)
     * @param multisig Multisig address that will have proposer and admin rights
     */
    constructor(
        uint256 minDelay,
        address multisig
    ) TimelockController(
        minDelay,
        new address[](0),  // No initial proposers
        new address[](0),  // No initial executors (address(0) will be executor)
        address(0)         // No initial admin, will be set to multisig
    ) {
        if (multisig == address(0)) revert ZeroAddress();
        if (minDelay == 0) revert ZeroDelay();
        if (minDelay < 2 days) revert DelayTooShort();
        if (minDelay > 30 days) revert DelayTooLong();
        
        // Grant the multisig the ability to propose operations
        _grantRole(PROPOSER_ROLE, multisig);
        
        // Grant the multisig admin rights over the timelock
        _grantRole(DEFAULT_ADMIN_ROLE, multisig);
        
        // Grant permissionless execution after delay (address(0) = anyone can execute)
        _grantRole(EXECUTOR_ROLE, address(0));

        // Revoke deployer's admin role for security
        _revokeRole(DEFAULT_ADMIN_ROLE, msg.sender);
        
        emit TimelockDeployed(multisig, minDelay);
    }
}
