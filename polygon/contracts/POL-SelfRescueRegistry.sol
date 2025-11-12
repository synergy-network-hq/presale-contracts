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
*  /$$$$$$            /$$  /$$$$$$      /$$$$$$$                                                   
* /$$__  $$          | $$ /$$__  $$    | $$__  $$                                                  
*| $$  \__/  /$$$$$$ | $$| $$  \__/    | $$  \ $$  /$$$$$$   /$$$$$$$  /$$$$$$$ /$$   /$$  /$$$$$$ 
*|  $$$$$$  /$$__  $$| $$| $$$$ /$$$$$$| $$$$$$$/ /$$__  $$ /$$_____/ /$$_____/| $$  | $$ /$$__  $$
* \____  $$| $$$$$$$$| $$| $$_/|______/| $$__  $$| $$$$$$$$|  $$$$$$ | $$      | $$  | $$| $$$$$$$$
* /$$  \ $$| $$_____/| $$| $$          | $$  \ $$| $$_____/ \____  $$| $$      | $$  | $$| $$_____/
*|  $$$$$$/|  $$$$$$$| $$| $$          | $$  | $$|  $$$$$$$ /$$$$$$$/|  $$$$$$$|  $$$$$$/|  $$$$$$$
* \______/  \_______/|__/|__/          |__/  |__/ \_______/|_______/  \_______/ \______/  \_______/
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
*
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

    /* -------------------------------------------------------------------------- */
    /*                               STATE VARIABLES                              */
    /* -------------------------------------------------------------------------- */

    /// @notice Prevents re-initialization vulnerability (OWASP SCWE-045)
    bool private initialized;

    struct Plan {
        address recovery;
        uint64 delay;
        uint64 eta;
    }

    uint64 public constant MINIMUM_RESCUE_DELAY = 7 days;
    uint256 public constant RESCUE_COOLDOWN = 90 days;

    uint256 public maxRescueAmount;
    address public immutable TOKEN;

    mapping(address => Plan) public plans;
    mapping(address => uint256) public lastRescueTime;
    mapping(address => bool) public isExecutor;

    /* -------------------------------------------------------------------------- */
    /*                                    EVENTS                                  */
    /* -------------------------------------------------------------------------- */

    event PlanRegistered(address indexed user, address indexed recovery, uint64 delay);
    event RescueInitiated(address indexed user, uint64 eta);
    event RescueCanceled(address indexed user);
    event RescueExecuted(address indexed user, address indexed recovery, uint256 amount);
    event ExecutorSet(address indexed executor, bool enabled);
    event MaxRescueAmountSet(uint256 amount);
    event Initialized(address indexed initializer);

    /* -------------------------------------------------------------------------- */
    /*                                    ERRORS                                  */
    /* -------------------------------------------------------------------------- */

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
    error AlreadyInitialized();

    /* -------------------------------------------------------------------------- */
    /*                                 CONSTRUCTOR                                */
    /* -------------------------------------------------------------------------- */

    constructor(address owner_, address _TOKEN) Ownable(owner_) {
        if (owner_ == address(0)) revert ZeroAddress();
        if (_TOKEN == address(0)) revert ZeroAddress();

        TOKEN = _TOKEN;
        isExecutor[address(this)] = true;
        emit ExecutorSet(address(this), true);
    }

    /* -------------------------------------------------------------------------- */
    /*                                 INITIALIZER                                */
    /* -------------------------------------------------------------------------- */

    /**
     * @notice One-time initializer to set up configuration for proxy/factory use.
     * @dev Protected by the `initializer` modifier to prevent re-invocation.
     */
    modifier initializer() {
        if (initialized) revert AlreadyInitialized();
        initialized = true;
        _;
        emit Initialized(msg.sender);
    }

    function initialize(address _owner, address _token) external initializer {
        if (_owner == address(0) || _token == address(0)) revert ZeroAddress();
        _transferOwnership(_owner);
        isExecutor[address(this)] = true;
    }

    /* -------------------------------------------------------------------------- */
    /*                              USER  INTERFACE                               */
    /* -------------------------------------------------------------------------- */

    function registerPlan(address recovery, uint64 delay) external whenNotPaused {
        if (recovery == address(0)) revert ZeroAddress();
        if (recovery == msg.sender) revert InvalidRecovery();
        if (delay < MINIMUM_RESCUE_DELAY) revert DelayTooShort();
        if (delay > 365 days) revert DelayTooLong();

        plans[msg.sender] = Plan({recovery: recovery, delay: delay, eta: 0});
        emit PlanRegistered(msg.sender, recovery, delay);
    }

    function initiateRescue() external whenNotPaused {
        Plan storage p = plans[msg.sender];
        if (p.recovery == address(0)) revert NoPlanRegistered();
        if (p.eta != 0) revert RescueAlreadyActive();
        if (block.timestamp < lastRescueTime[msg.sender] + RESCUE_COOLDOWN) revert CooldownActive();

        lastRescueTime[msg.sender] = block.timestamp;
        p.eta = uint64(block.timestamp + p.delay);
        emit RescueInitiated(msg.sender, p.eta);
    }

    function cancelRescue() external whenNotPaused {
        Plan storage p = plans[msg.sender];
        if (p.eta == 0) revert NoActiveRescue();

        p.eta = 0;
        lastRescueTime[msg.sender] = 0;
        emit RescueCanceled(msg.sender);
    }

    function canExecuteRescue(address victim) external view returns (bool) {
        Plan memory p = plans[victim];
        return (p.eta != 0 && block.timestamp >= p.eta);
    }

    function isRescueExecutor(address caller) external view returns (bool) {
        return isExecutor[caller];
    }

    /* -------------------------------------------------------------------------- */
    /*                            OWNER / EXECUTOR OPS                            */
    /* -------------------------------------------------------------------------- */

    function setExecutor(address exec, bool enabled) external onlyOwner {
        if (exec == address(0)) revert ZeroAddress();
        isExecutor[exec] = enabled;
        emit ExecutorSet(exec, enabled);
    }

    function setMaxRescueAmount(uint256 maxAmount) external onlyOwner {
        if (maxAmount == 0) revert ZeroAmount();
        maxRescueAmount = maxAmount;
        emit MaxRescueAmountSet(maxAmount);
    }

    function executeRescue(address victim, uint256 amount)
        external
        whenNotPaused
        nonReentrant
    {
        // Authorization check
        if (!isExecutor[msg.sender] && msg.sender != victim) {
            Plan memory victimPlan = plans[victim];
            if (msg.sender != victimPlan.recovery) revert UnauthorizedCaller();
        }

        if (victim == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();

        Plan memory p = plans[victim];
        if (p.recovery == address(0)) revert NoPlanRegistered();
        if (p.eta == 0 || block.timestamp < p.eta) revert NotMatured();

        if (maxRescueAmount > 0 && amount > maxRescueAmount) revert ExceedsMaxRescue();

        uint256 balance = IERC20(TOKEN).balanceOf(victim);
        if (amount > balance) revert InsufficientBalance();

        uint256 allowance = IERC20(TOKEN).allowance(victim, address(this));
        if (allowance < amount) revert InsufficientAllowance();

        plans[victim].eta = 0; // clear before external call
        bool ok = IRestrictedToken(TOKEN).transferFrom(victim, p.recovery, amount);
        if (!ok) revert TransferFailed();

        emit RescueExecuted(victim, p.recovery, amount);
    }

    /* -------------------------------------------------------------------------- */
    /*                              PAUSE CONTROLS                                */
    /* -------------------------------------------------------------------------- */

    function pause() external onlyOwner {
        _pause();
    }

    function unpause() external onlyOwner {
        _unpause();
    }
}
