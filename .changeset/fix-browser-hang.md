---
"automa-core": patch
---

Fix infinite hang in browser worker launch:
- Fixed an issue where `BrowserManager` used a static `user-data-dir`, causing new Chrome processes to instantly exit if a previous instance was still alive. Now dynamically appends timestamps to the profile directory.
- Fixed an issue where Worker Extension could not connect to SSE stream by adding `CorsLayer` to `axum::Router`.
- Increased Handshake Timeout to 30s and properly return `503 Service Unavailable` if the worker fails to connect, preventing VS Code from hanging infinitely.
