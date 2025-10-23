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
*  /$$$$$$            /$$  /$$$$$$      /$$$$$$$                                                   
* /$$__  $$          | $$ /$$__  $$    | $$__  $$                                                  
*| $$  \__/  /$$$$$$ | $$| $$  \__/    | $$  \ $$  /$$$$$$   /$$$$$$$  /$$$$$$$ /$$   /$$  /$$$$$$ 
*|  $$$$$$  /$$__  $$| $$| $$$$ /$$$$$$| $$$$$$$/ /$$__  $$ /$$_____/ /$$_____/| $$  | $$ /$$__  $$
* \____  $$| $$$$$$$$| $$| $$_/|______/| $$__  $$| $$$$$$$$|  $$$$$$ | $$      | $$  | $$| $$$$$$$$
* /$$  \ $$| $$_____/| $$| $$          | $$  \ $$| $$_____/ \____  $$| $$      | $$  | $$| $$_____/
*|  $$$$$$/|  $$$$$$$| $$| $$          | $$  | $$|  $$$$$$$ /$$$$$$$/|  $$$$$$$|  $$$$$$/|  $$$$$$$
* \______/  \_______/|__/|__/          |__/  |__/ \_______/|_______/  \_______/ \______/  \_______/
*                                                                                                  
*                                                                                                  
*                                                                                                  
* /$$$$$$$                      /$$             /$$                                                
*| $$__  $$                    |__/            | $$                                                
*| $$  \ $$  /$$$$$$   /$$$$$$  /$$  /$$$$$$$ /$$$$$$    /$$$$$$  /$$   /$$                        
*| $$$$$$$/ /$$__  $$ /$$__  $$| $$ /$$_____/|_  $$_/   /$$__  $$| $$  | $$                        
*| $$__  $$| $$$$$$$$| $$  \ $$| $$|  $$$$$$   | $$    | $$  \__/| $$  | $$                        
*| $$  \ $$| $$_____/| $$  | $$| $$ \____  $$  | $$ /$$| $$      | $$  | $$                        
*| $$  | $$|  $$$$$$$|  $$$$$$$| $$ /$$$$$$$/  |  $$$$/| $$      |  $$$$$$$                        
*|__/  |__/ \_______/ \____  $$|__/|_______/    \___/  |__/       \____  $$                        
*                     /$$  \ $$                                   /$$  | $$                        
*                    |  $$$$$$/                                  |  $$$$$$/                        
*                     \______/                                    \______/                         
* */

/**
 * SelfRescueRegistry
 * ------------------
 * - Users opt-in by registering a recovery address and a timelock.
 * - They can **initiate** a rescue which starts the clock.
 * - After the timelock, anyone can call `executeRescue(from)` but funds move **only to the registered recovery**.
 * - Users can **cancel** before the timelock elapses.
 * - No owner seizes funds; no centralized role invokes arbitrary transfers.
 * - Marked as a **rescue executor** for SNRG so restricted transfers allow this move.
 */

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

interface IRestrictedToken is IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

contract SelfRescueRegistry is Ownable, ReentrancyGuard {
    struct Plan {
        address recovery;
        uint64 delay;       // seconds
        uint64 eta;         // when executable (0 = none / canceled)
    }
    
    // MODIFIED: Added constant for clarity
    uint64 public constant MINIMUM_RESCUE_DELAY = 1 days;

    mapping(address => Plan) public plans;
    address public token;
    mapping(address => bool) public isExecutor; // contracts allowed to call token during execute

    event PlanRegistered(address indexed user, address indexed recovery, uint64 delay);
    event RescueInitiated(address indexed user, uint64 eta);
    event RescueCanceled(address indexed user);
    event RescueExecuted(address indexed user, address indexed recovery, uint256 amount);
    event ExecutorSet(address indexed executor, bool enabled);

    constructor(address owner_) Ownable(owner_) {
        isExecutor[address(this)] = true; // registry itself is an executor
        emit ExecutorSet(address(this), true);
    }

    function registerPlan(address recovery, uint64 delay) external {
        require(recovery != address(0), "recovery=0");
        // MODIFIED: Using constant
        require(delay >= MINIMUM_RESCUE_DELAY, "delay too short");
        plans[msg.sender] = Plan({recovery: recovery, delay: delay, eta: 0});
        emit PlanRegistered(msg.sender, recovery, delay);
    }

    function initiateRescue() external {
        Plan storage p = plans[msg.sender];
        require(p.recovery != address(0), "no plan");
        p.eta = uint64(block.timestamp) + p.delay;
        emit RescueInitiated(msg.sender, p.eta);
    }

    function cancelRescue() external {
        Plan storage p = plans[msg.sender];
        require(p.eta != 0, "no active");
        p.eta = 0;
        emit RescueCanceled(msg.sender);
    }

    function canExecuteRescue(address victim) external view returns (bool) {
        Plan memory p = plans[victim];
        return (p.eta != 0 && block.timestamp >= p.eta);
    }

    function isRescueExecutor(address caller) external view returns (bool) {
        return isExecutor[caller];
    }

    function setExecutor(address exec, bool enabled) external onlyOwner {
        isExecutor[exec] = enabled;
        emit ExecutorSet(exec, enabled);
    }

    function setToken(address _token) external onlyOwner {
        require(token == address(0), "Token address already set");
        require(_token != address(0), "token=0");
        token = _token;
    }
    
    /**
     * Executes the rescue by transferring the specified balance to the recovery address.
     * This call is permissionless once matured.
     * MODIFIED: Now accepts an `amount` for flexible rescues.
     */
    function executeRescue(address victim, uint256 amount) external nonReentrant {
        Plan memory p = plans[victim];
        // Ensure a token has been configured
        require(token != address(0), "token=0");
        // Validate that a rescue plan exists and has matured
        require(p.recovery != address(0), "no plan");
        require(p.eta != 0 && block.timestamp >= p.eta, "not matured");
        require(amount > 0, "amount=0");

        // Clear ETA to prevent re-entrancy or repeated calls for the *same* initiation
        plans[victim].eta = 0;

        uint256 balance = IERC20(token).balanceOf(victim);
        require(amount <= balance, "insufficient balance");
        
        // Transfer the specified amount from the victim to the recovery address
        bool ok = IRestrictedToken(token).transferFrom(victim, p.recovery, amount);
        require(ok, "transferFrom fail");

        emit RescueExecuted(victim, p.recovery, amount);
    }
}