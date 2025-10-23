# Audit Findings Report: Polygon SNRGSwap

**Overview**

The `SNRGSwap` contract on Polygon behaves like its Ethereum counterpart by burning SNRG tokens and recording the burn amounts until finalisation.  No vulnerabilities were discovered.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Ensure users approve the contract for token burning prior to calling `burnForReceipt`.
- A detailed event for finalisation would improve transparency.

**Recommendations**

- Synchronise the Merkle root distribution process with the Ethereum and BSC chains to avoid inconsistencies.