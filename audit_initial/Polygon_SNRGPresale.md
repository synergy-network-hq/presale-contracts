# Audit Findings Report: Polygon SNRGPresale

**Overview**

The Polygon `SNRGPresale` contract replicates the Ethereum implementation and facilitates presale purchases with native MATIC and ERC20 tokens.  The audit detected no security flaws.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- As Polygon uses a different chain ID than Ethereum, ensure that the signature verification includes the correct chain ID to prevent replay attacks.
- Payment tokens must be whitelisted explicitly.

**Recommendations**

- Keep contract parameters consistent across chains and test thoroughly on testnet.