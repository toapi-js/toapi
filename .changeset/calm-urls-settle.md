---
"@toapi/client": patch
---

Consume pending cache invalidations before replaying them so duplicate invalidations stop scheduling timers and notifying subscribers after the debounce period. Replay only pending URLs after unlocking the expired batch, preserving invalidations for multiple URLs.
