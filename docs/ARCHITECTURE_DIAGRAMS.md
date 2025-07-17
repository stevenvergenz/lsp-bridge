# LSPBridge Architecture Diagrams

This document provides visual representations of the LSPBridge architecture, component interactions, and protocol flows.

## Table of Contents

1. [High-Level Architecture](#high-level-architecture)
2. [Component Interaction Diagram](#component-interaction-diagram)
3. [LSP Protocol Flow](#lsp-protocol-flow)
4. [Server Lifecycle Management](#server-lifecycle-management)
5. [Error Handling Flow](#error-handling-flow)
6. [Multi-Server Architecture](#multi-server-architecture)

## High-Level Architecture

```mermaid
graph TB
    subgraph "Client Application"
        App[Your Application]
        App --> API[LSPBridge API]
    end
    
    subgraph "LSPBridge Core"
        API --> Bridge[LspBridge]
        Bridge --> Client[LspClient]
        Bridge --> Config[Configuration]
        
        Client --> Server1[LspServer 1]
        Client --> Server2[LspServer 2]
        Client --> ServerN[LspServer N]
    end
    
    subgraph "LSP Servers"
        Server1 --> Process1[rust-analyzer]
        Server2 --> Process2[typescript-language-server]
        ServerN --> ProcessN[Other LSP Servers]
    end
    
    subgraph "External Resources"
        Process1 --> FS1[File System]
        Process2 --> FS2[File System]
        ProcessN --> FSN[File System]
    end
    
    style Bridge fill:#e1f5fe
    style Client fill:#f3e5f5
    style API fill:#e8f5e8
```

### Architecture Overview

- **LspBridge**: Main coordinator managing multiple LSP servers
- **LspClient**: Protocol implementation and server communication
- **LspServer**: Individual server lifecycle management
- **LspProcess**: Low-level process communication

## Component Interaction Diagram

```mermaid
graph TD
    subgraph "Application Layer"
        A[Client Application]
    end
    
    subgraph "Bridge Layer"
        B[LspBridge]
        B1[Server Registry]
        B2[Configuration Manager]
        B3[Active Server Manager]
    end
    
    subgraph "Client Layer"
        C[LspClient]
        C1[Server Collection]
        C2[Message Router]
        C3[Active Server Tracker]
    end
    
    subgraph "Server Layer"
        S[LspServer]
        S1[State Manager]
        S2[Request Handler]
        S3[Notification Handler]
        S4[Capabilities Manager]
    end
    
    subgraph "Process Layer"
        P[LspProcess]
        P1[Stdin/Stdout]
        P2[Process Monitor]
        P3[Message Parser]
    end
    
    subgraph "LSP Server Process"
        LSP[External LSP Server]
    end
    
    A --> B
    B --> B1
    B --> B2
    B --> B3
    B --> C
    
    C --> C1
    C --> C2
    C --> C3
    C1 --> S
    
    S --> S1
    S --> S2
    S --> S3
    S --> S4
    S --> P
    
    P --> P1
    P --> P2
    P --> P3
    P1 --> LSP
    
    style B fill:#e1f5fe
    style C fill:#f3e5f5
    style S fill:#fff3e0
    style P fill:#fce4ec
```

## LSP Protocol Flow

### Request/Response Flow

```mermaid
sequenceDiagram
    participant App as Client App
    participant Bridge as LspBridge
    participant Client as LspClient
    participant Server as LspServer
    participant Process as LspProcess
    participant LSP as LSP Server
    
    App->>Bridge: request(method, params)
    Bridge->>Client: forward_request()
    Client->>Server: handle_request()
    
    alt Server Ready
        Server->>Process: send_request()
        Process->>LSP: JSON-RPC request
        LSP->>Process: JSON-RPC response
        Process->>Server: parsed response
        Server->>Client: response
        Client->>Bridge: response
        Bridge->>App: response
    else Server Not Ready
        Server->>Client: ServerNotReady error
        Client->>Bridge: error
        Bridge->>App: error
    end
```

### Notification Flow

```mermaid
sequenceDiagram
    participant App as Client App
    participant Bridge as LspBridge
    participant Client as LspClient
    participant Server as LspServer
    participant Process as LspProcess
    participant LSP as LSP Server
    
    App->>Bridge: notify(method, params)
    Bridge->>Client: forward_notification()
    Client->>Server: handle_notification()
    Server->>Process: send_notification()
    Process->>LSP: JSON-RPC notification
    
    Note over LSP: No response expected
```

### Bidirectional Communication

```mermaid
sequenceDiagram
    participant App as Client App
    participant Bridge as LspBridge
    participant Server as LspServer
    participant LSP as LSP Server
    
    Note over App,LSP: Server-initiated notifications
    LSP->>Server: publishDiagnostics
    Server->>Bridge: forward_notification()
    Bridge->>App: diagnostic_notification()
    
    Note over App,LSP: Client-initiated requests
    App->>Bridge: completion_request()
    Bridge->>Server: forward_request()
    Server->>LSP: textDocument/completion
    LSP->>Server: completion_response
    Server->>Bridge: response
    Bridge->>App: completion_items
```

## Server Lifecycle Management

```mermaid
stateDiagram-v2
    [*] --> Stopped
    
    Stopped --> Starting : start_server()
    Starting --> Initializing : process_spawned
    Starting --> Crashed : spawn_failed
    
    Initializing --> Ready : initialize_success
    Initializing --> Crashed : initialize_failed
    Initializing --> Ready : capabilities_received
    
    Ready --> ShuttingDown : stop_server()
    Ready --> Crashed : process_died
    
    ShuttingDown --> Stopped : shutdown_complete
    ShuttingDown --> Crashed : shutdown_timeout
    
    Crashed --> Starting : restart_server()
    Crashed --> Stopped : give_up
    
    note right of Ready
        Server can handle
        requests and notifications
    end note
    
    note right of Crashed
        Automatic restart
        with exponential backoff
    end note
```

## Error Handling Flow

```mermaid
graph TD
    subgraph "Error Sources"
        E1[Process Spawn Error]
        E2[Communication Error]
        E3[Protocol Error]
        E4[Timeout Error]
        E5[Server Crash]
    end
    
    subgraph "Error Processing"
        EH[Error Handler]
        EH --> R1[Retry Logic]
        EH --> R2[Fallback Strategy]
        EH --> R3[Error Reporting]
    end
    
    subgraph "Recovery Actions"
        A1[Restart Server]
        A2[Switch to Backup]
        A3[Graceful Degradation]
        A4[Circuit Breaker]
    end
    
    E1 --> EH
    E2 --> EH
    E3 --> EH
    E4 --> EH
    E5 --> EH
    
    R1 --> A1
    R2 --> A2
    R2 --> A3
    R3 --> A4
    
    style EH fill:#ffebee
    style A1 fill:#e8f5e8
    style A2 fill:#e8f5e8
    style A3 fill:#fff3e0
    style A4 fill:#fce4ec
```

## Multi-Server Architecture

```mermaid
graph TB
    subgraph "Client Applications"
        App1[Code Editor]
        App2[IDE Plugin]
        App3[CLI Tool]
    end
    
    subgraph "LSPBridge Instance"
        Bridge[LspBridge]
        
        subgraph "Server Management"
            SM[Server Manager]
            LB[Load Balancer]
            HM[Health Monitor]
        end
        
        subgraph "Registered Servers"
            S1[rust-analyzer]
            S2[typescript-language-server]
            S3[pylsp]
            S4[gopls]
            S5[clangd]
        end
    end
    
    subgraph "Language Projects"
        P1[Rust Project]
        P2[TypeScript Project]
        P3[Python Project]
        P4[Go Project]
        P5[C++ Project]
    end
    
    App1 --> Bridge
    App2 --> Bridge
    App3 --> Bridge
    
    Bridge --> SM
    SM --> LB
    SM --> HM
    
    LB --> S1
    LB --> S2
    LB --> S3
    LB --> S4
    LB --> S5
    
    S1 --> P1
    S2 --> P2
    S3 --> P3
    S4 --> P4
    S5 --> P5
    
    HM -.-> S1
    HM -.-> S2
    HM -.-> S3
    HM -.-> S4
    HM -.-> S5
    
    style Bridge fill:#e1f5fe
    style SM fill:#f3e5f5
    style LB fill:#e8f5e8
    style HM fill:#fff3e0
```

## Data Flow Architecture

```mermaid
graph LR
    subgraph "Input Layer"
        I1[LSP Requests]
        I2[LSP Notifications]
        I3[Configuration]
    end
    
    subgraph "Processing Layer"
        P1[Message Validation]
        P2[Server Selection]
        P3[Protocol Translation]
        P4[Error Handling]
    end
    
    subgraph "Transport Layer"
        T1[JSON-RPC Serialization]
        T2[Stdin/Stdout Communication]
        T3[Process Management]
    end
    
    subgraph "Output Layer"
        O1[LSP Responses]
        O2[Error Messages]
        O3[Status Updates]
    end
    
    I1 --> P1
    I2 --> P1
    I3 --> P2
    
    P1 --> P2
    P2 --> P3
    P3 --> P4
    
    P4 --> T1
    T1 --> T2
    T2 --> T3
    
    T3 --> O1
    P4 --> O2
    T3 --> O3
    
    style P1 fill:#e8f5e8
    style P2 fill:#f3e5f5
    style P3 fill:#fff3e0
    style P4 fill:#ffebee
```

## Thread Safety Architecture

```mermaid
graph TB
    subgraph "Concurrent Access Points"
        T1[Request Thread 1]
        T2[Request Thread 2]
        T3[Notification Thread]
        T4[Monitoring Thread]
    end
    
    subgraph "Shared Data Structures"
        D1[DashMap<ServerId, LspServer>]
        D2[Arc<RwLock<Option<ServerId>>>]
        D3[Arc<RwLock<HashMap<ConfigId, Config>>>]
    end
    
    subgraph "Synchronization Primitives"
        S1[Arc for Shared Ownership]
        S2[RwLock for Reader/Writer Access]
        S3[DashMap for Concurrent HashMap]
        S4[Mutex for Exclusive Access]
    end
    
    T1 --> D1
    T2 --> D1
    T3 --> D2
    T4 --> D3
    
    D1 --> S3
    D2 --> S1
    D2 --> S2
    D3 --> S1
    D3 --> S2
    
    style D1 fill:#e1f5fe
    style D2 fill:#f3e5f5
    style D3 fill:#e8f5e8
    style S1 fill:#fff3e0
    style S2 fill:#fce4ec
    style S3 fill:#f1f8e9
    style S4 fill:#fdf2e9
```

---

## Diagram Legend

- **Blue boxes**: Main coordination components
- **Purple boxes**: Client/communication layers  
- **Orange boxes**: Server management components
- **Pink boxes**: Process and low-level components
- **Green boxes**: Successful operations/states
- **Red boxes**: Error conditions/handling
- **Dotted lines**: Monitoring/health check connections
- **Solid lines**: Direct communication paths

These diagrams provide a comprehensive visual understanding of the LSPBridge architecture, from high-level component organization to detailed protocol flows and error handling strategies.
