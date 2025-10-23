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
* /$$$$$$$$ /$$                         /$$                     /$$      
*|__  $$__/|__/                        | $$                    | $$      
*   | $$    /$$ /$$$$$$/$$$$   /$$$$$$ | $$  /$$$$$$   /$$$$$$$| $$   /$$
*   | $$   | $$| $$_  $$_  $$ /$$__  $$| $$ /$$__  $$ /$$_____/| $$  /$$/
*   | $$   | $$| $$ \ $$ \ $$| $$$$$$$$| $$| $$  \ $$| $$      | $$$$$$/ 
*   | $$   | $$| $$ | $$ | $$| $$_____/| $$| $$  | $$| $$      | $$_  $$ 
*   | $$   | $$| $$ | $$ | $$|  $$$$$$$| $$|  $$$$$$/|  $$$$$$$| $$ \  $$
*   |__/   |__/|__/ |__/ |__/ \_______/|__/ \______/  \_______/|__/  \__/
* */

import "@openzeppelin/contracts/governance/TimelockController.sol";

/**
 * @title Timelock
 * @dev Uses OpenZeppelin's TimelockController to manage administrative actions.
 * - The multisig is the sole PROPOSER and ADMIN.
 * - The EXECUTOR role is granted to address(0), allowing permissionless execution after the delay.
 */
contract Timelock is TimelockController {
    constructor(
        uint256 minDelay,
        address multisig
    ) TimelockController(minDelay, new address[](1), new address[](0), address(0)) {
        // Grant the multisig the ability to propose proposals
        grantRole(PROPOSER_ROLE, multisig);
        
        // Grant the multisig admin rights over the timelock itself
        grantRole(DEFAULT_ADMIN_ROLE, multisig);

        // Revoke the deployer's admin role, leaving the multisig as the sole admin
        renounceRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }
}