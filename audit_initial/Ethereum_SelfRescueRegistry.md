# Audit Findings Report: Ethereum SelfRescueRegistry

**Overview**

The `SelfRescueRegistry` contract allows users to register a recovery plan and execute a rescue after a delay.  It features configurable executors and enforces a minimum delay.  The `executeRescue` function was modified to accept an `amount` parameter for flexible rescues.  No high or medium severity issues were observed.

**High severity issues**

- None.

**Medium severity issues**

- None.

**Low severity issues / Observations**

- Ensure that the rescue delay and executor permissions are well documented to prevent misuse.
- Gas optimisation opportunities exist in removing redundant storage loads when resetting the ETA.

**Recommendations**

- Add additional events for plan registration, initiation and cancellation to improve off‑chain tracking.