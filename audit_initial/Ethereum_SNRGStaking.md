# Audit Findings Report: Ethereum SNRGStaking

**Overview**

This report covers the Ethereum `SNRGStaking` contract.  The contract allows users to stake SNRG for fixed durations with predetermined reward rates, supports early withdrawals with a fee and includes a funding function.  No high or medium severity vulnerabilities were detected.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Consider adding checks to prevent staking before the contract has been funded.
- Reward rates are hard coded; ensure they remain appropriate for your tokenomics.

**Recommendations**

- Maintain comprehensive test coverage and monitor staking behaviour on mainnet.
