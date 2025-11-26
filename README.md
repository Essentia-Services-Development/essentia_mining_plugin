# Essentia Mining Plugin

Bitcoin and cryptocurrency mining plugin for the Essentia ecosystem.

## Features

- **Hardware Detection**: Leverages `essentia_hwdetect` for CPU/GPU capability detection
- **Background Processing**: Uses `essentia_async_runtime` for non-blocking mining
- **Resource Management**: Integrates with `essentia_resource_management` for CPU throttling
- **Pool Support**: Stratum protocol implementation for mining pool integration
- **SHA-256 Implementation**: Pure Rust SHA-256 for Proof-of-Work validation

## Usage

```rust
use essentia_mining_plugin::{MiningPlugin, MiningConfig};

let config = MiningConfig::default()
    .with_max_cpu_usage(50)
    .with_background_priority(true);

let plugin = MiningPlugin::new(config)?;
plugin.start_background_mining()?;
```

## SSOP Compliance

This plugin is fully SSOP-compliant (std-only, zero third-party dependencies).

## License

MIT
