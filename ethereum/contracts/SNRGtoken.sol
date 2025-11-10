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

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC20Permit} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import {ERC20Burnable} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

interface IRescueRegistry {
    function isRescueExecutor(address caller) external view returns (bool);
    function canExecuteRescue(address from) external view returns (bool);
}

/**
 * @title SNRGToken
 * @author DevPup
 * @notice SNRG presale token with restricted transfers and rescue mechanism
 * @dev ERC20 token with transfer restrictions - only specific endpoints and rescue operations allowed
 *      Implements ERC20Permit for gasless approvals and ERC20Burnable for token burning
 */
contract SNRGToken is ERC20, ERC20Permit, ERC20Burnable, Ownable2Step {
    error TransfersDisabled();
    error NotAuthorized();
    error EndpointsAlreadySet();
    error ZeroAddress();
    error InvalidEndpoint();

    /// @notice Staking contract address - tokens can be transferred to/from
    address public staking;
    
    /// @notice Swap contract address - tokens can be transferred to/from
    address public swap;
    
    /// @notice Presale contract address - tokens can be transferred from Treasury
    address public presale;
    
    /// @notice Rescue registry contract - enables emergency token recovery
    IRescueRegistry public rescueRegistry;
    
    /// @notice Treasury address - tokens can be transferred from
    address public immutable TREASURY;
    
    /// @notice Whether endpoints have been configured (one-time setup)
    bool private _endpointsSet;

    /// @notice Token decimals (9)
    uint8 private constant _DECIMALS = 9;
    
    // FIX H-02: Removed 4th indexed parameter (only 3 indexed allowed)
    event EndpointsSet(address indexed staking, address indexed swap, address indexed presale, address rescueRegistry);

    /**
     * @notice Constructor
     * @dev Mints total supply to treasury and sets immutable treasury address
     * @param _treasury Treasury address receiving initial token supply
     */
    constructor(
        address _treasury
    ) ERC20("Synergy Presale Coin", "SNRG") ERC20Permit("Synergy Presale Coin") Ownable(msg.sender) {
        if (_treasury == address(0)) revert ZeroAddress();
        TREASURY = _treasury;
        _mint(_treasury, 6_000_000_000 * 10 ** _DECIMALS);
    }

    /**
     * @notice Get token decimals
     * @dev Returns 9 decimals for SNRG token
     * @return uint8 Number of decimals (9)
     */
    function decimals() public pure override returns (uint8) {
        return _DECIMALS;
    }

    /**
     * @notice Set endpoint contracts for transfer restrictions
     * @dev Can only be called once by owner to configure staking, swap, presale, and rescue registry
     * @param stakingContract Staking contract address
     * @param swapContract Swap contract address
     * @param presaleContract Presale contract address
     * @param rescueRegistryContract Rescue registry contract address
     */
    function setEndpoints(
        address stakingContract, 
        address swapContract, 
        address presaleContract,
        address rescueRegistryContract
    ) external onlyOwner {
        if (_endpointsSet) revert EndpointsAlreadySet();
        if (stakingContract == address(0) || swapContract == address(0) || presaleContract == address(0) || rescueRegistryContract == address(0)) {
            revert ZeroAddress();
        }
        
        // Prevent setting treasury as endpoint for security
        if (stakingContract == TREASURY || swapContract == TREASURY || presaleContract == TREASURY || rescueRegistryContract == TREASURY) {
            revert InvalidEndpoint();
        }
        
        _endpointsSet = true;
        staking = stakingContract;
        swap = swapContract;
        presale = presaleContract;
        rescueRegistry = IRescueRegistry(rescueRegistryContract);
        
        emit EndpointsSet(stakingContract, swapContract, presaleContract, rescueRegistryContract);
    }

    /**
     * @notice Internal update function with transfer restrictions
     * @dev Overrides ERC20 _update to implement transfer restrictions
     *      Transfers are only allowed:
     *      - From treasury to endpoints (staking/swap/presale) for distribution
     *      - From endpoints to any address (claims/unstaking/distribution)
     *      - To endpoints from any address (deposits/staking)
     *      - Presale distribution: Treasury → buyer when called by presale contract
     *      - Rescue operations when authorized by rescue registry
     * @param from Sender address
     * @param to Recipient address
     * @param amount Amount to transfer
     */
    function _update(address from, address to, uint256 amount) internal override {
        // FIX H-01 & L-05: Short-circuit mint/burn path first for gas optimization
        if (from == address(0) || to == address(0)) {
            super._update(from, to, amount);
            return;
        }

        // Define endpoint addresses
        bool toEndpoint   = (to == staking) || (to == swap) || (to == presale);
        bool fromEndpoint = (from == staking) || (from == swap);
        bool treasuryToEndpoint = (from == TREASURY) && toEndpoint;

        // FIX H-01: Special case for presale distribution (Treasury → buyer via presale)
        bool presaleDistribution = (msg.sender == presale && from == TREASURY);

        // Check for rescue operations
        bool rescueMove = false;
        // FIX L-05: Cache rescueRegistry address
        IRescueRegistry rr = rescueRegistry;
        if (address(rr) != address(0)) {
            // Use try-catch for safety in case rescue registry reverts
            try rr.isRescueExecutor(msg.sender) returns (bool isExecutor) {
                if (isExecutor) {
                    try rr.canExecuteRescue(from) returns (bool canExecute) {
                        rescueMove = canExecute;
                    } catch {
                        // Silent catch - not a rescue operation
                    }
                }
            } catch {
                // Silent catch - registry call failed
            }
        }

        // Allow: Treasury → endpoints, endpoint → any, any → endpoint, presale distribution, rescue operations
        if (!(treasuryToEndpoint || fromEndpoint || toEndpoint || presaleDistribution || rescueMove)) {
            revert TransfersDisabled();
        }

        super._update(from, to, amount);
    }
}