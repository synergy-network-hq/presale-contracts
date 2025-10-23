# Audit Findings Report: Solana snrg_presale

**Overview**

This report evaluates the Solana `snrg_presale` program.  The program is a simplified Anchor implementation of the SNRG presale allowing purchases with SOL or supported SPL tokens.  It lacks off‑chain signature verification but records used nonces to prevent replay.  No critical vulnerabilities were found.

**High severity issues**

- None.

**Medium severity issues**

- The current implementation does not verify off‑chain signatures on‑chain.  This could allow unauthorised purchases if signature verification is expected.  Consider implementing ECDSA verification using secp256k1 instructions.

**Low severity issues / Observations**

- The program stores used nonces in an ever‑growing vector.  Over time this may become inefficient; a more scalable approach is recommended.
- There is no mechanism to remove supported tokens en masse; tokens must be removed one by one.

**Recommendations**

- Add signature verification to enforce authorisation of presale purchases.
- Use a mapping (e.g., using `HashSet`) or a Bloom filter to track used nonces more efficiently.