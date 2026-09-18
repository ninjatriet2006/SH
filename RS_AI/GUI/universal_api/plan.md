# Universal API GUI Plan (formerly WorkBuddy GUI Full Port Plan)

Goal: Port WorkBuddy2API (Go ~16k LOC) into standalone Tauri v2 GUI app.

Phases:
1. Scaffold boilerplate - active
2. Port small Rust modules (auth, config, prompt, vpn, session, redisstore)
3. Port upstream module (4.7k LOC)
4. Port pool module (3.2k LOC)
5. Port server + scheduler (4.3k LOC)
6. Tauri API commands
7. Frontend React pages + stores
8. Bridge TypeScript
9. Assets + workspace wiring
10. Verification

Current: Phase 1 active. Next gate: cargo check on scaffold.
