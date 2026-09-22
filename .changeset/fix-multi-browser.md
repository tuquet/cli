---
"automa-core": patch
---

Fix multi-browser routing bug in daemon:
- Fixed a bug where `submit_job_handler` evaluated `worker_tx.receiver_count() == 0` globally, causing jobs for a new `browserId` to be ignored instead of launching a new isolated browser process.
- Implemented `connected_browsers` registry using `std::sync::Mutex<HashSet<String>>` to properly track active SSE connections on a per-browser basis.
- Standardized `SubmitJobResponse` to include an optional `message` field, unifying error reporting schemas across 400, 500, and 503 error codes.
- Enhanced `GET /api/browsers` endpoint to include an `is_online` boolean field, which correctly reflects whether the browser currently has an active SSE daemon connection.
