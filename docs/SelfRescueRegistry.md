# Solidity API

## IRestrictedToken

### Contract
IRestrictedToken : ethereum/contracts/SelfRescueRegistry.sol

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
SelfRescueRegistry : ethereum/contracts/SelfRescueRegistry.sol

 --- 
### Functions:
### constructor

```solidity
constructor(address owner_) public
```

### registerPlan

```solidity
function registerPlan(address recovery, uint64 delay) external
```

### initiateRescue

```solidity
function initiateRescue() external
```

### cancelRescue

```solidity
function cancelRescue() external
```

### canExecuteRescue

```solidity
function canExecuteRescue(address victim) external view returns (bool)
```

### isRescueExecutor

```solidity
function isRescueExecutor(address caller) external view returns (bool)
```

### setExecutor

```solidity
function setExecutor(address exec, bool enabled) external
```

### setToken

```solidity
function setToken(address _token) external
```

### executeRescue

```solidity
function executeRescue(address victim, uint256 amount) external
```

Executes the rescue by transferring the specified balance to the recovery address.
This call is permissionless once matured.
MODIFIED: Now accepts an `amount` for flexible rescues.

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

### transferOwnership

```solidity
function transferOwnership(address newOwner) public virtual
```

_Transfers ownership of the contract to a new account (`newOwner`).
Can only be called by the current owner._

### _transferOwnership

```solidity
function _transferOwnership(address newOwner) internal virtual
```

_Transfers ownership of the contract to a new account (`newOwner`).
Internal function without access restriction._

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

inherits Ownable:
### OwnershipTransferred

```solidity
event OwnershipTransferred(address previousOwner, address newOwner)
```

