---
"automa-core": minor
---

Added `POST /api/browsers/{id}/launch` and `POST /api/browsers/{id}/kill` endpoints for remote browser management.
Implemented HashMap PID registry for tracking and safely terminating browser instances.
