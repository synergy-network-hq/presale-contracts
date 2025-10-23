# Audit Findings Report: BSC SNRGStaking

**Overview**

The BSC `SNRGStaking` contract is identical to the Ethereum staking contract after alignment.  It supports staking for set durations with fixed rewards and early withdrawal fees.  No significant issues were discovered.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- The contract must be funded once before users can withdraw rewards; ensure this occurs before opening staking to the public.

**Recommendations**

- Monitor contract state and adopt best practices for BSC gas optimisation.