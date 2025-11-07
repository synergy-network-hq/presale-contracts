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
*  /$$$$$$   /$$               /$$       /$$                           
* /$$__  $$ | $$              | $$      |__/                           
*| $$  \__//$$$$$$    /$$$$$$ | $$   /$$ /$$ /$$$$$$$   /$$$$$$        
*|  $$$$$$|_  $$_/   |____  $$| $$  /$$/| $$| $$__  $$ /$$__  $$       
* \____  $$ | $$      /$$$$$$$| $$$$$$/ | $$| $$  \ $$| $$  \ $$       
* /$$  \ $$ | $$ /$$ /$$__  $$| $$_  $$ | $$| $$  | $$| $$  | $$       
*|  $$$$$$/ |  $$$$/|  $$$$$$$| $$ \  $$| $$| $$  | $$|  $$$$$$$       
* \______/   \___/   \_______/|__/  \__/|__/|__/  |__/ \____  $$       
*                                                      /$$  \ $$       
*                                                     |  $$$$$$/       
*                                                      \______/        
* */

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

/**
 * @title SNRGStaking
 * @notice Staking contract for SNRG tokens with fixed-term rewards
 * @dev Implements time-locked staking with reward distribution
 */
contract SNRGStaking is Ownable2Step, ReentrancyGuard, Pausable {
    using SafeERC20 for IERC20;
    
    /// @notice SNRG token contract
    IERC20 public immutable SNRG;
    
    /// @notice Treasury address
    address public immutable TREASURY;
    
    /// @notice Whether contract has been funded
    bool public isFunded;
    
    /// @notice Total reward reserves available for distribution (FIX H-03: tracks unspent rewards)
    uint256 public rewardReserve;
    
    /// @notice Total promised rewards (obligations)
    uint256 public promisedRewards;
    
    /// @notice Individual stake information
    struct Stake {
        uint256 amount;
        uint256 reward;
        uint256 endTime;
        bool withdrawn;
    }

    /// @notice Reward rates by duration (in days) to basis points
    mapping(uint64 => uint256) public rewardRates;
    
    /// @notice User stakes
    mapping(address => Stake[]) public userStakes;
    
    /// @notice Early withdrawal fee (5%)
    uint256 public constant EARLY_WITHDRAWAL_FEE_BPS = 500;
    
    /// @notice Emergency withdrawal fee (10%)
    uint256 public constant EMERGENCY_FEE_BPS = 1000;

    event Staked(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 reward, uint256 endTime);
    event Withdrawn(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 reward);
    event WithdrawnEarly(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 fee);
    event EmergencyWithdrawal(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 fee);
    event ContractFunded(uint256 amount);
    event ReserveToppedUp(uint256 amount);
    event InsufficientReserves(uint256 required, uint256 available);

    error ZeroAddress();
    error AlreadyFunded();
    error NotFunded();
    error ZeroAmount();
    error InvalidDuration();
    error InvalidIndex();
    error AlreadyWithdrawn();
    error StakeNotMatured();
    error StakeMatured();
    error FeeExceedsAmount();
    error InsufficientBalance();
    error InsufficientReservesError();

    /**
     * @notice Constructor
     * @param _TREASURY Treasury address
     * @param _SNRG SNRG token address
     * @param owner_ Owner address
     */
    constructor(address _TREASURY, address _SNRG, address owner_) Ownable(owner_) {
        if (_TREASURY == address(0)) revert ZeroAddress();
        if (_SNRG == address(0)) revert ZeroAddress();
        if (owner_ == address(0)) revert ZeroAddress();
        
        TREASURY = _TREASURY;
        SNRG = IERC20(_SNRG);

        rewardRates[30] = 125;   // 1.25%
        rewardRates[60] = 250;   // 2.50%
        rewardRates[90] = 375;   // 3.75%
        rewardRates[180] = 500;  // 5.00%
    }

    /**
     * @notice Fund the contract with SNRG for rewards
     * @param amount Amount to fund
     */
    function fundContract(uint256 amount) external onlyOwner nonReentrant {
        if (isFunded) revert AlreadyFunded();
        if (amount == 0) revert ZeroAmount();
        
        // Check treasury has sufficient balance
        if (SNRG.balanceOf(TREASURY) < amount) {
            revert InsufficientBalance();
        }
        
        isFunded = true;
        SNRG.safeTransferFrom(TREASURY, address(this), amount);
        rewardReserve = amount;
        
        emit ContractFunded(amount);
    }
    
    /**
     * @notice Top up reward reserves
     * @dev FIX H-03: Allows owner to add more rewards after initial funding
     * @param amount Amount to add to reserves
     */
    function topUpReserves(uint256 amount) external onlyOwner nonReentrant {
        if (amount == 0) revert ZeroAmount();
        
        // Check treasury has sufficient balance
        if (SNRG.balanceOf(TREASURY) < amount) {
            revert InsufficientBalance();
        }
        
        SNRG.safeTransferFrom(TREASURY, address(this), amount);
        // FIX H-03: Increase reserve counter to track unspent rewards
        rewardReserve += amount;
        
        emit ReserveToppedUp(amount);
    }
    
    /**
     * @notice Stake SNRG tokens
     * @param amount Amount to stake
     * @param duration Duration in days
     */
    function stake(uint256 amount, uint64 duration) external nonReentrant whenNotPaused {
        if (!isFunded) revert NotFunded();
        if (amount == 0) revert ZeroAmount();
        
        uint256 rewardBps = rewardRates[duration];
        if (rewardBps == 0) revert InvalidDuration();

        SNRG.safeTransferFrom(msg.sender, address(this), amount);

        uint256 reward = (amount * rewardBps) / 10000;
        
        // FIX H-03: Check if we have sufficient reserves for this reward
        if (rewardReserve < promisedRewards + reward) {
            emit InsufficientReserves(promisedRewards + reward, rewardReserve);
            revert InsufficientReservesError();
        }
        
        // Note: block.timestamp can be manipulated by miners within ~15 minutes
        // This is acceptable for staking periods as the variance is minimal
        // Check for overflow in duration calculation
        if (duration > type(uint64).max / 1 days) revert InvalidDuration();
        uint256 endTime = block.timestamp + (duration * 1 days);

        uint256 stakeIndex = userStakes[msg.sender].length;
        userStakes[msg.sender].push(Stake({
            amount: amount,
            reward: reward,
            endTime: endTime,
            withdrawn: false
        }));

        // Update promised rewards
        promisedRewards += reward;

        emit Staked(msg.sender, stakeIndex, amount, reward, endTime);
    }

    /**
     * @notice Withdraw matured stake with rewards
     * @dev FIX H-03: Decreases rewardReserve when rewards are actually paid out
     * @param stakeIndex Index of the stake
     */
    function withdraw(uint256 stakeIndex) external nonReentrant {
        if (stakeIndex >= userStakes[msg.sender].length) revert InvalidIndex();
        
        Stake storage s = userStakes[msg.sender][stakeIndex];
        if (s.withdrawn) revert AlreadyWithdrawn();
        if (block.timestamp < s.endTime) revert StakeNotMatured();

        s.withdrawn = true;
        uint256 totalPayout = s.amount + s.reward;
        
        // FIX H-03: Decrease promised rewards AND rewardReserve when rewards are paid
        promisedRewards -= s.reward;
        rewardReserve -= s.reward;  // Track actual reward payout
        
        SNRG.safeTransfer(msg.sender, totalPayout);
        emit Withdrawn(msg.sender, stakeIndex, s.amount, s.reward);
    }

    /**
     * @notice Withdraw stake early with penalty
     * @dev FIX H-03: Rewards are forfeited (not paid), so rewardReserve is NOT decreased
     * @param stakeIndex Index of the stake
     */
    function withdrawEarly(uint256 stakeIndex) external nonReentrant {
        if (stakeIndex >= userStakes[msg.sender].length) revert InvalidIndex();
        
        Stake storage s = userStakes[msg.sender][stakeIndex];
        if (s.withdrawn) revert AlreadyWithdrawn();
        if (block.timestamp >= s.endTime) revert StakeMatured();

        s.withdrawn = true;
        
        // FIX H-03: Update promised rewards (early withdrawal forfeits rewards)
        // Do NOT decrease rewardReserve since rewards weren't paid out
        promisedRewards -= s.reward;
        
        uint256 fee = (s.amount * EARLY_WITHDRAWAL_FEE_BPS) / 10000;
        if (fee >= s.amount) revert FeeExceedsAmount();
        uint256 returnAmount = s.amount - fee;
        
        SNRG.safeTransfer(TREASURY, fee);
        SNRG.safeTransfer(msg.sender, returnAmount);
        
        emit WithdrawnEarly(msg.sender, stakeIndex, returnAmount, fee);
    }
    
    /**
     * @notice Emergency withdraw with higher penalty
     * @dev FIX H-03: Rewards are forfeited (not paid), so rewardReserve is NOT decreased
     * @param stakeIndex Index of the stake
     */
    function emergencyWithdraw(uint256 stakeIndex) external nonReentrant {
        if (stakeIndex >= userStakes[msg.sender].length) revert InvalidIndex();
        
        Stake storage s = userStakes[msg.sender][stakeIndex];
        if (s.withdrawn) revert AlreadyWithdrawn();

        s.withdrawn = true;
        
        // FIX H-03: Update promised rewards (emergency withdrawal forfeits rewards)
        // Do NOT decrease rewardReserve since rewards weren't paid out
        promisedRewards -= s.reward;
        
        uint256 fee = (s.amount * EMERGENCY_FEE_BPS) / 10000;
        if (fee >= s.amount) revert FeeExceedsAmount();
        uint256 returnAmount = s.amount - fee;
        
        SNRG.safeTransfer(TREASURY, fee);
        SNRG.safeTransfer(msg.sender, returnAmount);
        
        emit EmergencyWithdrawal(msg.sender, stakeIndex, returnAmount, fee);
    }

    /**
     * @notice Get stake count for user
     * @param user User address
     * @return Number of stakes
     */
    function getStakeCount(address user) external view returns (uint256) {
        return userStakes[user].length;
    }
    
    /**
     * @notice Check if contract has sufficient reserves for all promised rewards
     * @dev FIX H-03: View function to verify solvency
     * @return bool True if reserves are sufficient
     */
    function isSolvent() external view returns (bool) {
        return rewardReserve >= promisedRewards;
    }
    
    /**
     * @notice Get reserve accounting information
     * @dev FIX H-03: Added view for monitoring reserve status
     * @return rewardReserve_ Current unspent reward reserves
     * @return promisedRewards_ Total promised rewards
     * @return availableRewards_ Available rewards (reserves - promised)
     */
    function getReserveInfo() external view returns (uint256 rewardReserve_, uint256 promisedRewards_, uint256 availableRewards_) {
        rewardReserve_ = rewardReserve;
        promisedRewards_ = promisedRewards;
        availableRewards_ = rewardReserve > promisedRewards ? rewardReserve - promisedRewards : 0;
    }

    /**
     * @notice Get stake details
     * @param user User address
     * @param stakeIndex Stake index
     * @return Stake details
     */
    function getStake(address user, uint256 stakeIndex) external view returns (Stake memory) {
        if (stakeIndex >= userStakes[user].length) revert InvalidIndex();
        return userStakes[user][stakeIndex];
    }
    
    /**
     * @notice Pause the contract
     */
    function pause() external onlyOwner {
        _pause();
    }
    
    /**
     * @notice Unpause the contract
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}