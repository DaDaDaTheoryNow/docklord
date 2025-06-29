# Docklord

> **Docklord** — Container Management and Monitoring Tool  
> ⚡️ Blazingly fast, lightweight, and flexible system written in Rust for real-time monitoring and management of Docker containers across distributed environments.

---

## 🚀 Overview

- **Coordinator** is a public server (with a white/public IP) — all REST & WebSocket commands from users go to it.
- **Node (client)** runs on your device/server, generates a `node_id` & `password` so you (or your app) can later connect and manage containers securely via the Coordinator.
- **Self-Hosted** combines both Coordinator and Node in a single process for easy deployment.
- **Blazingly fast**: async-first architecture powered by Tokio and Rust.
- **Lightweight**: minimal dependencies, low resource usage.
- **Flexible**: modular design, supports gRPC, REST, and WebSocket APIs.

---

## 🏗️ Architecture

```
+-------------------+         +-------------------+
|   Coordinator     | <-----> |      Node(s)      |
|  (public server)  |         |   (your device)   |
|-------------------|         |-------------------|
| gRPC server       |         | gRPC client       |
| REST API          |         | Docker API        |
| WebSocket server  |         | Container watcher |
| Auth, pending map |         |                   |
+-------------------+         +-------------------+
        ^
        |
        |
   +-----------+
   |   User    |
   | (Web/Mob) |
   +-----------+
```

- **Coordinator**: The public-facing server (with a white IP), aggregates data, manages clients, exposes APIs, and acts as a bridge between users and nodes. All REST & WebSocket requests from users go here.
- **Node (client)**: Runs on your device or server, generates a unique `node_id` and `password` for secure access. Watches Docker events and reports them to the Coordinator.
- **Self-Hosted**: Combines Coordinator and Node in a single process for simplified deployment.
- **User**: Connects to the Coordinator (via REST/WebSocket), can monitor and control containers on nodes using a web or mobile interface.

---

## ⚙️ Key Features

- **gRPC**: Async server and client for communication between Coordinator and Nodes.
- **REST API**: Endpoints for fetching container info (e.g., `/api/containers`).
- **WebSocket**: Real-time streaming of container updates to clients.
- **Authentication**: Every client is identified by a `node_id` + `password` pair.
- **Pending responses**: Map for correlating async requests and responses (DashMap, oneshot).
- **Modular**: All logic is split into independent crates (lib-coordinator-ws, lib-coordinator-rest, lib-node-containers, etc).
- **Protocol**: Strictly typed proto files for all inter-component messages.

---

## 🔑 How It Works

1. **Node** watches Docker events (start/stop/create/destroy) and sends them to the Coordinator via gRPC.
2. **Coordinator** keeps track of clients, authenticates them, and dispatches commands/requests.
3. **REST and WebSocket** APIs let external clients fetch up-to-date container info in real time.
4. **Pending responses**: Each REST request creates an entry in the map; the Node's gRPC response resolves the oneshot channel.
5. **Security**: All actions require authentication; data is isolated per node (`node_id` + password).

---

## 📦 What's Implemented

The following features are already implemented:

- ✔️ Async gRPC server/client (tonic)
- ✔️ REST API for container listing
- ✔️ WebSocket server for real-time updates
- ✔️ Modular architecture (separate crates for core, grpc, ws, rest, node)
- ✔️ Docker API support via bollard
- ✔️ Client authentication (`node_id` + password)
- ✔️ Pending responses (DashMap + oneshot for REST/gRPC correlation)
- ✔️ Protocol via proto files
- ✔️ Logging with tracing
- ✔️ Error and timeout handling
- ✔️ Clean separation of business logic and transport

---

## 🐳 Docker Deployment

### Quick Start with Docker

**Build the image:**

```sh
docker build -t docklord .
```

**Run self-hosted mode:**

```sh
docker run -d \
  --name docklord \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e API_PORT=3000 \
  -e GRPC_PORT=50051 \
  -p 3000:3000 \
  -p 50051:50051 \
  docklord --type self-hosted
```

**Run with custom parameters:**

