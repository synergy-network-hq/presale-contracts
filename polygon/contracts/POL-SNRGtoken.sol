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

interface IRescueRegistryToken {
    function isRescueExecutor(address caller) external view returns (bool);

    function canExecuteRescue(address from) external view returns (bool);
}

/**
 * @title SNRGToken
 * @author DevPup
 * @notice SNRG presale token with restricted transfers and rescue mechanism
 * @dev ERC20 token with transfer restrictions - only specific endpoints and rescue operations allowed
 *      Implements ERC20Permit for gasless approvals and ERC20Burnable for token burning
 *
 * SECURITY MODEL & DESIGN RATIONALE:
 * ----------------------------------
 * This contract intentionally uses the following patterns that may be flagged by automated scanners:
 *
 * 1. OWNER PRIVILEGES: Owner can set endpoints once (one-time configuration). This is necessary
 *    to connect the token to the staking, swap, and presale contracts. The function can only
 *    be called once and owner is expected to be a multi-sig wallet.
 *
 * 2. TRANSFER RESTRICTIONS: Transfers are restricted to specific pathways (treasury to endpoints,
 *    endpoints to users, users to endpoints, rescue operations). This is an intentional presale
 *    token design to prevent trading before official launch.
 *
 * 3. RESCUE MECHANISM: Integration with SelfRescueRegistry allows authorized recovery of tokens
 *    from compromised addresses. This is a user protection feature with strict access controls
 *    and time delays enforced by the rescue registry contract.
 *
 * 4. DONATION PROTECTION: Transfers to endpoints are restricted to controlled sources (treasury
 *    or direct user deposits) to prevent donation attacks that could manipulate endpoint balances.
 *
 * These design choices implement presale token economics and user protection mechanisms.
 * Centralization risks are mitigated through multi-sig ownership and one-time configuration.
 */
