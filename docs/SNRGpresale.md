# Solidity API

## SNRGPresale Contract

SNRGPresale : ethereum/contracts/SNRGpresale.sol

---

### Functions:

#### constructor

```solidity
constructor(address _snrg, address _treasury, address owner_, address _signer)
```

#### setSigner

```solidity
function setSigner(address _signer) external
```

#### setOpen

```solidity
function setOpen(bool v) external
```

#### setSupportedToken

```solidity
function setSupportedToken(address token, bool isSupported) external
```

#### buyWithNative

```solidity
function buyWithNative(uint256 snrgAmount, uint256 nonce, bytes calldata signature) external payable
```

#### buyWithToken

```solidity
function buyWithToken(address paymentToken, uint256 paymentAmount, uint256 snrgAmount, uint256 nonce, bytes calldata signature) external
```