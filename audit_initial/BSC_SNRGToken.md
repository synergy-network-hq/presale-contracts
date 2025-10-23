# Audit Findings Report: BSC SNRGToken

**Overview**

The BSC `SNRGToken` contract implements the same restricted ERC20 token behaviour as on Ethereum.  It prohibits arbitrary transfers except between specific contract addresses.  The audit did not reveal any critical issues.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Ensure that the BEP20 implementation adheres to BSC network conventions regarding decimals and metadata.
- The `_update` override restricts transfers; user education is important to avoid failed transactions.

**Recommendations**

- Perform routine upgrades to maintain alignment with upstream ERC20/BEP20 standards.