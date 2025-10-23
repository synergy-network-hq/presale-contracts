# Audit Findings Report: BSC Timelock

**Overview**

On BSC the `Timelock` contract wraps OpenZeppelin's `TimelockController` in the same way as Ethereum.  It correctly assigns the multisig wallet as proposer and admin.  No issues were identified.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- The timelock parameters (delay and roles) should be tested on testnet before mainnet deployment to ensure cross‑chain compatibility.

**Recommendations**

- Keep OpenZeppelin contracts updated across all chains.