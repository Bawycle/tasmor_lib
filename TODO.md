# TODO - Next Releases

## Bugs

## Features

- `EnergyData` (`src/response/energy.rs`) is missing the `frequency: Option<f32>` field present in `EnergyReading` (`src/telemetry/sensor_parser.rs`). Both structs represent the same Tasmota `ENERGY` JSON block — `Frequency` is reported by AC energy monitors (e.g. BL0942, CSE7766) and exposed in both HTTP Status 10 and MQTT SENSOR responses.