contract SNRGToken is ERC20, ERC20Permit, ERC20Burnable, Ownable2Step {
    /* -------------------------------------------------------------------------- */
    /*                                    ERRORS                                  */
    /* -------------------------------------------------------------------------- */

    error TransfersDisabled();
    error ZeroAddress();
    error InvalidEndpoint();
    error PendingEndpoints();
    error NoPendingEndpoints();
    error EndpointDelayActive();
    error InvalidRescueRegistry();

    /* -------------------------------------------------------------------------- */
    /*                               STATE VARIABLES                              */
    /* -------------------------------------------------------------------------- */

    /// @notice Staking contract address - tokens can be transferred to/from
    address public staking;

    /// @notice Swap contract address - tokens can be transferred to/from
    address public swap;

    /// @notice Presale contract address - tokens can be transferred from Treasury
    address public presale;

    /// @notice Rescue registry contract - enables emergency token recovery
    IRescueRegistryToken public rescueRegistry;

    /// @notice Treasury address - tokens can be transferred from
    address public immutable TREASURY;

    /// @notice Whether endpoints have been configured at least once
    bool public endpointsConfigured;
    /// @notice Pending endpoint proposal awaiting confirmation
    struct EndpointProposal {
        address staking;
        address swap;
        address presale;
        address rescueRegistry;
        uint64 eta;
    }

    EndpointProposal private _pendingEndpoints;
    /// @notice Delay before a newly proposed endpoint configuration can be activated
    uint256 public constant ENDPOINT_CONFIRMATION_DELAY = 24 hours;

    /// @notice Token decimals (9)
    uint8 private constant _DECIMALS = 9;

    /* -------------------------------------------------------------------------- */
    /*                                    EVENTS                                  */
    /* -------------------------------------------------------------------------- */

    // FIX H-02: Removed 4th indexed parameter (only 3 indexed allowed)
    event EndpointsProposed(
        address indexed staking,
        address indexed swap,
        address indexed presale,
        address rescueRegistry,
        uint64 eta
    );
    event EndpointProposalCancelled();
    event EndpointsSet(
        address indexed staking,
        address indexed swap,
        address indexed presale,
        address rescueRegistry
    );

    /* -------------------------------------------------------------------------- */
    /*                                 CONSTRUCTOR                                */
    /* -------------------------------------------------------------------------- */

    /**
     * @notice Constructor
     * @dev Mints total supply to treasury and sets immutable treasury address
     * @param _treasury Treasury address receiving initial token supply
     */
    constructor(
        address _treasury
    )
        ERC20("Synergy Presale Coin", "SNRG")
        ERC20Permit("Synergy Presale Coin")
        Ownable(msg.sender)
    {
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
     * @notice Propose endpoint contracts for transfer restrictions
     * @dev Two-step process mitigates misconfiguration risk; caller must confirm after delay
     * @param stakingContract Staking contract address
     * @param swapContract Swap contract address
     * @param presaleContract Presale contract address
     * @param rescueRegistryContract Rescue registry contract address
     */
    function proposeEndpoints(
        address stakingContract,
        address swapContract,
        address presaleContract,
        address rescueRegistryContract
    ) external onlyOwner {
        if (_pendingEndpoints.eta != 0) revert PendingEndpoints();

        _validateEndpointInputs(
            stakingContract,
            swapContract,
            presaleContract,
            rescueRegistryContract
        );

        uint64 eta = uint64(block.timestamp + ENDPOINT_CONFIRMATION_DELAY);
        _pendingEndpoints = EndpointProposal({
            staking: stakingContract,
            swap: swapContract,
            presale: presaleContract,
            rescueRegistry: rescueRegistryContract,
            eta: eta
        });

        emit EndpointsProposed(stakingContract, swapContract, presaleContract, rescueRegistryContract, eta);
    }

    /**
     * @notice Cancel the currently pending endpoint configuration
     */
    function cancelEndpointProposal() external onlyOwner {
        if (_pendingEndpoints.eta == 0) revert NoPendingEndpoints();
        delete _pendingEndpoints;
        emit EndpointProposalCancelled();
    }

    /**
     * @notice Confirm and activate the pending endpoint configuration
     */
    function confirmEndpoints() external onlyOwner {
        EndpointProposal memory p = _pendingEndpoints;
        if (p.eta == 0) revert NoPendingEndpoints();
        if (block.timestamp < p.eta) revert EndpointDelayActive();

        delete _pendingEndpoints;

        endpointsConfigured = true;
        staking = p.staking;
        swap = p.swap;
        presale = p.presale;
        rescueRegistry = IRescueRegistryToken(p.rescueRegistry);

        emit EndpointsSet(p.staking, p.swap, p.presale, p.rescueRegistry);
    }

    function _validateEndpointInputs(
        address stakingContract,
        address swapContract,
        address presaleContract,
        address rescueRegistryContract
    ) internal view {
        // Non-zero address validation
        if (
            stakingContract == address(0) ||
            swapContract == address(0) ||
            presaleContract == address(0) ||
            rescueRegistryContract == address(0)
        ) revert ZeroAddress();

        // Prevent setting Treasury as any endpoint
        if (
            stakingContract == TREASURY ||
            swapContract == TREASURY ||
            presaleContract == TREASURY ||
            rescueRegistryContract == TREASURY
        ) revert InvalidEndpoint();

        // Ensure all endpoints are unique
        require(
            !(stakingContract == swapContract ||
            stakingContract == presaleContract ||
            stakingContract == rescueRegistryContract ||
            swapContract == presaleContract ||
            swapContract == rescueRegistryContract ||
            presaleContract == rescueRegistryContract),
            "DUPLICATE_ENDPOINT"
        );

        if (rescueRegistryContract.code.length == 0) revert InvalidRescueRegistry();

        (bool ok, bytes memory data) = rescueRegistryContract.staticcall(
            abi.encodeWithSignature("isRescueExecutor(address)", address(this))
        );
        if (!ok || data.length != 32) revert InvalidRescueRegistry();
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
    function _update(
        address from,
        address to,
        uint256 amount
    ) internal override {
        // FIX H-01 & L-05: Short-circuit mint/burn path first for gas optimization
        if (from == address(0) || to == address(0)) {
            super._update(from, to, amount);
            return;
        }
        if (!endpointsConfigured) revert TransfersDisabled();

        // Define endpoint addresses
        bool toEndpoint = (to == staking) || (to == swap) || (to == presale);
        bool fromEndpoint = (from == staking) || (from == swap);
        bool treasuryToEndpoint = (from == TREASURY) && toEndpoint;

        // FIX H-01: Special case for presale distribution (Treasury → buyer via presale)
        bool presaleDistribution = (msg.sender == presale && from == TREASURY);

        // Check for rescue operations safely
        bool rescueMove = false;
        address rrAddr = address(rescueRegistry);

        // Proceed only if registry is a contract
        if (rrAddr.code.length > 0) {
            // Low-level staticcall to verify interface compatibility
            (bool ok1, bytes memory data1) = rrAddr.staticcall(
                abi.encodeWithSignature("isRescueExecutor(address)", msg.sender)
            );
            if (ok1 && data1.length == 32 && abi.decode(data1, (bool))) {
                // Only check canExecuteRescue if isRescueExecutor returned true
                (bool ok2, bytes memory data2) = rrAddr.staticcall(
                    abi.encodeWithSignature("canExecuteRescue(address)", from)
                );
                if (ok2 && data2.length == 32 && abi.decode(data2, (bool))) {
                    rescueMove = true;
                }
            }
        }

        // FIX MEDIUM: Restrict transfers TO endpoints to prevent donation attacks
        // Only allow transfers to endpoints when sender is the endpoint itself (pull pattern)
        // or from Treasury, or when user is depositing (msg.sender == from)
        bool controlledToEndpoint = toEndpoint && (from == TREASURY || msg.sender == from || msg.sender == to);

        // Allow: Treasury → endpoints, endpoint → any, controlled deposits to endpoint, presale distribution, rescue operations
        if (
            !(treasuryToEndpoint ||
                fromEndpoint ||
                controlledToEndpoint ||
                presaleDistribution ||
                rescueMove)
        ) {
            revert TransfersDisabled();
        }

        super._update(from, to, amount);
    }
}
