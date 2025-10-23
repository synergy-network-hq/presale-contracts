# Audit Findings Report: Solana snrg_staking

**Overview**

The Solana `snrg_staking` program replicates the Ethereum staking logic.  Users may stake SNRG SPL tokens for fixed durations and earn rewards.  The program uses Anchor PDAs for staking vaults and tracks stakes in individual accounts.  No critical issues were detected.

**High severity issues**

- None.

**Medium severity issues**

- The reward rates are hard coded.  There is no mechanism for governance to update them; consider adding a `set_reward_rates` instruction.
- Funding is a one‑time action; ensure the contract cannot be under‑funded for subsequent reward redemptions.

**Low severity issues / Observations**

- The `reward_rates` vector is stored on chain; large numbers of durations could increase storage costs.
- Clock drift could cause slight variations in end times; consider using slot‑based delays instead of timestamp.

**Recommendations**

- Implement configurable reward rates and allow the owner to update them via multisig governance.