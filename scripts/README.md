# scripts

Procedural / generator helpers written in **F#** (`dotnet fsi`). Prefer F# for
generators, stimulus loops, and ad-hoc tooling that is not mandated to be
Python or shell. Keep generated F# scripts in this folder.

| Script | Purpose |
|---|---|
| `clic-smoke-loop.fsx [intervalSeconds]` | Streams channel-10 clicks (even=fort, odd=faible) into SirenOrchestra via MCP `send_midi`. |
| `clic-get-volume.fsx` | Reads ClicVolume / ClicSpread / ClicBias / MasterVolume over MCP. |
| `clic-set-volume.fsx [value]` | Forces ClicVolume (and MasterVolume) to `value`, default 1.0. |
| `clic-set-output.fsx ["device" or "Main output"]` | Sets `clic.output_device` via MCP `set_setting`. |
| `capture-window.fsx [nameSubstring] [out.png]` | Captures an on-screen window by owner name (CGWindowID + `screencapture -l`), not the desktop. |

Discovery: `~/.composesiren_mcp.json` → `http://127.0.0.1:<port>/mcp`.

Clic audio parameters (APVTS group `"C"`): `ClicVolume`, `ClicSpread`,
`ClicBias`, `ClicDecay` (0 = ~2 ms fade, 1 = full sample).
