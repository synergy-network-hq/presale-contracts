# Audit Findings Report: BSC SelfRescueRegistry

**Overview**

The BSC version of `SelfRescueRegistry` mirrors the Ethereum contract.  Users may register rescue plans and execute them after a delay.  The analysis found no vulnerabilities.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Gas costs for dynamic arrays (`executors`) may vary across chains; monitor growth.
- The emergency rescue delay should reflect the block time differences on BSC.

**Recommendations**

- Consider adding off‑chain monitoring to alert users when their rescue plan matures.