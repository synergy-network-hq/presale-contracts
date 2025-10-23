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
* */

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

contract SNRGPresale is Ownable, ReentrancyGuard {
    IERC20 public immutable snrg;
    address public immutable treasury;
    address public signer;
    bool public open;

    mapping(address => bool) public supportedTokens;
    mapping(uint256 => bool) public usedNonces;

    event Purchased(address indexed buyer, address indexed paymentToken, uint256 snrgAmount, uint256 paidAmount);
    event SignerSet(address newSigner);
    event SupportedTokenSet(address indexed token, bool isSupported);
    event OpenSet(bool open);

    constructor(address _snrg, address _treasury, address owner_, address _signer) Ownable(owner_) {
        require(_snrg != address(0) && _treasury != address(0) && _signer != address(0), "zero address");
        snrg = IERC20(_snrg);
        treasury = _treasury;
        signer = _signer;
    }

    // --- Owner Functions ---
    function setSigner(address _signer) external onlyOwner {
        require(_signer != address(0), "zero signer");
        signer = _signer;
        emit SignerSet(_signer);
    }

    function setOpen(bool v) external onlyOwner {
        open = v;
        emit OpenSet(v);
    }
    
    function setSupportedToken(address token, bool isSupported) external onlyOwner {
        require(token != address(0), "zero address");
        supportedTokens[token] = isSupported;
        emit SupportedTokenSet(token, isSupported);
    }

    // --- Purchase Functions ---
    function buyWithNative(uint256 snrgAmount, uint256 nonce, bytes calldata signature) external payable nonReentrant {
        require(open, "closed");
        // Validate purchase amounts to prevent zero-value transfers
        require(snrgAmount > 0, "snrg=0");
        require(msg.value > 0, "zero paid");
        
        bytes32 messageHash = _buildMessageHash(msg.sender, address(0), msg.value, snrgAmount, nonce);
        _verifySignature(messageHash, signature, nonce);
        
        _processPurchase(msg.sender, snrgAmount);
        
        (bool success, ) = payable(treasury).call{value: msg.value}("");
        require(success, "treasury forward fail");
        
        emit Purchased(msg.sender, address(0), snrgAmount, msg.value);
    }

    function buyWithToken(address paymentToken, uint256 paymentAmount, uint256 snrgAmount, uint256 nonce, bytes calldata signature) external nonReentrant {
        require(open, "closed");
        // Validate token address and amounts up front
        require(paymentToken != address(0), "token=0");
        require(supportedTokens[paymentToken], "token not supported");
        require(paymentAmount > 0, "amount=0");
        require(snrgAmount > 0, "snrg=0");
        
        bytes32 messageHash = _buildMessageHash(msg.sender, paymentToken, paymentAmount, snrgAmount, nonce);
        _verifySignature(messageHash, signature, nonce);

        _processPurchase(msg.sender, snrgAmount);

        IERC20(paymentToken).transferFrom(msg.sender, treasury, paymentAmount);
        
        emit Purchased(msg.sender, paymentToken, snrgAmount, paymentAmount);
    }

    // --- Internal Logic ---
    function _buildMessageHash(address buyer, address paymentToken, uint256 paymentAmount, uint256 snrgAmount, uint256 nonce) internal view returns (bytes32) {
        return keccak256(abi.encodePacked(buyer, paymentToken, paymentAmount, snrgAmount, nonce, block.chainid));
    }

    function _verifySignature(bytes32 messageHash, bytes calldata signature, uint256 nonce) internal {
        require(!usedNonces[nonce], "nonce already used");
        usedNonces[nonce] = true;

        bytes32 ethSignedMessageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash));
        
        address recoveredSigner = ECDSA.recover(ethSignedMessageHash, signature);
        
        require(recoveredSigner == signer, "invalid signature");
    }

    function _processPurchase(address buyer, uint256 snrgAmount) internal {
        require(snrg.transferFrom(treasury, buyer, snrgAmount), "treasury transfer fail");
    }
}