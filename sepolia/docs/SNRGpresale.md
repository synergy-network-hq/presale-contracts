# Solidity API

## SNRGPresale

Presale contract for SNRG tokens with signature-based verification

_Implements rate limiting and purchase controls with cryptographic signatures_

### Contract
SNRGPresale : contracts/SNRGpresale.sol

Implements rate limiting and purchase controls with cryptographic signatures

 --- 
### Functions:
### constructor

```solidity
constructor(address _snrg, address _TREASURY, address owner_, address _signer) public
```

Constructor

_Initializes presale contract with immutable addresses_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| _snrg | address | SNRG token address |
| _TREASURY | address | Treasury address receiving payments |
| owner_ | address | Owner address |
| _signer | address | Authorized signer address for purchases |

### setSigner

```solidity
function setSigner(address _signer) external
```

Set the authorized signer address

_Only owner can update the signer_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| _signer | address | New signer address |

### setOpen

```solidity
function setOpen(bool v) external
```

Set presale open status

_Only owner can open/close presale_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| v | bool | True to open, false to close |

### setSupportedToken

```solidity
function setSupportedToken(address token, bool isSupported) external
```

Set supported payment token

_Only owner can add/remove supported tokens_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| token | address | Token address |
| isSupported | bool | Support status |

### setMaxPurchaseAmount

```solidity
function setMaxPurchaseAmount(uint256 _max) external
```

Set maximum purchase amount

_Only owner can set the limit_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| _max | uint256 | Maximum amount in token units |

### buyWithNative

```solidity
function buyWithNative(uint256 snrgAmount, uint256 nonce, bytes signature) external payable
```

Purchase SNRG with native token (ETH/MATIC/etc)

_Requires valid signature from authorized signer_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| snrgAmount | uint256 | Amount of SNRG to purchase |
| nonce | uint256 | Unique nonce for this transaction |
| signature | bytes | Cryptographic signature from signer |

### buyWithToken

```solidity
function buyWithToken(address paymentToken, uint256 paymentAmount, uint256 snrgAmount, uint256 nonce, bytes signature) external
```

Purchase SNRG with ERC20 token

_Requires valid signature from authorized signer_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| paymentToken | address | Payment token address |
| paymentAmount | uint256 | Payment amount |
| snrgAmount | uint256 | Amount of SNRG to purchase |
| nonce | uint256 | Unique nonce for this transaction |
| signature | bytes | Cryptographic signature from signer |

### _checkNonce

```solidity
function _checkNonce(uint256 nonce) internal pure
```

Validate nonce

_Internal function to check nonce validity_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| nonce | uint256 | Nonce to validate |

### _checkPurchaseLimits

```solidity
function _checkPurchaseLimits(address buyer, uint256 snrgAmount) internal view
```

Check purchase limits and restrictions

_Internal function to validate purchase constraints_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| buyer | address | Buyer address |
| snrgAmount | uint256 | SNRG amount to purchase |

### _updatePurchaseTracking

```solidity
function _updatePurchaseTracking(address buyer) internal
```

Update purchase tracking data

_Internal function to maintain purchase limits_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| buyer | address | Buyer address |

### _buildMessageHash

```solidity
function _buildMessageHash(address buyer, address paymentToken, uint256 paymentAmount, uint256 snrgAmount, uint256 nonce) internal view returns (bytes32)
```

Build message hash for signature verification

_FIX L-02: Updated documentation - uses EIP-191 style hash (personal_sign), not EIP-712_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| buyer | address | Buyer address |
| paymentToken | address | Payment token address (0 for native) |
| paymentAmount | uint256 | Payment amount |
| snrgAmount | uint256 | SNRG amount |
| nonce | uint256 | Transaction nonce |

#### Return Values

| Name | Type | Description |
| ---- | ---- | ----------- |
| [0] | bytes32 | bytes32 Message hash |

### _verifySignature

```solidity
function _verifySignature(bytes32 messageHash, bytes signature, uint256 nonce) internal
```

Verify cryptographic signature

_FIX L-03: Moved nonce consumption AFTER signature validation to prevent DoS_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| messageHash | bytes32 | Hash of the message |
| signature | bytes | Signature bytes |
| nonce | uint256 | Transaction nonce |

### _processPurchase

```solidity
function _processPurchase(address buyer, uint256 snrgAmount) internal
```

Process SNRG purchase transfer

_Internal function to transfer SNRG from TREASURY to buyer_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| buyer | address | Buyer address |
| snrgAmount | uint256 | Amount of SNRG |

### getRemainingPurchasesToday

```solidity
function getRemainingPurchasesToday(address buyer) external view returns (uint256)
```

Get remaining purchases allowed today

_View function for user's daily limit status_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| buyer | address | Buyer address |

#### Return Values

| Name | Type | Description |
| ---- | ---- | ----------- |
| [0] | uint256 | uint256 Remaining purchase count |

