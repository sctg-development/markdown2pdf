# Rust Backend Architecture

## Project Structure

```
rust/src/
├── lib.rs                      # Library entry point
├── main.rs                     # Application entry point
├── build_info.rs               # Git/build metadata
│
├── acquisition/                # Audio signal acquisition
│   ├── mod.rs                  # Module exports
│   ├── daemon.rs               # AcquisitionDaemon
│   ├── realtime_daemon.rs      # RealTimeAcquisitionDaemon
│   ├── stream.rs               # SharedAudioStream
│   ├── microphone.rs           # CPAL microphone source
│   ├── file.rs                 # WAV file source
│   ├── mock.rs                 # Mock audio source
│   ├── simulated_photoacoustic.rs  # Physics simulation
│   └── universal.rs            # Universal source wrapper
│
├── preprocessing/              # Signal preprocessing
│   ├── mod.rs
│   ├── differential.rs         # A-B subtraction
│   └── filter/                 # Digital filters
│       ├── standard_filters.rs     # Basic IIR
│       ├── scipy_butter_filter.rs  # Butterworth (SOS)
│       ├── scipy_cheby_filter.rs   # Chebyshev
│       └── scipy_cauer_filter.rs   # Elliptic
│
├── spectral/                   # Spectral analysis
│   ├── mod.rs
│   └── fft.rs                  # FFT analyzer
│
├── processing/                 # Processing pipeline
│   ├── mod.rs
│   ├── graph.rs                # ProcessingGraph (~3000 lines)
│   ├── consumer.rs             # ProcessingConsumer
│   ├── nodes/                  # Processing nodes
│   │   ├── input.rs            # InputNode
│   │   ├── filter.rs           # FilterNode
│   │   ├── differential.rs     # DifferentialNode
│   │   ├── channel.rs          # ChannelSelector/Mixer
│   │   ├── gain.rs             # GainNode
│   │   ├── output.rs           # PhotoacousticOutputNode
│   │   ├── streaming.rs        # StreamingNode (SSE)
│   │   └── python.rs           # PythonNode (PyO3)
│   └── computing_nodes/        # Analytics nodes
│       ├── peak_finder.rs      # PeakFinderNode
│       ├── concentration.rs    # ConcentrationNode
│       ├── universal_action.rs # ActionNode
│       └── action_drivers/     # Pluggable drivers
│           ├── redis.rs
│           ├── http.rs
│           ├── kafka.rs
│           └── python.rs
│
├── visualization/              # Web server
│   ├── mod.rs
│   ├── server/                 # Rocket configuration
│   ├── auth/                   # OAuth2 + JWT
│   ├── api/                    # REST endpoints
│   └── streaming/              # SSE audio streaming
│
├── thermal_regulation/         # Temperature control
│   ├── mod.rs
│   ├── daemon.rs               # PID controller
│   ├── controller.rs           # Advanced thermal control
│   └── drivers/                # I2C drivers
│       ├── mock.rs
│       ├── native.rs           # Linux i2c-dev
│       └── cp2112.rs           # USB-I2C bridge
│
├── modbus/                     # Industrial protocol
│   └── modbus_server.rs        # Modbus TCP server
│
├── daemon/                     # Main daemon
│   └── launch_daemon.rs        # Service orchestration
│
├── config/                     # YAML configuration
│   ├── mod.rs
│   └── *.rs                    # Config structs
│
└── utility/                    # Utilities
    ├── certificate_utilities.rs
    ├── noise_generator.rs
    └── system_stats.rs
```