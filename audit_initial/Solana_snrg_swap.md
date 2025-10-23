# Audit Findings Report: Solana snrg_swap

**Overview**

The `snrg_swap` program on Solana allows users to burn SNRG SPL tokens and records their burn amounts.  After finalisation a Merkle root is recorded and no further burns are allowed.  The audit found no high or medium severity issues.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- The program currently does not provide an instruction to claim redeemed tokens using the Merkle proof; this must be implemented off‑chain or in a separate program.
- There is no event system on Solana; however, you may log messages via `msg!` for tracking finalisation.

**Recommendations**

- Consider adding instructions to allow users to redeem their receipts according to the Merkle distribution.