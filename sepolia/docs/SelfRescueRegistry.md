# Solidity API

## IRescueRegistry

### Contract
IRescueRegistry : contracts/SelfRescueRegistry.sol

 --- 
### Functions:
### isRescueExecutor

```solidity
function isRescueExecutor(address caller) external view returns (bool)
```

### canExecuteRescue

```solidity
function canExecuteRescue(address from) external view returns (bool)
```

## IRestrictedToken

### Contract
IRestrictedToken : contracts/SelfRescueRegistry.sol

 --- 
### Functions:
### transferFrom

```solidity
function transferFrom(address from, address to, uint256 amount) external returns (bool)
```

inherits IERC20:
### totalSupply

```solidity
function totalSupply() external view returns (uint256)
```

_Returns the value of tokens in existence._

### balanceOf

```solidity
function balanceOf(address account) external view returns (uint256)
```

_Returns the value of tokens owned by `account`._

### transfer

```solidity
function transfer(address to, uint256 value) external returns (bool)
```

_Moves a `value` amount of tokens from the caller's account to `to`.

Returns a boolean value indicating whether the operation succeeded.

Emits a {Transfer} event._

### allowance

```solidity
function allowance(address owner, address spender) external view returns (uint256)
```

_Returns the remaining number of tokens that `spender` will be
allowed to spend on behalf of `owner` through {transferFrom}. This is
zero by default.

This value changes when {approve} or {transferFrom} are called._

### approve

```solidity
function approve(address spender, uint256 value) external returns (bool)
```

_Sets a `value` amount of tokens as the allowance of `spender` over the
caller's tokens.

Returns a boolean value indicating whether the operation succeeded.

IMPORTANT: Beware that changing an allowance with this method brings the risk
that someone may use both the old and the new allowance by unfortunate
transaction ordering. One possible solution to mitigate this race
condition is to first reduce the spender's allowance to 0 and set the
desired value afterwards:
https://github.com/ethereum/EIPs/issues/20#issuecomment-263524729

Emits an {Approval} event._

 --- 
### Events:
inherits IERC20:
### Transfer

```solidity
event Transfer(address from, address to, uint256 value)
```

_Emitted when `value` tokens are moved from one account (`from`) to
another (`to`).

Note that `value` may be zero._

### Approval

```solidity
event Approval(address owner, address spender, uint256 value)
```

_Emitted when the allowance of a `spender` for an `owner` is set by
a call to {approve}. `value` is the new allowance._

## SelfRescueRegistry

### Contract
SelfRescueRegistry : contracts/SelfRescueRegistry.sol

 --- 
### Functions:
### constructor

```solidity
constructor(address owner_, address _TOKEN) public
```

Constructor

_Initializes the contract with owner and TOKEN address_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| owner_ | address | The owner address |
| _TOKEN | address | The TOKEN address that can be rescued |

### registerPlan

```solidity
function registerPlan(address recovery, uint64 delay) external
```

Register a rescue plan

_Allows users to set up a recovery address and delay period_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| recovery | address | The recovery address that will receive rescued funds |
| delay | uint64 | The delay period in seconds before rescue can be executed |

### initiateRescue

```solidity
function initiateRescue() external
```

Initiate a rescue operation

_Starts the timelock countdown for rescue execution_

### cancelRescue

```solidity
function cancelRescue() external
```

Cancel an active rescue operation

_FIX L-04: Resets cooldown timer to allow immediate re-initiation after cancel
Allows users to cancel their pending rescue_

### canExecuteRescue

```solidity
function canExecuteRescue(address victim) external view returns (bool)
```

Check if a rescue can be executed for a victim

_View function to check rescue eligibility_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| victim | address | The address to check |

#### Return Values

| Name | Type | Description |
| ---- | ---- | ----------- |
| [0] | bool | bool True if rescue can be executed |

### isRescueExecutor

```solidity
function isRescueExecutor(address caller) external view returns (bool)
```

Check if an address is a rescue executor

_View function for executor status_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| caller | address | The address to check |

#### Return Values

| Name | Type | Description |
| ---- | ---- | ----------- |
| [0] | bool | bool True if the address is an executor |

### setExecutor

```solidity
function setExecutor(address exec, bool enabled) external
```

Set executor status for an address

_Only owner can set executors_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| exec | address | The executor address |
| enabled | bool | Whether to enable or disable |

### setMaxRescueAmount

```solidity
function setMaxRescueAmount(uint256 maxAmount) external
```

Set the maximum rescue amount

_Only owner can set the maximum amount per rescue_

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| maxAmount | uint256 | The maximum amount in TOKEN units |

### executeRescue

```solidity
function executeRescue(address victim, uint256 amount) external
```

Execute a rescue operation

_FIX M-01: Transfers TOKENs from victim to their registered recovery address
     IMPORTANT: This requires the victim to have approved this contract for at least
     the rescue amount. This is a self-rescue mechanism, NOT forced recovery._

#### Parameters

| Name | Type | Description |
| ---- | ---- | ----------- |
| victim | address | The address to rescue from |
| amount | uint256 | The amount to rescue |

### pause

```solidity
function pause() external
```

Pause the contract

_Only owner can pause_

### unpause

```solidity
function unpause() external
```

Unpause the contract

_Only owner can unpause_

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

inherits IRescueRegistry:

 --- 
### Events:
### PlanRegistered

```solidity
event PlanRegistered(address user, address recovery, uint64 delay)
```

### RescueInitiated

```solidity
event RescueInitiated(address user, uint64 eta)
```

### RescueCanceled

```solidity
event RescueCanceled(address user)
```

### RescueExecuted

```solidity
event RescueExecuted(address user, address recovery, uint256 amount)
```

### ExecutorSet

```solidity
event ExecutorSet(address executor, bool enabled)
```

### MaxRescueAmountSet

```solidity
event MaxRescueAmountSet(uint256 amount)
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

inherits IRescueRegistry:

