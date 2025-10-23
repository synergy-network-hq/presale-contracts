# Audit Findings Report: Polygon SNRGStaking

**Overview**

The Polygon `SNRGStaking` contract matches the Ethereum staking logic.  It supports fixed‑duration staking with set rewards and early withdrawal fees.  No major issues were found.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Ensure the contract is funded before enabling staking.
- Consider adjusting reward durations and rates to account for Polygon’s shorter block times.

**Recommendations**

- Monitor staking metrics and adjust parameters as needed.