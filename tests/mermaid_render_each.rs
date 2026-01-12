use std::fs;
use std::path::Path;

use markdown2pdf::{config::ConfigSource, parse_into_file};

const MERMAID_BLOCKS: &[&str] = &[
    r#"
mindmap
  root((Rust-Photoacoustic))
    Signal Processing
      Real-time FFT
      Butterworth filters
      Differential subtraction
      Peak detection
    Hardware Control
      USB-HID interface
      I2C thermal sensors
      SPI DAC/DDS control
      Modbus TCP server
    Web Interface
      Real-time streaming
      OAuth2/JWT security
      Multi-language UI
      Interactive graphs
    Extensibility
      Python integration
      Plugin drivers
      Hot-reload config
      REST API
"#,
    r#"
flowchart TB
    subgraph Hardware["🔧 Hardware Layer"]
        MIC["Microphones A/B"]
        LASER["QCL Laser"]
        TEC["TEC Controllers"]
        NTC["NTC Sensors"]
    end

    subgraph LaserSmart["⚡ Laser+Smart Interface"]
        USB["USB-HID"]
        ADC["16-bit ADC"]
        DAC["12-bit DAC"]
        DDS["DDS Modulator"]
    end

    subgraph Backend["🦀 Rust Backend"]
        ACQ["Acquisition Daemon"]
        PROC["Processing Graph"]
        THERM["Thermal Regulation"]
        API["REST API + SSE"]
        MODBUS["Modbus Server"]
    end

    subgraph Frontend["⚛️ React Frontend"]
        DASH["Dashboard"]
        AUDIO["Audio Analyzer"]
        GRAPH["Processing Graph View"]
        THERMAL["Thermal Monitor"]
    end

    subgraph External["🌐 External Systems"]
        SCADA["SCADA/PLC"]
        REDIS["Redis Pub/Sub"]
        KAFKA["Kafka"]
    end

    MIC --> ADC
    LASER --> DAC
    TEC --> DAC
    NTC --> ADC
    
    ADC --> USB
    DAC --> USB
    DDS --> USB
    USB --> ACQ
    
    ACQ --> PROC
    PROC --> API
    PROC --> MODBUS
    THERM --> API
    
    API --> DASH
    API --> AUDIO
    API --> GRAPH
    API --> THERMAL
    
    MODBUS --> SCADA
    PROC --> REDIS
    PROC --> KAFKA
"#,
    r#"
graph LR
    subgraph Traditional["Traditional Sensors"]
        A[Electrochemical] --> A1["± 1-5% accuracy"]
        B[NDIR] --> B1["ppm resolution"]
        C[Catalytic] --> C1["Cross-sensitivity"]
    end
    
    subgraph LPAS["Laser Photoacoustic"]
        D[QCL Laser] --> D1["ppb resolution"]
        E[Helmholtz Cell] --> E1["Amplified signal"]
        F[Differential] --> F1["Noise rejection"]
    end
    
    style LPAS fill:#90EE90
"#,
    r#"
gantt
    title Revenue Projection (€M)
    dateFormat YYYY
    axisFormat %Y
    
    section R&D Phase
    Development & Patents    :2024, 2025
    
    section Launch Phase
    Lab Product Launch      :2025, 2026
    Industrial Launch       :2026, 2027
    
    section Growth Phase
    Market Expansion        :2027, 2029
"#,
    r#"
pie title Code Distribution
    "Open Source Core" : 70
    "Commercial Plugins" : 20
    "Hardware Designs" : 10
"#,
    r#"
timeline
    title Development Roadmap
    
    Q1 2025 : Hardware prototype v1
            : First customer trials
    
    Q2 2025 : CE certification
            : Production tooling
    
    Q3 2025 : Commercial launch
            : 10 units delivered
    
    Q4 2025 : Series A preparation
            : 30 units backlog
"#,
    r#"
flowchart LR
    subgraph Cell["Differential Helmholtz Cell"]
        subgraph Chamber_A["Chamber A (Excited)"]
            LA["Laser Beam"]
            MA["Microphone A"]
        end
        
        subgraph Neck["Connecting Neck"]
            N["Acoustic Coupling"]
        end
        
        subgraph Chamber_B["Chamber B (Reference)"]
            MB["Microphone B"]
        end
    end
    
    LA --> MA
    MA <--> N
    N <--> MB
    
    style Chamber_A fill:#ffcccc
    style Chamber_B fill:#ccccff
"#,
    r#"
flowchart TB
    subgraph Input["Raw Signals"]
        A["Signal A = PA + Noise"]
        B["Signal B = Noise"]
    end
    
    subgraph Processing["Differential Processing"]
        SUB["A - B"]
    end
    
    subgraph Output["Result"]
        PA["Pure PA Signal"]
    end
    
    A --> SUB
    B --> SUB
    SUB --> PA
    
    style PA fill:#90EE90
"#,
    r#"
flowchart LR
    subgraph Acquisition["1. Acquisition"]
        ADC["48kHz 16-bit ADC"]
    end
    
    subgraph Preprocessing["2. Preprocessing"]
        BP["Bandpass Filter"]
        DIFF["Differential"]
    end
    
    subgraph Spectral["3. Spectral Analysis"]
        WIN["Windowing"]
        FFT["FFT 4096pt"]
        AVG["Averaging"]
    end
    
    subgraph Detection["4. Peak Detection"]
        PEAK["Find f₀"]
        AMP["Extract Amplitude"]
    end
    
    subgraph Output["5. Concentration"]
        CAL["Calibration"]
        CONC["ppm Output"]
    end
    
    ADC --> BP --> DIFF --> WIN --> FFT --> AVG --> PEAK --> AMP --> CAL --> CONC
"#,
    r#"
flowchart TB
    subgraph Input["FFT Magnitude Spectrum"]
        SPEC["mag[0..N/2]"]
    end
    
    subgraph Search["Peak Search"]
        RANGE["Define search range: f₀ ± Δf"]
        MAX["Find local maximum"]
        PARA["Parabolic interpolation"]
    end
    
    subgraph Output["Results"]
        FREQ["Precise frequency"]
        AMP["Amplitude"]
        PHASE["Phase"]
    end
    
    SPEC --> RANGE --> MAX --> PARA --> FREQ
    PARA --> AMP
    PARA --> PHASE
"#,
    r#"
flowchart TB
    subgraph Parameters["Simulation Parameters"]
        F0["Resonance freq: 2000 Hz"]
        Q["Quality factor: 100"]
        T["Temperature: 25°C"]
        C["Concentration: 500 ppm"]
    end
    
    subgraph Model["Physical Model"]
        PA["PA Signal Generation"]
        NOISE["Noise Model"]
        DRIFT["Thermal Drift"]
    end
    
    subgraph Output["Simulated Signals"]
        CH_A["Channel A (Excited)"]
        CH_B["Channel B (Reference)"]
    end
    
    Parameters --> Model --> Output
"#,
    r#"
classDiagram
    class AudioSource {
        <<trait>>
        +name() String
        +sample_rate() u32
        +channels() u16
        +read_frame() Result~AudioFrame~
        +is_realtime() bool
    }
    
    class MicrophoneSource {
        -device: cpal::Device
        -config: StreamConfig
    }
    
    class FileSource {
        -reader: WavReader
        -path: PathBuf
    }
    
    class MockSource {
        -sample_rate: u32
        -frequency: f32
    }
    
    class SimulatedPhotoacousticSource {
        -config: SimulatedSourceConfig
        -resonance_freq: f32
        -concentration: f32
    }
    
    AudioSource <|-- MicrophoneSource
    AudioSource <|-- FileSource
    AudioSource <|-- MockSource
    AudioSource <|-- SimulatedPhotoacousticSource
"#,
    r#"
flowchart TB
    subgraph Graph["ProcessingGraph"]
        INPUT["InputNode"]
        
        subgraph Processing["Processing Nodes"]
            FILTER["FilterNode<br/>(Bandpass)"]
            DIFF["DifferentialNode"]
            GAIN["GainNode"]
        end
        
        subgraph Analytics["Computing Nodes"]
            PEAK["PeakFinderNode"]
            CONC["ConcentrationNode"]
            ACTION["UniversalActionNode"]
        end
        
        subgraph Output["Output Nodes"]
            PA["PhotoacousticOutputNode"]
            STREAM["StreamingNode"]
            RECORD["RecordNode"]
        end
    end
    
    INPUT --> FILTER
    FILTER --> DIFF
    DIFF --> GAIN
    GAIN --> PEAK
    PEAK --> CONC
    CONC --> ACTION
    GAIN --> PA
    GAIN --> STREAM
    GAIN --> RECORD
"#,
    r#"
sequenceDiagram
    participant Source as AudioSource
    participant Daemon as AcquisitionDaemon
    participant Stream as SharedAudioStream
    participant Consumer1 as ProcessingConsumer
    participant Consumer2 as StreamingNode
    
    Source->>Daemon: read_frame()
    Daemon->>Stream: broadcast(frame)
    Stream-->>Consumer1: frame.clone()
    Stream-->>Consumer2: frame.clone()
    Consumer1->>Consumer1: process()
    Consumer2->>Consumer2: encode_sse()
"#,
    r#"
classDiagram
    class ActionDriver {
        <<trait>>
        +execute(measurement: &ActionMeasurement) Result
        +driver_type() String
        +is_available() bool
    }
    
    class RedisActionDriver {
        -client: redis::Client
        -mode: RedisMode
    }
    
    class HttpsCallbackDriver {
        -client: reqwest::Client
        -url: String
    }
    
    class KafkaActionDriver {
        -producer: FutureProducer
        -topic: String
    }
    
    class PythonActionDriver {
        -py_function: PyObject
    }
    
    ActionDriver <|-- RedisActionDriver
    ActionDriver <|-- HttpsCallbackDriver
    ActionDriver <|-- KafkaActionDriver
    ActionDriver <|-- PythonActionDriver
"#,
    r#"
flowchart LR
    subgraph Input
        SP["Setpoint"]
        PV["Process Value<br/>(NTC reading)"]
    end
    
    subgraph PID["PID Controller"]
        E["Error = SP - PV"]
        P["P: Kp × e"]
        I["I: Ki × ∫e dt"]
        D["D: Kd × de/dt"]
        SUM["Σ"]
    end
    
    subgraph Output
        DAC["DAC Output"]
        TEC["TEC Driver"]
    end
    
    SP --> E
    PV --> E
    E --> P --> SUM
    E --> I --> SUM
    E --> D --> SUM
    SUM --> DAC --> TEC
"#,
    r#"
flowchart TB
    subgraph Rocket["Rocket Web Server"]
        subgraph Auth["Authentication"]
            OAUTH["OAuth2 Endpoints"]
            JWT["JWT Validation"]
        end
        
        subgraph API["REST API"]
            CONFIG["GET /api/config"]
            THERMAL["GET /api/thermal"]
            GRAPH["GET /api/graph"]
            COMPUTING["GET /api/computing"]
        end
        
        subgraph SSE["Server-Sent Events"]
            AUDIO["GET /api/audio/stream"]
            SPECTRAL["GET /api/spectral/stream"]
        end
        
        subgraph Static["Static Files"]
            SPA["React SPA"]
            ASSETS["Assets"]
        end
    end
    
    Auth --> API
    Auth --> SSE
"#,
    r#"
sequenceDiagram
    participant Client
    participant Rocket as Rocket Server
    participant OAuth as OAuth2 Provider
    participant JWT as JWT Validator
    
    Client->>Rocket: GET /oauth/authorize
    Rocket->>OAuth: Redirect to login
    OAuth->>Client: Authorization code
    Client->>Rocket: POST /oauth/token
    Rocket->>JWT: Generate JWT
    JWT->>Client: Access token
    Client->>Rocket: GET /api/data (Bearer token)
    Rocket->>JWT: Validate token
    JWT->>Rocket: Claims
    Rocket->>Client: Protected resource
"#,
    r#"
flowchart LR
    subgraph Input["Input Registers (Read-Only)"]
        IR0["0: Resonance Freq (Hz×10)"]
        IR1["1: Amplitude (×1000)"]
        IR2["2: Concentration (ppm×10)"]
        IR3["3-4: Timestamp (low/high)"]
        IR5["5: Status Code"]
    end
    
    subgraph Holding["Holding Registers (R/W)"]
        HR0["0: Measurement Interval"]
        HR1["1: Averaging Count"]
        HR2["2: Gain"]
        HR3["3: Filter Strength"]
    end
"#,
    r#"
flowchart TB
    subgraph Public["Public Routes"]
        HOME["/"]
        E404["/*  (404)"]
    end
    
    subgraph Protected["Protected Routes (AuthenticationGuard)"]
        AUDIO["/audio"]
        THERMAL["/thermal"]
        GRAPH["/graph"]
        BLOG["/blog"]
    end
    
    subgraph Auth["Auth Flow"]
        LOGIN["Login"]
        CALLBACK["Callback"]
    end
    
    HOME --> |"Click protected"| LOGIN
    LOGIN --> CALLBACK
    CALLBACK --> Protected
"#,
    r#"
classDiagram
    class AuthProvider {
        <<interface>>
        +isAuthenticated: boolean
        +isLoading: boolean
        +user: AuthUser
        +login(): Promise
        +logout(): Promise
        +getAccessToken(): Promise~string~
        +hasPermission(permission): Promise~boolean~
        +getJson(url): Promise~any~
        +postJson(url, data): Promise~any~
    }
    
    class Auth0Provider {
        -auth0Client: Auth0Client
    }
    
    class GenerixProvider {
        -oidcClient: UserManager
    }
    
    AuthProvider <|-- Auth0Provider
    AuthProvider <|-- GenerixProvider
"#,
    r#"
sequenceDiagram
    participant Component
    participant Hook as useAudioStream
    participant SSE as EventSource
    participant Audio as Web Audio API
    participant Viz as AudioMotion
    
    Component->>Hook: useAudioStream(streamId)
    Hook->>SSE: Connect to /api/audio/stream
    
    loop Every frame (~20ms)
        SSE->>Hook: AudioFrame (JSON/Binary)
        Hook->>Hook: Decode & buffer
        Hook->>Audio: Queue for playback
        Hook->>Viz: Update analyzer
    end
    
    Hook->>Component: { stats, controls, isConnected }
"#,
    r#"
flowchart TB
    subgraph Host["Host Computer"]
        RUST["Rust Backend<br/>(USB-HID Driver)"]
    end
    
    subgraph LaserSmart["Laser+Smart Board"]
        subgraph MCU["ATmega32U4"]
            USB["USB 2.0<br/>HID Device"]
            I2C["I²C Master<br/>400kHz"]
            SPI["SPI Master<br/>4MHz"]
        end
        
        subgraph Analog["Analog Subsystem"]
            ADC1["ADS1115 #1<br/>0x48"]
            ADC2["ADS1115 #2<br/>0x49"]
            ADC3["ADS1115 #3<br/>0x4A"]
            ADC4["ADS1115 #4<br/>0x4B"]
            REF["REF5040<br/>4.096V"]
        end
        
        subgraph Digital["Digital Subsystem"]
            DDS["AD9833<br/>DDS"]
            GPIO["MCP23017<br/>GPIO"]
        end
    end
    
    subgraph External["External Hardware"]
        DTL1["DTL100 #1<br/>Laser + TEC"]
        DTL2["DTL100 #2<br/>Cell TEC"]
        NTC["NTC Sensors<br/>×4"]
        MIC["Microphones<br/>A/B"]
    end
    
    RUST <-->|"USB-HID"| USB
    USB <--> I2C
    USB <--> SPI
    I2C <--> ADC1
    I2C <--> ADC2
    I2C <--> ADC3
    I2C <--> ADC4
    I2C <--> GPIO
    SPI <--> DDS
    SPI <-->|"J5 Connector"| DTL1
    SPI <-->|"J5 Connector"| DTL2
    REF --> ADC1
    REF --> ADC2
    REF --> ADC3
    REF --> ADC4
    NTC --> ADC4
    MIC --> ADC1
"#,
    r#"
sequenceDiagram
    participant Host
    participant MCU as ATmega32U4
    participant ADC as ADS1115
    
    Host->>MCU: READ_ADC [0, 2]
    MCU->>ADC: I2C: Config register (single-shot, A2)
    MCU->>ADC: I2C: Start conversion
    MCU->>MCU: Wait ~1.2ms (860 SPS)
    ADC->>MCU: I2C: Conversion result (16-bit)
    MCU->>Host: HID: [value_h, value_l]
"#,
    r#"
sequenceDiagram
    participant MCU as ATmega32U4
    participant DAC as LTC2641 (DTL100)
    
    Note over MCU,DAC: Set TEC temperature setpoint
    MCU->>DAC: CS_TEC = LOW
    MCU->>DAC: SPI: [0x30, value_h, value_l]
    MCU->>DAC: CS_TEC = HIGH
    Note over DAC: DAC output updated
"#,
    r#"
flowchart LR
    subgraph Semester1["Step 1"]
        A1["Git Basics"]
        A2["Rust Fundamentals"]
        A3["TypeScript/React Intro"]
    end
    
    subgraph Semester2["Step 2"]
        B1["Systems Programming"]
        B2["Network Protocols"]
        B3["Database Integration"]
    end
    
    subgraph Advanced["Advanced Topics"]
        C1["Concurrency"]
        C2["Signal Processing"]
        C3["Hardware Interfaces"]
    end
    
    A1 --> A2 --> B1 --> C1
    A3 --> B2 --> C2
    B3 --> C3
"#,
    r#"
sequenceDiagram
    participant Client
    participant Server
    participant Auth as Auth Service
    
    Note over Client,Auth: Authentication Flow
    Client->>Server: POST /oauth/token {credentials}
    Server->>Auth: Validate credentials
    Auth-->>Server: User valid
    Server-->>Client: JWT Token
    
    Note over Client,Server: API Request
    Client->>Server: GET /api/data<br/>Authorization: Bearer <token>
    Server->>Server: Validate JWT signature
    Server->>Server: Check expiration
    Server->>Server: Extract claims
    Server-->>Client: Protected data
"#,
    r#"
flowchart LR
    subgraph Input
        SIG["Mixed Signal<br/>100Hz + 2000Hz + 5000Hz"]
    end
    
    subgraph Filter["Bandpass Filter<br/>1800-2200 Hz"]
        BP["Butterworth<br/>4th order"]
    end
    
    subgraph Output
        OUT["Filtered Signal<br/>2000Hz only"]
    end
    
    SIG --> BP --> OUT
"#,
];
#[test]
#[ignore]
fn render_each_mermaid_block_to_pdf() {
    // Use the embedded constant blocks (extracted from the original document by a script).
    let blocks: Vec<&str> = MERMAID_BLOCKS.iter().copied().collect();
    println!(
        "Using {} mermaid blocks from embedded constant",
        blocks.len()
    );

    // Ensure output dir
    let out_dir = Path::new("tests/output");
    if let Err(e) = fs::create_dir_all(out_dir) {
        panic!("Could not create tests/output directory: {}", e);
    }

    let mut successes = 0usize;
    let mut failures: Vec<(usize, String)> = Vec::new();

    for (i, block) in MERMAID_BLOCKS.iter().enumerate() {
        let md = format!("# Mermaid block {}\n\n```mermaid\n{}\n```\n", i + 1, block);
        let out_path = out_dir.join(format!("mermaid_{:03}.pdf", i + 1));
        let out_str = out_path.to_string_lossy().to_string();

        println!("Rendering block {} -> {}", i + 1, out_str);

        match parse_into_file(md, &out_str, ConfigSource::Default, None) {
            Ok(()) => {
                println!("  ✅ OK");
                successes += 1;
            }
            Err(e) => {
                println!("  ❌ FAILED: {:?}", e);
                failures.push((i + 1, format!("{}: {:?}", out_str, e)));
            }
        }
    }

    println!(
        "Summary: {} succeeded, {} failed",
        successes,
        failures.len()
    );
    for (idx, msg) in failures.iter() {
        println!("Failed block {}: {}", idx, msg);
    }

    // Fail the test if any diagrams failed
    assert!(
        failures.is_empty(),
        "{} mermaid block(s) failed; see logs for details",
        failures.len()
    );
}
