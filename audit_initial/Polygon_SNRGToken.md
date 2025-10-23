# Audit Findings Report: Polygon SNRGToken

**Overview**

The `SNRGToken` contract on Polygon replicates the restricted ERC20 logic.  Transfers are limited to specific contract addresses.  No vulnerabilities of concern were detected.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Confirm that token decimals and permit parameters are accurate for Polygon.
- Clear communication of transfer restrictions will help avoid failed user transactions.

**Recommendations**

- Keep dependencies up to date and test permit signatures across chains.