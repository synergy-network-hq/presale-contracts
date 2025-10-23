# Solidity API

## SNRGStaking Contract

SNRGStaking : ethereum/contracts/SNRGstaking.sol

---

### Functions:

#### constructor

```solidity
constructor(address _treasury, address owner_)
```

#### fundContract

```solidity
function fundContract(uint256 amount) external
```

#### stake

```solidity
function stake(uint256 amount, uint64 duration) external
```

#### withdraw

```solidity
function withdraw(uint256 stakeIndex) external
```

#### withdrawEarly

```solidity
function withdrawEarly(uint256 stakeIndex) external
```

#### getStakeCount

```solidity
function getStakeCount(address user) external view returns (uint256)
```

#### getStake

```solidity
function getStake(address user, uint256 stakeIndex) external view returns (Stake memory)
```

#### setSnrgToken

```solidity
function setSnrgToken(address _snrg) external
```