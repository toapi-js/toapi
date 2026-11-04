---
"@toapi/cache": patch
---

Serialize PostgreSQL cache invalidations and garbage collection with a transaction-level advisory lock to prevent overlapping deletions from deadlocking. Commit invalidation notifications atomically with deletion.
