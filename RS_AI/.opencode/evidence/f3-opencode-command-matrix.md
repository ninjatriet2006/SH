# F3 OpenCode command matrix evidence

Probe date/platform: 2026-09-09, Linux x86_64. Commands were run from the workspace without modifying environment variables.

| Purpose | Exact command/arguments | Result / fallback contract |
|---|---|---|
| Version | `opencode --version` | exit 0, `1.18.30` |
| Root capabilities | `opencode --help` | exit 0; lists `opencode [project]`, `opencode web`, `--hostname`, `--port` |
| Web capability | `opencode web --help` | exit 0; `start opencode server and open web interface`; supports `--hostname` and `--port` |
| Hidden web | `opencode web --hostname 127.0.0.1 --port <reserved-port>` | smoke exit via timeout after printing `Web interface: http://127.0.0.1:19876/`; backend discards stdio and verifies HTTP readiness |
| Visible terminal (Linux) | `gnome-terminal -- opencode` | `/usr/bin/gnome-terminal` installed; fallback `/usr/bin/x-terminal-emulator -e opencode` installed |

Runtime matrix: binary is resolved from inherited `PATH`; environment is inherited; CWD is changed to the user home so launch is project-neutral. Web support is accepted only when version and web help probes succeed and help advertises both required flags. Missing binary, unsupported web command, spawn failure, missing terminal emulator, and browser failure have distinct typed error codes. All arguments use `Command::args`; no shell interpolation is used. Web always binds literal `127.0.0.1` and generated/opened URLs are restricted to `http://127.0.0.1|localhost:<nonzero-port>/`.
