# TODO - Next Releases

> This file tracks planned work for the next releases. It lives only in the `dev` branch and is not included in releases.

## Bugs

- [ ] **MQTT power_off response parsing error**: When sending `power_off` via MQTT, the command executes correctly but the response parsing fails with "expected value at line 1 column 1". This suggests the library receives a non-JSON message (possibly telemetry) instead of the command result. The `power_on` command works correctly. Needs investigation of the MQTT response handling logic.

## Features

- [ ] Auto-discovery via mDNS
- [ ] Auto-discovery via MQTT broker
- [ ] WebSocket support for real-time updates
- [ ] Sequence command builder (via Tasmota Backlog command)