```sh
docker run -d \
  --name docklord \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e API_PORT=8080 \
  -e GRPC_PORT=50052 \
  -p 8080:3000 \
  -p 50052:50051 \
  docklord --type self-hosted \
    --node-id mynode \
    --password mysecret123
```

### Docker Compose (Recommended)

**Setup environment (optional):**

```sh
# Copy example config
cp env.example .env

# Edit ports if needed
# API_PORT=3000
# GRPC_PORT=50051
```

**Self-hosted mode:**

```sh
docker-compose up docklord-self-hosted
```

**With custom ports:**

```sh
API_PORT=8080 GRPC_PORT=50052 docker-compose up docklord-self-hosted
```

**Separate coordinator and node:**

```sh
# Start coordinator
docker-compose up docklord-coordinator

# In another terminal, start node
docker-compose up docklord-node
```

**Deploy on different servers:**

```sh
# Server 1: Start coordinator
docker-compose up docklord-coordinator

# Server 2: Start node (replace with actual coordinator IP)
COORDINATOR_ADDR=http://192.168.1.100:50051 docker-compose up docklord-node

# Or via .env file on Server 2:
# COORDINATOR_ADDR=http://192.168.1.100:50051
docker-compose up docklord-node
```

**All services:**

```sh
docker-compose up
```

### Docker Socket Access

**Important:** Node services need access to Docker socket to monitor containers:

```sh
# For manual Docker run
docker run -d \
  --name docklord \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -p 3000:3000 \
  -p 50051:50051 \
  docklord --type self-hosted
```

**Docker Compose** automatically includes Docker socket access for node services.

### Port Mapping

- **3000** - API port (REST + WebSocket) - configurable via `API_PORT`
- **50051** - gRPC port (node-coordinator communication) - configurable via `GRPC_PORT`

---

## 🔧 Environment Variables

Docklord supports configuration via environment variables. You can set these in your `.env` file or pass them directly to Docker:

### Available Variables

| Variable           | Default                  | Description                                               |
| ------------------ | ------------------------ | --------------------------------------------------------- |
| `API_PORT`         | `3000`                   | Port for REST API and WebSocket server                    |
| `GRPC_PORT`        | `50051`                  | Port for gRPC communication between nodes and coordinator |
| `COORDINATOR_ADDR` | `http://localhost:50051` | Coordinator address for node connections                  |
| `RUST_LOG`         | `info`                   | Logging level (debug, info, warn, error)                  |

### Example Usage

**Using .env file:**

```bash
# Copy example config
cp env.example .env

# Edit .env file
API_PORT=8080
GRPC_PORT=50052
COORDINATOR_ADDR=http://my-coordinator:50051
RUST_LOG=debug
```

**Using Docker with environment variables:**

```bash
docker run -d \
  --name docklord \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e API_PORT=8080 \
  -e GRPC_PORT=50052 \
  -p 8080:3000 \
  -p 50052:50051 \
  docklord --type self-hosted
```

**Using Docker Compose with custom ports:**

```bash
API_PORT=8080 GRPC_PORT=50052 docker-compose up docklord-self-hosted
```

### Priority Order

Configuration values are read in the following priority order:

1. Command line arguments (highest priority)
2. Environment variables
3. Default values (lowest priority)

This means you can override environment variables with command line arguments if needed:

```bash
# Environment variable will be ignored, CLI argument takes precedence
API_PORT=8080 docker run docklord --type coordinator --api-port 9000
```

---

## 🛠️ Getting Started

### Prerequisites

1. **Install Rust** (no nightly required):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install Docker** (for container monitoring):

   ```sh
   # Ubuntu/Debian
   sudo apt-get install docker.io
   sudo systemctl start docker
   sudo usermod -aG docker $USER

   # macOS
   brew install docker
   ```

3. **Clone and build**:
   ```sh
   git clone <repository-url>
   cd docklord
   cargo build --release
   ```

### 🚀 Running Modes

#### 1. Self-Hosted Mode (Recommended for Development)

**Quick start with defaults:**

```sh
cargo run --release -- --type self-hosted
```

**With custom parameters:**

