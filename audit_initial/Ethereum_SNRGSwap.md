# Audit Findings Report: Ethereum SNRGSwap

**Overview**

This report analyzes the Ethereum `SNRGSwap` contract.  The contract allows users to burn SNRG tokens in exchange for a receipt recorded off‑chain and supports a one‑time finalization that sets a Merkle root for the distribution.  No critical issues were found.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- The `burnForReceipt` function assumes the user has granted sufficient allowance; ensure UIs prompt users accordingly.
- The contract does not emit an event for the Merkle root update; consider adding a detailed event for transparency (currently the `Finalized` event only logs the root).

**Recommendations**

- Maintain off‑chain logs of burned amounts and correlate with the on‑chain `burned` mapping.