### getTimeTillNextPurchase

```solidity
function getTimeTillNextPurchase(address buyer) external view returns (uint256)
```

Get time until next purchase allowed

_View function for cooldown status_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| buyer | address | Buyer address |

#### Return Values

| Name | Type | Description |
| ---- | ---- | ----------- |
| [0] | uint256 | uint256 Seconds until next purchase |

### isNonceUsed

```solidity
function isNonceUsed(uint256 nonce) external view returns (bool)
```

Check if nonce has been used

_View function for nonce status_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| nonce | uint256 | Nonce to check |

#### Return Values

| Name | Type | Description |
| ---- | ---- | ----------- |
| [0] | bool | bool True if nonce is used |

### pause

```solidity
function pause() external
```

Pause the contract

_Only owner can pause operations_

### unpause

```solidity
function unpause() external
```

Unpause the contract

_Only owner can resume operations_

### recoverToken

```solidity
function recoverToken(address token, uint256 amount) external
```

Recover accidentally sent ERC20 tokens

_Emergency function to recover non-SNRG tokens_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| token | address | Token address to recover |
| amount | uint256 | Amount to recover |

### recoverEth

```solidity
function recoverEth() external
```

Recover accidentally sent native tokens

_Emergency function to recover ETH/MATIC/etc_

inherits Pausable:
### paused

```solidity
function paused() public view virtual returns (bool)
```

_Returns true if the contract is paused, and false otherwise._

### _requireNotPaused

```solidity
function _requireNotPaused() internal view virtual
```

_Throws if the contract is paused._

### _requirePaused

```solidity
function _requirePaused() internal view virtual
```

_Throws if the contract is not paused._

### _pause

```solidity
function _pause() internal virtual
```

_Triggers stopped state.

Requirements:

- The contract must not be paused._

### _unpause

```solidity
function _unpause() internal virtual
```

_Returns to normal state.

Requirements:

- The contract must be paused._

inherits ReentrancyGuard:
### _reentrancyGuardEntered

```solidity
function _reentrancyGuardEntered() internal view returns (bool)
```

_Returns true if the reentrancy guard is currently set to "entered", which indicates there is a
`nonReentrant` function in the call stack._

### _reentrancyGuardStorageSlot

```solidity
function _reentrancyGuardStorageSlot() internal pure virtual returns (bytes32)
```

inherits Ownable2Step:
### pendingOwner

```solidity
function pendingOwner() public view virtual returns (address)
```

_Returns the address of the pending owner._

### transferOwnership

```solidity
function transferOwnership(address newOwner) public virtual
```

_Starts the ownership transfer of the contract to a new account. Replaces the pending transfer if there is one.
Can only be called by the current owner.

Setting `newOwner` to the zero address is allowed; this can be used to cancel an initiated ownership transfer._

### _transferOwnership

```solidity
function _transferOwnership(address newOwner) internal virtual
```

_Transfers ownership of the contract to a new account (`newOwner`) and deletes any pending owner.
Internal function without access restriction._

### acceptOwnership

```solidity
function acceptOwnership() public virtual
```

_The new owner accepts the ownership transfer._

inherits Ownable:
### owner

```solidity
function owner() public view virtual returns (address)
```

_Returns the address of the current owner._

### _checkOwner

```solidity
function _checkOwner() internal view virtual
```

_Throws if the sender is not the owner._

### renounceOwnership

```solidity
function renounceOwnership() public virtual
```

_Leaves the contract without owner. It will not be possible to call
`onlyOwner` functions. Can only be called by the current owner.

NOTE: Renouncing ownership will leave the contract without an owner,
thereby disabling any functionality that is only available to the owner._

 --- 
### Events:
### Purchased

```solidity
event Purchased(address buyer, address paymentToken, uint256 snrgAmount, uint256 paidAmount)
```

### SignerSet

```solidity
event SignerSet(address oldSigner, address newSigner)
```

### SupportedTokenSet

```solidity
event SupportedTokenSet(address token, bool isSupported)
```

### OpenSet

```solidity
event OpenSet(bool open)
```

### MaxPurchaseAmountSet

```solidity
event MaxPurchaseAmountSet(uint256 amount)
```

### TokenRecovered

```solidity
event TokenRecovered(address token, uint256 amount)
```

### EthRecovered

```solidity
event EthRecovered(uint256 amount)
```

inherits Pausable:
### Paused

```solidity
event Paused(address account)
```

_Emitted when the pause is triggered by `account`._

### Unpaused

```solidity
event Unpaused(address account)
```

_Emitted when the pause is lifted by `account`._

inherits ReentrancyGuard:
inherits Ownable2Step:
### OwnershipTransferStarted

```solidity
event OwnershipTransferStarted(address previousOwner, address newOwner)
```

inherits Ownable:
### OwnershipTransferred

```solidity
event OwnershipTransferred(address previousOwner, address newOwner)
```

