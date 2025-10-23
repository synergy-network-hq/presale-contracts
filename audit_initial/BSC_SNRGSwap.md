# Audit Findings Report: BSC SNRGSwap

**Overview**

The BSC implementation of `SNRGSwap` follows the Ethereum design: users burn SNRG in exchange for a receipt and a Merkle root is recorded upon finalisation.  The analysis found no security issues.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- As with Ethereum, ensure allowance checks remain consistent with BEP20 semantics.
- Consider using events to improve traceability of burns and finalisation.

**Recommendations**

- Coordinate off‑chain distribution of receipts with on‑chain `burned` mappings.