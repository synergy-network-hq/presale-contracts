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
* /$$$$$$$                                         /$$                 
*| $$__  $$                                       | $$                 
*| $$  \ $$ /$$$$$$   /$$$$$$   /$$$$$$$  /$$$$$$ | $$  /$$$$$$        
*| $$$$$$$//$$__  $$ /$$__  $$ /$$_____/ |____  $$| $$ /$$__  $$       
*| $$____/| $$  \__/| $$$$$$$$|  $$$$$$   /$$$$$$$| $$| $$$$$$$$       
*| $$     | $$      | $$_____/ \____  $$ /$$__  $$| $$| $$_____/       
*| $$     | $$      |  $$$$$$$ /$$$$$$$/|  $$$$$$$| $$|  $$$$$$$       
*|__/     |__/       \_______/|_______/  \_______/|__/ \_______/       
*                                                                      
*                                                                      
*                                                                      
*  /$$$$$$            /$$                                              
* /$$__  $$          |__/                                              
*| $$  \__/  /$$$$$$  /$$ /$$$$$$$                                     
*| $$       /$$__  $$| $$| $$__  $$                                    
*| $$      | $$  \ $$| $$| $$  \ $$                                    
*| $$    $$| $$  | $$| $$| $$  | $$                                    
*|  $$$$$$/|  $$$$$$/| $$| $$  | $$                                    
* \______/  \______/ |__/|__/  |__/                                    
* */

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

interface IRescueRegistry {
    function isRescueExecutor(address caller) external view returns (bool);
    function canExecuteRescue(address from) external view returns (bool);
}

contract SNRGToken is ERC20, ERC20Permit, ERC20Burnable, Ownable {
    error TransfersDisabled();
    error NotAuthorized();

    address public staking;
    address public swap;
    IRescueRegistry public rescueRegistry;
    address public treasury;

    uint8 private constant _DECIMALS = 9;

    /// @notice Emitted when the staking, swap or rescue registry endpoints are set.
    event EndpointsSet(address indexed staking, address indexed swap, address indexed rescueRegistry);

    // CHANGE: Removed staking, swap, and rescueRegistry from the constructor
    constructor(
        address _treasury
    ) ERC20("Synergy Presale Coin", "SNRG") ERC20Permit("SNRG") Ownable(msg.sender) {
        require(_treasury != address(0), "treasury=0");
        _mint(_treasury, 6_000_000_000 * 10 ** _DECIMALS);
        treasury = _treasury;
    }

    function decimals() public pure override returns (uint8) {
        return _DECIMALS;
    }

    // This function is now used to set the addresses AFTER all contracts are deployed
    function setEndpoints(address _staking, address _swap, address _rescueRegistry) external onlyOwner {
        // Ensure endpoints are not already configured
        require(staking == address(0) && swap == address(0) && address(rescueRegistry) == address(0), "already set");
        // Validate all endpoint addresses to prevent zero-address assignments
        require(_staking != address(0) && _swap != address(0) && _rescueRegistry != address(0), "zero endpoint");
        staking = _staking;
        swap = _swap;
        rescueRegistry = IRescueRegistry(_rescueRegistry);
        emit EndpointsSet(_staking, _swap, _rescueRegistry);
    }

    function _update(address from, address to, uint256 amount) internal override {
        // ... rest of the function remains the same
        bool isMint = from == address(0);
        bool isBurn = to == address(0);
        if (!isMint && !isBurn) {
            bool fromAllowed = (from == staking) || (from == swap) || (from == treasury);
            bool toAllowed   = (to == staking)   || (to == swap);

            bool rescueMove = false;
            if (address(rescueRegistry) != address(0)) {
                if (rescueRegistry.isRescueExecutor(msg.sender) && rescueRegistry.canExecuteRescue(from)) {
                    rescueMove = true;
                }
            }

            if (!(fromAllowed || toAllowed || rescueMove)) {
                revert TransfersDisabled();
            }
        }
        super._update(from, to, amount);
    }
}