---
"@toapi/cache": patch
---

Serialize PostgreSQL cache invalidations and garbage collection with a transaction-level advisory lock to prevent overlapping deletions from deadlocking. Send invalidation notifications after the deletion commits, so a notification that cannot be sent no longer blocks invalidation. Queue deletions per instance so lock waiters cannot exhaust the connection pool, and discard connections whose transaction could not be rolled back.
