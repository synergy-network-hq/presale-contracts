// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

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
* */

/**
 * @title SelfRescueRegistry
 * @author DevPup
 * @notice Allows users to set up self-rescue plans for their TOKENs with timelocks
 * @dev FIX M-01: This is a SELF-RESCUE mechanism that requires user opt-in via allowance.
 *      It is NOT a forced recovery system. Users must approve this contract for the rescue
 *      amount BEFORE a rescue can be executed. If keys are lost or users cannot grant
 *      allowance, rescue cannot occur. This is by design for security and user sovereignty.
 */

import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

interface IRescueRegistry {
    function isRescueExecutor(address caller) external view returns (bool);
    function canExecuteRescue(address from) external view returns (bool);
}

interface IRestrictedToken is IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

contract SelfRescueRegistry is IRescueRegistry, Ownable2Step, ReentrancyGuard, Pausable {
    using SafeERC20 for IERC20;
    
    struct Plan {
        address recovery;
        uint64 delay;
        uint64 eta;
    }
    
    uint64 public constant MINIMUM_RESCUE_DELAY = 7 days;
    uint256 public maxRescueAmount;
    
    mapping(address => Plan) public plans;
    mapping(address => uint256) public lastRescueTime;
    address public immutable TOKEN;
    mapping(address => bool) public isExecutor;
    
    uint256 public constant RESCUE_COOLDOWN = 90 days;

    event PlanRegistered(address indexed user, address indexed recovery, uint64 delay);
    event RescueInitiated(address indexed user, uint64 eta);
    event RescueCanceled(address indexed user);
    event RescueExecuted(address indexed user, address indexed recovery, uint256 amount);
    event ExecutorSet(address indexed executor, bool enabled);
    event MaxRescueAmountSet(uint256 amount);

    error ZeroAddress();
    error InvalidRecovery();
    error DelayTooShort();
    error DelayTooLong();
    error NoPlanRegistered();
    error RescueAlreadyActive();
    error CooldownActive();
    error NoActiveRescue();
    error NotMatured();
    error UnauthorizedCaller();
    error ExceedsMaxRescue();
    error InsufficientBalance();
    error TransferFailed();
    error ZeroAmount();
    error InsufficientAllowance();

    /**
     * @notice Constructor
     * @dev Initializes the contract with owner and TOKEN address
     * @param owner_ The owner address
     * @param _TOKEN The TOKEN address that can be rescued
     */
    constructor(address owner_, address _TOKEN) Ownable(owner_) {
        if (owner_ == address(0)) revert ZeroAddress();
        if (_TOKEN == address(0)) revert ZeroAddress();
        
        TOKEN = _TOKEN;
        isExecutor[address(this)] = true;
        
        emit ExecutorSet(address(this), true);
    }

    /**
     * @notice Register a rescue plan
     * @dev Allows users to set up a recovery address and delay period
     * @param recovery The recovery address that will receive rescued funds
     * @param delay The delay period in seconds before rescue can be executed
     */
    function registerPlan(address recovery, uint64 delay) external whenNotPaused {
        if (recovery == address(0)) revert ZeroAddress();
        if (recovery == msg.sender) revert InvalidRecovery();
        if (delay < MINIMUM_RESCUE_DELAY) revert DelayTooShort();
        if (delay > 365 days) revert DelayTooLong();
        
        plans[msg.sender] = Plan({recovery: recovery, delay: delay, eta: 0});
        emit PlanRegistered(msg.sender, recovery, delay);
    }

    /**
     * @notice Initiate a rescue operation
     * @dev Starts the timelock countdown for rescue execution
     */
    function initiateRescue() external whenNotPaused {
        Plan storage p = plans[msg.sender];
        if (p.recovery == address(0)) revert NoPlanRegistered();
        if (p.eta != 0) revert RescueAlreadyActive();
        // Note: block.timestamp manipulation (~15 min) is acceptable for cooldowns
        if (block.timestamp < lastRescueTime[msg.sender] + RESCUE_COOLDOWN) {
            revert CooldownActive();
        }
        
        lastRescueTime[msg.sender] = block.timestamp;
        p.eta = uint64(block.timestamp) + p.delay;
        emit RescueInitiated(msg.sender, p.eta);
    }

    /**
     * @notice Cancel an active rescue operation
     * @dev FIX L-04: Resets cooldown timer to allow immediate re-initiation after cancel
     * @dev Allows users to cancel their pending rescue
     */
    function cancelRescue() external {
        Plan storage p = plans[msg.sender];
        if (p.eta == 0) revert NoActiveRescue();
        
        p.eta = 0;
        // FIX L-04: Reset cooldown on cancel to allow re-initiation without waiting
        lastRescueTime[msg.sender] = 0;
        
        emit RescueCanceled(msg.sender);
    }

    /**
     * @notice Check if a rescue can be executed for a victim
     * @dev View function to check rescue eligibility
     * @param victim The address to check
     * @return bool True if rescue can be executed
     */
    function canExecuteRescue(address victim) external view returns (bool) {
        Plan memory p = plans[victim];
        return (p.eta != 0 && block.timestamp >= p.eta);
    }

    /**
     * @notice Check if an address is a rescue executor
     * @dev View function for executor status
     * @param caller The address to check
     * @return bool True if the address is an executor
     */
    function isRescueExecutor(address caller) external view returns (bool) {
        return isExecutor[caller];
    }

    /**
     * @notice Set executor status for an address
     * @dev Only owner can set executors
     * @param exec The executor address
     * @param enabled Whether to enable or disable
     */
    function setExecutor(address exec, bool enabled) external onlyOwner {
        if (exec == address(0)) revert ZeroAddress();
        isExecutor[exec] = enabled;
        emit ExecutorSet(exec, enabled);
    }
    
    /**
     * @notice Set the maximum rescue amount
     * @dev Only owner can set the maximum amount per rescue
     * @param maxAmount The maximum amount in TOKEN units
     */
    function setMaxRescueAmount(uint256 maxAmount) external onlyOwner {
        if (maxAmount == 0) revert ZeroAmount();
        maxRescueAmount = maxAmount;
        emit MaxRescueAmountSet(maxAmount);
    }
    
    /**
     * @notice Execute a rescue operation
     * @dev FIX M-01: Transfers TOKENs from victim to their registered recovery address
     *      IMPORTANT: This requires the victim to have approved this contract for at least
     *      the rescue amount. This is a self-rescue mechanism, NOT forced recovery.
     * @param victim The address to rescue from
     * @param amount The amount to rescue
     */
    function executeRescue(address victim, uint256 amount) external nonReentrant whenNotPaused {
        // Check if caller is authorized executor or victim/recovery
        if (!isExecutor[msg.sender] && msg.sender != victim) {
            Plan memory victimPlan = plans[victim];
            if (msg.sender != victimPlan.recovery) {
                revert UnauthorizedCaller();
            }
        }
        // Validate inputs
        if (victim == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        
        Plan memory p = plans[victim];
        
        if (p.recovery == address(0)) revert NoPlanRegistered();
        if (p.eta == 0 || block.timestamp < p.eta) revert NotMatured();
        
        
        if (maxRescueAmount > 0 && amount > maxRescueAmount) {
            revert ExceedsMaxRescue();
        }

        // Validate victim has sufficient balance
        uint256 balance = IERC20(TOKEN).balanceOf(victim);
        if (amount > balance) revert InsufficientBalance();
        
        // FIX M-01: Check if victim has approved this contract for the amount
        // This is the key requirement for self-rescue - user must grant allowance
        uint256 allowance = IERC20(TOKEN).allowance(victim, address(this));
        if (allowance < amount) revert InsufficientAllowance();

        // Clear ETA before external call to prevent reentrancy
        plans[victim].eta = 0;
        
        // Perform the transfer
        bool ok = IRestrictedToken(TOKEN).transferFrom(victim, p.recovery, amount);
        if (!ok) revert TransferFailed();

        emit RescueExecuted(victim, p.recovery, amount);
    }
    
    /**
     * @notice Pause the contract
     * @dev Only owner can pause
     */
    function pause() external onlyOwner {
        _pause();
    }
    
    /**
     * @notice Unpause the contract
     * @dev Only owner can unpause
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}