```sh
cargo run --release -- --type self-hosted \
  --grpc-port 50051 \
  --api-port 3000 \
  --node-id mynode \
  --password mysecret123
```

**Auto-generated credentials (recommended):**

```sh
cargo run --release -- --type self-hosted
# Node ID and password will be automatically generated and displayed
```

**What this does:**

- Starts Coordinator on `0.0.0.0:50051` (gRPC) and `0.0.0.0:3000` (API)
- Starts Node that connects to `localhost:50051`
- Uses provided `node_id` and `password` for authentication (or generates secure ones)

#### 2. Coordinator Only

**Default ports:**

```sh
cargo run --release -- --type coordinator
```

**Custom ports:**

```sh
cargo run --release -- --type coordinator \
  --grpc-port 50052 \
  --api-port 3001
```

#### 3. Node Only

**Connect to remote coordinator:**

```sh
cargo run --release -- --type node \
  --coordinator-addr http://192.168.1.100:50051 \
  --node-id mynode \
  --password mysecret123
```

**Connect to local coordinator:**

```sh
cargo run --release -- --type node \
  --coordinator-addr http://localhost:50051 \
  --node-id mynode \
  --password mysecret123
```

### 📋 All Available CLI Options

```sh
cargo run --release -- --help
```

**Options:**

- `--type`: Launch type (`node`, `coordinator`, `self-hosted`)
- `--grpc-port`: gRPC port for coordinator (default: 50051)
- `--api-port`: API port for coordinator (default: 3000)
- `--coordinator-addr`: Coordinator gRPC address (default: http://localhost:50051)
- `--node-id`: Node ID (auto-generated if not specified)
- `--password`: Node password (auto-generated if not specified)

---

## 📚 Example Usage

### Testing with Self-Hosted Mode

1. **Start the service with auto-generated credentials:**

   ```sh
   # Default ports
   docker-compose up docklord-self-hosted

   # Or with custom ports
   API_PORT=8080 GRPC_PORT=50052 docker-compose up docklord-self-hosted
   ```

2. **Start with custom credentials:**

   ```sh
   docker-compose run docklord-self-hosted \
     -- --type self-hosted \
     --node-id testnode \
     --password testpass
   ```

3. **Test REST API (use credentials from step 1 or 2):**

   ```sh
   # Default ports
   curl "http://localhost:3000/api/containers?node_id=GENERATED_ID&password=GENERATED_PASSWORD"

   # Custom ports
   curl "http://localhost:8080/api/containers?node_id=GENERATED_ID&password=GENERATED_PASSWORD"
   ```

4. **Test WebSocket (install wscat first):**

   ```sh
   npm install -g wscat

   # Default ports
   wscat -c "ws://localhost:3000/observe-containers?node_id=GENERATED_ID&password=GENERATED_PASSWORD"

   # Custom ports
   wscat -c "ws://localhost:8080/observe-containers?node_id=GENERATED_ID&password=GENERATED_PASSWORD"
   ```

### Production Deployment

**For production, use separate Coordinator and Node:**

1. **Start Coordinator on public server:**

   ```sh
   cargo run --release -- --type coordinator \
     --grpc-port 50051 \
     --api-port 3000
   ```

2. **Start Node on your device:**

   ```sh
   cargo run --release -- --type node \
     --coordinator-addr http://your-public-ip:50051 \
     --node-id production-node \
     --password secure-password-here
   ```

3. **Access from anywhere:**
   ```sh
   curl "http://your-public-ip:3000/api/containers?node_id=production-node&password=secure-password-here"
   ```

---

## 🔧 Development

**Build in debug mode:**

```sh
cargo build
cargo run -- --type self-hosted
```

**Run tests:**

```sh
cargo test
```

**Check for issues:**

```sh
cargo clippy
cargo fmt
```

---

## 📝 TODO / Roadmap

- [ ] Expand REST API (container management)
- [ ] Mobile/WEB UI for monitoring
- [ ] Prometheus/Grafana integration
- [ ] Proto/gRPC documentation
- [ ] Docker Compose support

---

## License

MIT
