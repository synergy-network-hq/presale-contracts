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
 *  /$$$$$$
 * /$$__  $$
 *| $$  \__/ /$$  /$$  /$$  /$$$$$$   /$$$$$$
 *|  $$$$$$ | $$ | $$ | $$ |____  $$ /$$__  $$
 * \____  $$| $$ | $$ | $$  /$$$$$$$| $$  \ $$
 * /$$  \ $$| $$ | $$ | $$ /$$__  $$| $$  | $$
 *|  $$$$$$/|  $$$$$/$$$$/|  $$$$$$$| $$$$$$$/
 * \______/  \_____/\___/  \_______/| $$____/
 *                                  | $$
 *                                  | $$
 *                                  |__/
 * */

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";

interface IBurnable is IERC20 {
    function burnFrom(address account, uint256 amount) external;

    function allowance(
        address owner,
        address spender
    ) external view returns (uint256);
}

/**
 * @title SNRGSwap
 * @author DevPup
 * @notice Token swap contract that burns old tokens for migration to new token
 * @dev Allows users to burn tokens and receive a receipt (recorded burn amount) for claiming new tokens via merkle proof
 *
 * SECURITY MODEL & DESIGN RATIONALE:
 * ----------------------------------
 * This contract intentionally uses the following patterns that may be flagged by automated scanners:
 *
 * 1. PAUSABLE: Emergency stop mechanism protects users during migration if issues are discovered.
 *    Only owner can pause, and owner should be a multi-signature wallet.
 *
 * 2. OWNER PRIVILEGES: Owner controls finalization and merkle root setting for the migration.
 *    This is necessary for off-chain computation of the merkle tree. Owner is expected to be
 *    a multi-sig wallet or timelock contract to mitigate centralization risks.
 *
 * 3. TIMELOCK PATTERN: 48-hour delay between proposing and finalizing merkle root allows
 *    community verification and provides time to detect errors before finalization.
 *
 * 4. BURN TRACKING: Internal accounting tracks individual burns to prevent double-claiming
 *    in the migration process.
 *
 * These design choices follow OpenZeppelin security best practices and are standard for
 * token migration contracts. Centralization risks are mitigated through timelock and multi-sig.
 */
