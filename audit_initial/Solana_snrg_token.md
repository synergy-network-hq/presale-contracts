# Audit Findings Report: Solana snrg_token

**Overview**

The `snrg_token` program acts as a wrapper around the SPL Token program.  It mints a fixed supply of SNRG to the treasury and restricts transfers to authorised program accounts or via the rescue registry.  No critical issues were observed.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- The transfer restrictions rely on the owner addresses of token accounts; ensure these PDAs are correctly derived and that the staking/swap programs use associated token accounts controlled by their program addresses.
- The `transfer_restricted` instruction does not enforce that only whitelisted directions are allowed when the `rescue_registry` is unset; verify this logic carefully.

**Recommendations**

- Consider implementing an on‑chain registry of allowed program accounts to simplify transfer checks.