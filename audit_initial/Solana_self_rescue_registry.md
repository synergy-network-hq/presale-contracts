# Audit Findings Report: Solana self_rescue_registry

**Overview**

The Solana `self_rescue_registry` program implements rescue functionality akin to the Ethereum contract.  Users can register a recovery plan with a delay, initiate and cancel rescues, and executors may execute rescues after the delay.  The program uses PDAs to store registry and plan accounts.  No critical issues were found.

**High severity issues**

- None.

**Medium severity issues**

- The `executors` vector grows dynamically; there is no limit or paging mechanism.  Over time this may lead to high storage costs.
- Only approved executors may execute rescues; ensure the list is managed carefully to prevent abuse.

**Low severity issues / Observations**

- There is no event logging for rescue initiation or cancellation; use `msg!` to aid off‑chain monitoring.
- The delay is measured in seconds, which may vary relative to slot times; consider using slot numbers.

**Recommendations**

- Introduce a maximum number of executors or compress the list.