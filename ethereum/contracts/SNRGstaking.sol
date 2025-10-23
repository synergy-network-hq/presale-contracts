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

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract SNRGStaking is Ownable, ReentrancyGuard {
    IERC20 public snrg;
    address public immutable treasury;
    bool public isFunded; // <-- NEW: Flag to ensure it's only funded once

    // ... (Stake struct and mappings remain the same) ...
    
    struct Stake {
        uint256 amount;
        uint256 reward;
        uint256 endTime;
        bool withdrawn;
    }

    mapping(uint64 => uint256) public rewardRates;
    mapping(address => Stake[]) public userStakes;
    
    uint256 public constant EARLY_WITHDRAWAL_FEE_BPS = 500; // 5.0%

    event Staked(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 reward, uint256 endTime);
    event Withdrawn(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 reward);
    event WithdrawnEarly(address indexed user, uint256 indexed stakeIndex, uint256 amount, uint256 fee);
    event ContractFunded(uint256 amount); // <-- NEW: Event for funding

    constructor(address _treasury, address owner_) Ownable(owner_) {
        require(_treasury != address(0), "treasury=0");
        treasury = _treasury;

        rewardRates[30] = 125; // 1.25%
        rewardRates[60] = 250; // 2.50%
        rewardRates[90] = 375; // 3.75%
        rewardRates[180] = 500; // 5.00%
    }

    /**
     * @notice NEW: Pulls the approved reward funds from the treasury.
     * @dev The treasury wallet must have first called `approve()` on the SNRG token contract.
     * @param amount The total amount of SNRG to pull for rewards.
     */
    function fundContract(uint256 amount) external onlyOwner {
        require(!isFunded, "already funded");
        require(amount > 0, "amount=0");
        
        isFunded = true;
        require(snrg.transferFrom(treasury, address(this), amount), "fund transfer failed");
        
        emit ContractFunded(amount);
    }
    
    function stake(uint256 amount, uint64 duration) external nonReentrant {
        require(amount > 0, "amount=0");
        uint256 rewardBps = rewardRates[duration];
        require(rewardBps > 0, "invalid duration");

        require(snrg.transferFrom(msg.sender, address(this), amount), "transferFrom fail");

        uint256 reward = (amount * rewardBps) / 10000;
        uint256 endTime = block.timestamp + (duration * 1 days);

        uint256 stakeIndex = userStakes[msg.sender].length;
        userStakes[msg.sender].push(Stake({
            amount: amount,
            reward: reward,
            endTime: endTime,
            withdrawn: false
        }));

        emit Staked(msg.sender, stakeIndex, amount, reward, endTime);
    }

    function withdraw(uint256 stakeIndex) external nonReentrant {
        Stake storage s = userStakes[msg.sender][stakeIndex];
        
        require(!s.withdrawn, "already withdrawn");
        require(block.timestamp >= s.endTime, "stake not matured");

        s.withdrawn = true;
        uint256 totalPayout = s.amount + s.reward;
        
        require(snrg.transfer(msg.sender, totalPayout), "transfer fail");
        emit Withdrawn(msg.sender, stakeIndex, s.amount, s.reward);
    }

    function withdrawEarly(uint256 stakeIndex) external nonReentrant {
        Stake storage s = userStakes[msg.sender][stakeIndex];
        
        require(!s.withdrawn, "already withdrawn");
        require(block.timestamp < s.endTime, "stake has matured");

        s.withdrawn = true;
        
        uint256 fee = (s.amount * EARLY_WITHDRAWAL_FEE_BPS) / 10000;
        uint256 returnAmount = s.amount - fee;
        
        require(snrg.transfer(treasury, fee), "fee transfer fail");
        require(snrg.transfer(msg.sender, returnAmount), "return transfer fail");
        
        emit WithdrawnEarly(msg.sender, stakeIndex, returnAmount, fee);
    }

    function getStakeCount(address user) external view returns (uint256) {
        return userStakes[user].length;
    }

    function getStake(address user, uint256 stakeIndex) external view returns (Stake memory) {
        return userStakes[user][stakeIndex];
    }

    function setSnrgToken(address _snrg) external onlyOwner {
        require(address(snrg) == address(0), "SNRG token address already set");
        require(_snrg != address(0), "snrg=0");
        snrg = IERC20(_snrg);
    }
}