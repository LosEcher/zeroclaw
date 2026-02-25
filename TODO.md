# TODO

## Channel / Stability Audit Items

- [x] Avoid panic in pairing flow when `spawn_blocking` join fails.
  file: `src/security/pairing.rs`
- [x] Ensure WhatsApp Cloud listener exits when supervisor channel is closed.
  file: `src/channels/whatsapp.rs`
- [x] Ensure WhatsApp Web listener can stop on shutdown and surface unexpected bot exit.
  file: `src/channels/whatsapp_web.rs`
- [x] Ensure Signal SSE listener responds to shutdown while reconnecting/streaming.
  file: `src/channels/signal.rs`
- [x] Ensure shell tool kills child process on timeout cancellation.
  file: `src/tools/shell.rs`
- [x] Cache `reqwest::Client` in `http_request` tool to reuse connection pools.
  file: `src/tools/http_request.rs`

## Build / Resource Control

- [x] Add host-aware build helper with conservative default mode for busy machines.
  file: `scripts/build-auto.sh`
