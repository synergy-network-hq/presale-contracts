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
 * */

import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title SNRGPresale
 * @author DevPup
 * @notice Presale contract for SNRG tokens with signature-based verification
 * @dev Implements rate limiting and purchase controls with cryptographic signatures
 */
contract SNRGPresale is Ownable2Step, ReentrancyGuard, Pausable {
    using SafeERC20 for IERC20;

    /// @notice SNRG token contract address
    IERC20 public immutable SNRG;
    /// @notice Treasury address for receiving payments
    address public immutable TREASURY;
    /// @notice Signer address for signature verification
    address public signer;
    /// @notice Whether presale is currently open
    bool public open;

    /// @notice Mapping of supported payment tokens
    mapping(address => bool) public supportedTokens;
    /// @notice Mapping of used nonces to prevent replay attacks
    mapping(uint256 => bool) public usedNonces;

    /// @notice Last purchase time per user for cooldown enforcement
    mapping(address => uint256) public lastPurchaseTime;
    /// @notice Daily purchase count per user
    mapping(address => uint256) public purchaseCountToday;
    /// @notice Daily reset timestamp per user
    mapping(address => uint256) public dailyPurchaseReset;

    /// @notice Cooldown period between purchases (5 minutes)
    uint256 public constant PURCHASE_COOLDOWN = 5 minutes;
    /// @notice Maximum purchases allowed per day per user
    uint256 public constant MAX_PURCHASES_PER_DAY = 10;
    /// @notice Minimum purchase amount in SNRG tokens
    uint256 public constant MIN_PURCHASE_AMOUNT = 250 * 10 ** 9;
    /// @notice Maximum purchase amount per transaction
    uint256 public maxPurchaseAmount;

    error PresaleClosed();
    error ZeroAddress();
    error ZeroAmount();
    error TokenNotSupported();
    error NonceAlreadyUsed();
    error InvalidSignature();
    error TreasuryTransferFailed();
    error PurchaseTooSoon();
    error DailyLimitExceeded();
    error AmountTooLow();
    error AmountTooHigh();
    error InvalidNonce();
    error InsufficientBalance();

    event Purchased(
        address indexed buyer,
        address indexed paymentToken,
        uint256 indexed snrgAmount,
        uint256 paidAmount
    );
    event SignerSet(address indexed oldSigner, address indexed newSigner);
    event SupportedTokenSet(address indexed token, bool indexed isSupported);
    event OpenSet(bool open);
    event MaxPurchaseAmountSet(uint256 amount);
    event TokenRecovered(address indexed token, uint256 amount);
    event EthRecovered(uint256 amount);

    /**
     * @notice Constructor
     * @dev Initializes presale contract with immutable addresses
     * @param _snrg SNRG token address
     * @param _TREASURY Treasury address receiving payments
     * @param owner_ Owner address
     * @param _signer Authorized signer address for purchases
     */
    constructor(
        address _snrg,
        address _TREASURY,
        address owner_,
        address _signer
    ) Ownable(owner_) {
        if (_snrg == address(0)) revert ZeroAddress();
        if (_TREASURY == address(0)) revert ZeroAddress();
        if (_signer == address(0)) revert ZeroAddress();
        if (owner_ == address(0)) revert ZeroAddress();

        SNRG = IERC20(_snrg);
        TREASURY = _TREASURY;
        signer = _signer;
        maxPurchaseAmount = 10_000_000 * 10 ** 9;
    }

    /**
     * @notice Set the authorized signer address
     * @dev Only owner can update the signer
     * @param _signer New signer address
     */
    function setSigner(address _signer) external onlyOwner {
        if (_signer == address(0)) revert ZeroAddress();
        address oldSigner = signer;
        signer = _signer;
        emit SignerSet(oldSigner, _signer);
    }

    /**
     * @notice Set presale open status
     * @dev Only owner can open/close presale
     * @param v True to open, false to close
     */
    function setOpen(bool v) external onlyOwner {
        open = v;
        emit OpenSet(v);
    }

    /**
     * @notice Set supported payment token
     * @dev Only owner can add/remove supported tokens
     * @param token Token address
     * @param isSupported Support status
     */
    function setSupportedToken(address token, bool isSupported) external onlyOwner {
        if (token == address(0)) revert ZeroAddress();
        supportedTokens[token] = isSupported;
        emit SupportedTokenSet(token, isSupported);
    }

    /**
     * @notice Set maximum purchase amount
     * @dev Only owner can set the limit
     * @param _max Maximum amount in token units
     */
    function setMaxPurchaseAmount(uint256 _max) external onlyOwner {
        if (_max == 0) revert ZeroAmount();
        if (_max < MIN_PURCHASE_AMOUNT) revert AmountTooLow();
        maxPurchaseAmount = _max;
        emit MaxPurchaseAmountSet(_max);
    }

    /**
     * @notice Purchase SNRG with native token (ETH/MATIC/etc)
     * @dev Requires valid signature from authorized signer
     * @param snrgAmount Amount of SNRG to purchase
     * @param nonce Unique nonce for this transaction
     * @param signature Cryptographic signature from signer
     */
    function buyWithNative(
        uint256 snrgAmount,
        uint256 nonce,
        bytes calldata signature
    ) external payable nonReentrant whenNotPaused {
        if (!open) revert PresaleClosed();
        if (snrgAmount == 0) revert ZeroAmount();
        if (msg.value == 0) revert ZeroAmount();

        _checkPurchaseLimits(msg.sender, snrgAmount);
        _checkNonce(nonce);

        bytes32 messageHash = _buildMessageHash(
            msg.sender,
            address(0),
            msg.value,
            snrgAmount,
            nonce
        );
        _verifySignature(messageHash, signature, nonce);

        _processPurchase(msg.sender, snrgAmount);
        _updatePurchaseTracking(msg.sender);

        // Use safe transfer to treasury with unrestricted gas
        (bool success, ) = TREASURY.call{value: msg.value}("");
        if (!success) revert TreasuryTransferFailed();

        emit Purchased(msg.sender, address(0), snrgAmount, msg.value);
    }

    /**
     * @notice Purchase SNRG with ERC20 token
     * @dev Requires valid signature from authorized signer
     * @param paymentToken Payment token address
     * @param paymentAmount Payment amount
     * @param snrgAmount Amount of SNRG to purchase
     * @param nonce Unique nonce for this transaction
     * @param signature Cryptographic signature from signer
     */
    function buyWithToken(
        address paymentToken,
        uint256 paymentAmount,
        uint256 snrgAmount,
        uint256 nonce,
        bytes calldata signature
    ) external nonReentrant whenNotPaused {
        if (!open) revert PresaleClosed();
        if (paymentToken == address(0)) revert ZeroAddress();
        if (!supportedTokens[paymentToken]) revert TokenNotSupported();
        if (paymentAmount == 0) revert ZeroAmount();
        if (snrgAmount == 0) revert ZeroAmount();

        _checkPurchaseLimits(msg.sender, snrgAmount);
        _checkNonce(nonce);

        bytes32 messageHash = _buildMessageHash(
            msg.sender,
            paymentToken,
            paymentAmount,
            snrgAmount,
            nonce
        );
        _verifySignature(messageHash, signature, nonce);

        _processPurchase(msg.sender, snrgAmount);
        _updatePurchaseTracking(msg.sender);

        IERC20(paymentToken).safeTransferFrom(msg.sender, TREASURY, paymentAmount);

        emit Purchased(msg.sender, paymentToken, snrgAmount, paymentAmount);
    }

    /**
     * @notice Validate nonce
     * @dev Internal function to check nonce validity
     * @param nonce Nonce to validate
     */
    function _checkNonce(uint256 nonce) internal pure {
        if (nonce == 0 || nonce > type(uint128).max) revert InvalidNonce();
    }

    /**
     * @notice Check purchase limits and restrictions
     * @dev Internal function to validate purchase constraints
     * @param buyer Buyer address
     * @param snrgAmount SNRG amount to purchase
     */
    function _checkPurchaseLimits(address buyer, uint256 snrgAmount) internal view {
        if (snrgAmount < MIN_PURCHASE_AMOUNT) revert AmountTooLow();
        if (snrgAmount > maxPurchaseAmount) revert AmountTooHigh();

        // Note: block.timestamp manipulation (~15 min) is acceptable for cooldowns
        if (block.timestamp < lastPurchaseTime[buyer] + PURCHASE_COOLDOWN) {
            revert PurchaseTooSoon();
        }

        uint256 resetTime = dailyPurchaseReset[buyer];
        uint256 count = purchaseCountToday[buyer];

        if (block.timestamp >= resetTime + 1 days) {
            count = 0;
        }

        if (count >= MAX_PURCHASES_PER_DAY) {
            revert DailyLimitExceeded();
        }
    }

    /**
     * @notice Update purchase tracking data
     * @dev Internal function to maintain purchase limits
     * @param buyer Buyer address
     */
    function _updatePurchaseTracking(address buyer) internal {
        lastPurchaseTime[buyer] = block.timestamp;

        if (block.timestamp >= dailyPurchaseReset[buyer] + 1 days) {
            purchaseCountToday[buyer] = 1;
            dailyPurchaseReset[buyer] = block.timestamp;
        } else {
            purchaseCountToday[buyer]++;
        }
    }

    /**
     * @notice Build message hash for signature verification
     * @dev FIX L-02: Updated documentation - uses EIP-191 style hash (personal_sign), not EIP-712
     * @param buyer Buyer address
     * @param paymentToken Payment token address (0 for native)
     * @param paymentAmount Payment amount
     * @param snrgAmount SNRG amount
     * @param nonce Transaction nonce
     * @return bytes32 Message hash
     */
    function _buildMessageHash(
        address buyer,
        address paymentToken,
        uint256 paymentAmount,
        uint256 snrgAmount,
        uint256 nonce
    ) internal view returns (bytes32) {
        return keccak256(
            abi.encodePacked(
                buyer,
                paymentToken,
                paymentAmount,
                snrgAmount,
                nonce,
                block.chainid,
                address(this)
            )
        );
    }

    /**
     * @notice Verify cryptographic signature
     * @dev FIX L-03: Moved nonce consumption AFTER signature validation to prevent DoS
     * @param messageHash Hash of the message
     * @param signature Signature bytes
     * @param nonce Transaction nonce
     */
    function _verifySignature(
        bytes32 messageHash,
        bytes calldata signature,
        uint256 nonce
    ) internal {
        // Check nonce hasn't been used
        if (usedNonces[nonce]) revert NonceAlreadyUsed();

        bytes32 ethSignedMessageHash = MessageHashUtils.toEthSignedMessageHash(messageHash);
        address recoveredSigner = ECDSA.recover(ethSignedMessageHash, signature);

        // Prevent signature malleability by ensuring recovered signer is not zero address
        if (recoveredSigner == address(0) || recoveredSigner != signer) revert InvalidSignature();

        // FIX L-03: Consume nonce AFTER successful signature verification
        usedNonces[nonce] = true;
    }

    /**
     * @notice Process SNRG purchase transfer
     * @dev Internal function to transfer SNRG from TREASURY to buyer
     * @param buyer Buyer address
     * @param snrgAmount Amount of SNRG
     */
    function _processPurchase(address buyer, uint256 snrgAmount) internal {
        // Validate inputs
        if (buyer == address(0)) revert ZeroAddress();
        if (snrgAmount == 0) revert ZeroAmount();
        
        // Check treasury has sufficient balance
        if (SNRG.balanceOf(TREASURY) < snrgAmount) {
            revert InsufficientBalance();
        }
        
        // Transfer from treasury to buyer
        SNRG.safeTransferFrom(TREASURY, buyer, snrgAmount);
    }

    /**
     * @notice Get remaining purchases allowed today
     * @dev View function for user's daily limit status
     * @param buyer Buyer address
     * @return uint256 Remaining purchase count
     */
    function getRemainingPurchasesToday(address buyer) external view returns (uint256) {
        uint256 resetTime = dailyPurchaseReset[buyer];
        uint256 count = purchaseCountToday[buyer];

        if (block.timestamp >= resetTime + 1 days) {
            return MAX_PURCHASES_PER_DAY;
        }

        if (count >= MAX_PURCHASES_PER_DAY) {
            return 0;
        }
        return MAX_PURCHASES_PER_DAY - count;
    }

    /**
     * @notice Get time until next purchase allowed
     * @dev View function for cooldown status
     * @param buyer Buyer address
     * @return uint256 Seconds until next purchase
     */
    function getTimeTillNextPurchase(address buyer) external view returns (uint256) {
        uint256 lastTime = lastPurchaseTime[buyer];
        uint256 cooldownEnd = lastTime + PURCHASE_COOLDOWN;

        if (block.timestamp >= cooldownEnd) {
            return 0;
        }
        return cooldownEnd - block.timestamp;
    }

    /**
     * @notice Check if nonce has been used
     * @dev View function for nonce status
     * @param nonce Nonce to check
     * @return bool True if nonce is used
     */
    function isNonceUsed(uint256 nonce) external view returns (bool) {
        return usedNonces[nonce];
    }

    /**
     * @notice Pause the contract
     * @dev Only owner can pause operations
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @notice Unpause the contract
     * @dev Only owner can resume operations
     */
    function unpause() external onlyOwner {
        _unpause();
    }

    /**
     * @notice Recover accidentally sent ERC20 tokens
     * @dev Emergency function to recover non-SNRG tokens
     * @param token Token address to recover
     * @param amount Amount to recover
     */
    function recoverToken(address token, uint256 amount) external onlyOwner {
        if (token == address(SNRG)) revert TokenNotSupported();
        if (token == address(0)) revert ZeroAddress();
        
        IERC20(token).safeTransfer(owner(), amount);
        emit TokenRecovered(token, amount);
    }

    /**
     * @notice Recover accidentally sent native tokens
     * @dev Emergency function to recover ETH/MATIC/etc
     */
    function recoverEth() external onlyOwner {
        uint256 balance = address(this).balance;
        if (balance <= 0) revert ZeroAmount();
        
        // Emit event before external call to prevent reentrancy
        emit EthRecovered(balance);
        
        address ownerAddr = owner();
        (bool success, ) = ownerAddr.call{value: balance}("");
        if (!success) revert TreasuryTransferFailed();
    }
}