contract SNRGSwap is Ownable2Step, ReentrancyGuard, Pausable {
    /* -------------------------------------------------------------------------- */
    /*                               STATE VARIABLES                              */
    /* -------------------------------------------------------------------------- */

    /// @notice The SNRG token to be burned
    IBurnable public immutable SNRG;

    /// @notice Whether the burn period has been finalized
    bool public finalized;

    /// @notice Merkle root for verifying claims in new token contract
    bytes32 public merkleRoot;

    /// @notice Mapping of burned amounts per user address
    mapping(address => uint256) public burned;
    /// @notice Tracks aggregate burned amount for sanity checks
    uint256 public totalBurned;

    /* -------------------------------------------------------------------------- */
    /*                                    EVENTS                                  */
    /* -------------------------------------------------------------------------- */

    event Burned(address indexed user, uint256 amount);
    event Finalized(bytes32 merkleRoot);
    event RootProposed(bytes32 indexed root, uint256 timestamp);
    event RootCanceled(bytes32 indexed root);
    event FinalizationReopened(bytes32 indexed previousRoot, bytes32 indexed newRoot, uint256 eta);

    /* -------------------------------------------------------------------------- */
    /*                                    ERRORS                                  */
    /* -------------------------------------------------------------------------- */

    error AlreadyFinalizedError();
    error ZeroAddress();
    error ZeroAmount();
    error AlreadyFinalized();
    error ZeroMerkleRoot();
    error PendingRootExists();
    error NoPendingRoot();
    error NotFinalized();

    /* -------------------------------------------------------------------------- */
    /*                                 CONSTRUCTOR                                */
    /* -------------------------------------------------------------------------- */

    /**
     * @notice Constructor
     * @dev Initializes swap contract with immutable SNRG token address
     * @param _SNRG SNRG token address to be burned
     * @param owner_ Owner address for administrative functions
     */
    constructor(address _SNRG, address owner_) Ownable(owner_) {
        if (_SNRG == address(0)) revert ZeroAddress();
        if (owner_ == address(0)) revert ZeroAddress();
        SNRG = IBurnable(_SNRG);
    }

    /**
     * @notice Burn tokens to receive migration receipt
     * @dev FIX SCWE-090: verifies that the user's balance actually decreased by `amount`
     *      after calling burnFrom(). Prevents inflated receipts from mis-implemented tokens.
     * @param amount Amount of tokens to burn
     */
    function burnForReceipt(
        uint256 amount
    ) external nonReentrant whenNotPaused {
        if (finalized) revert AlreadyFinalizedError();
        if (amount == 0) revert ZeroAmount();

        // Record pre-burn balance
        uint256 balanceBefore = SNRG.balanceOf(msg.sender);

        // Call external burn
        SNRG.burnFrom(msg.sender, amount);

        // Verify that the balance really decreased
        uint256 balanceAfter = SNRG.balanceOf(msg.sender);
        uint256 burnedActual = balanceBefore - balanceAfter;
        require(burnedActual >= amount, "SNRGSwap: under-burn or non-standard token behavior");

        // FIX MEDIUM: Credit only the requested amount, not any excess from fees
        // This prevents over-crediting if burnFrom reduces balance by more than amount
        uint256 creditAmount = amount; // Credit exact requested amount, not burnedActual

        // Update state only after successful verification
        burned[msg.sender] += creditAmount;
        totalBurned += creditAmount;

        emit Burned(msg.sender, creditAmount);
    }

    /// @notice Timestamp when a merkle root was proposed
    uint256 public proposedAt;
    /// @notice Pending merkle root awaiting timelock expiration
    bytes32 public proposedRoot;
    /// @notice Minimum delay before finalize (e.g., 48 hours)
    uint256 public constant FINALIZE_DELAY = 48 hours;

    /**
     * @notice Propose a merkle root for later finalization
     * @dev Owner proposes; community can verify off-chain during delay window
     */
    function proposeRoot(bytes32 _root) external onlyOwner {
        if (_root == bytes32(0)) revert ZeroMerkleRoot();
        if (finalized) revert AlreadyFinalized();
        if (proposedRoot != bytes32(0)) revert PendingRootExists();
        _queueRoot(_root);
    }

    /**
     * @notice Cancel a pending root proposal
     * @dev Allows owner to correct mistakes before finalization
     */
    function cancelProposedRoot() external onlyOwner {
        if (proposedRoot == bytes32(0)) revert NoPendingRoot();
        bytes32 root = proposedRoot;
        proposedRoot = bytes32(0);
        proposedAt = 0;
        emit RootCanceled(root);
    }

    /**
     * @notice Finalize the burn period and set the verified merkle root
     * @dev FIX SCWE-057: adds timelock and ties to on-chain burned accounting
     */
    function finalize() external onlyOwner {
        if (finalized) revert AlreadyFinalized();
        if (proposedRoot == bytes32(0)) revert ZeroMerkleRoot();
        require(block.timestamp >= proposedAt + FINALIZE_DELAY, "Timelock not expired");
        if (totalBurned == 0) revert ZeroAmount();

        finalized = true;
        merkleRoot = proposedRoot;
        proposedRoot = bytes32(0);
        proposedAt = 0;

        emit Finalized(merkleRoot);
    }

    /**
     * @notice Re-open the migration finalization in case the root was incorrect
     * @dev Clears the finalized root and enforces a new delay window for the replacement root
     * @param newRoot Replacement merkle root proposal
     */
    function reopenFinalization(bytes32 newRoot) external onlyOwner {
        if (!finalized) revert NotFinalized();
        if (newRoot == bytes32(0)) revert ZeroMerkleRoot();

        bytes32 previousRoot = merkleRoot;
        finalized = false;
        merkleRoot = bytes32(0);

        _queueRoot(newRoot);
        emit FinalizationReopened(previousRoot, newRoot, proposedAt);
    }

    function _queueRoot(bytes32 _root) private {
        proposedRoot = _root;
        proposedAt = block.timestamp;
        emit RootProposed(_root, proposedAt);
    }

    /**
     * @notice Get burned amount for a user
     * @dev View function to check how many tokens a user has burned
     * @param user Address to query
     * @return uint256 Amount of tokens burned by user
     */
    function getBurnedAmount(address user) external view returns (uint256) {
        return burned[user];
    }

    /**
     * @notice Pause the contract
     * @dev Only owner can pause, prevents burning
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @notice Unpause the contract
     * @dev Only owner can unpause, allows burning
     */
    function unpause() external onlyOwner {
        _unpause();
    }
}
