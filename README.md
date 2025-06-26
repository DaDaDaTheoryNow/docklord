# Docklord

> **Docklord** — Container Management and Monitoring Tool  
> ⚡️ Blazingly fast, lightweight, and flexible system written in Rust for real-time monitoring and management of Docker containers across distributed environments.

---

## 🚀 Overview

- **Coordinator** is a public server (with a white/public IP) — all REST & WebSocket commands from users go to it.
- **Node (client)** runs on your device/server, generates a `node_id` & `password` so you (or your app) can later connect and manage containers securely via the Coordinator.
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

## 🛠️ Getting Started (dev)

1. Install Rust (no nightly required):
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. Clone the repo and build:
   ```sh
   git clone ...
   cd docklord
   cargo build --release
   ```
3. Run the Coordinator and Node (with explicit type):
   ```sh
   cargo run --release -- --type coordinator
   cargo run --release -- --type node
   ```
4. Use REST or WebSocket to fetch container data.

---

## 📚 Example Requests

**REST (using curl):**

```sh
curl "http://localhost:3000/api/containers?node_id=YOUR_ID&password=YOUR_PASSWORD"
```

**WebSocket (using wscat):**

```sh
wscat -c "ws://localhost:3000/observe-containers?node_id=YOUR_ID&password=YOUR_PASSWORD"
```

---

## 📝 TODO / Roadmap

- [ ] Expand REST API (container management)
- [ ] Mobile/WEB UI for monitoring
- [ ] Prometheus/Grafana integration
- [ ] Proto/gRPC documentation

---

## License

MIT
