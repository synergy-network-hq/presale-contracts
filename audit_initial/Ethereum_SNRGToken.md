# Audit Findings Report: Ethereum SNRGToken

**Overview**

The Ethereum `SNRGToken` contract extends the ERC20 standard with burnable and permit functionality.  It restricts transfers by overriding `_update` so that only the treasury, staking, swap and rescue registry addresses may transact under specific conditions.  No high or medium severity issues were found.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- The `_update` function could emit descriptive errors when transfers are disallowed; currently it reverts silently with `invalid transfer`.
- Consider documenting the restrictions clearly for token holders to avoid confusion.

**Recommendations**

- Monitor gas usage and ensure the permit logic remains compliant with the latest EIP‑712 specifications.