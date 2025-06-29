## Docklord

**Docklord** is a lightweight, high-performance container management and monitoring tool written in Rust. Designed for distributed environments, it provides real-time insights and seamless control over your Docker containers.

### Why Docklord?

- 🚀 **Instant Monitoring**: Track container status and events in real time via WebSocket.
- 🔗 **Flexible Deployment**: Connect to our public coordinator or self-host your entire stack.
- ⚡ **Blazing Fast**: Minimal resource consumption (\~13 MB images).
- 🔒 **Secure**: Unique `node_id` and `password` for each node.

---

## 🚀 Quick Start

Choose one of the following options to get started:

### 1. Connect to Public Server (Fastest)

```bash
COORDINATOR_ADDR=http://82.27.2.230:50051 \
  docker-compose up docklord-node
```

### 2. Self-Hosted All-in-One

```bash
git clone https://github.com/DaDaDaTheoryNow/docklord.git
cd docklord
docker-compose up docklord-self-hosted
```

### 3. Separate Coordinator + Node

```bash
# Start the coordinator:
docker-compose up docklord-coordinator

# In a new terminal, start the node:
docker-compose up docklord-node
```

### 4. Build and Run from Source

```bash
cargo run --release -- --type self-hosted
```

---

## 🌐 Using the Public Server

1. **Run your node**

   ```bash
   COORDINATOR_ADDR=http://82.27.2.230:50051 \
     docker-compose up docklord-node
   ```

2. **Fetch containers via REST API**

   ```bash
   curl "http://82.27.2.230:3000/api/containers?node_id=YOUR_NODE_ID&password=YOUR_PASSWORD"
   ```

3. **Subscribe to real-time updates**

   ```javascript
   const ws = new WebSocket(
     "ws://82.27.2.230:3000/ws?node_id=YOUR_NODE_ID&password=YOUR_PASSWORD"
   );
   ws.onmessage = (event) =>
     console.log("Container event:", JSON.parse(event.data));
   ```

---

## ⚙️ Environment Variables

| Variable           | Default                             | Description                   |
| ------------------ | ----------------------------------- | ----------------------------- |
| `API_PORT`         | `3000`                              | Port for REST API & WebSocket |
| `GRPC_PORT`        | `50051`                             | Port for gRPC communications  |
| `COORDINATOR_ADDR` | `http://host.docker.internal:50051` | Coordinator URL for nodes     |
| `RUST_LOG`         | `info`                              | Logging level                 |

**Examples:**

```bash
# Public server:
COORDINATOR_ADDR=http://82.27.2.230:50051 \
  docker-compose up docklord-node

# Custom ports:
API_PORT=8080 GRPC_PORT=50052 \
  docker-compose up docklord-self-hosted

# Using a .env file:
echo "COORDINATOR_ADDR=http://82.27.2.230:50051" > .env
docker-compose up docklord-node
```

---

## 📁 Project Layout

```
docklord/
├── crates/
│   ├── bin/docklord-runner/     # Main executable
│   ├── coordinator/             # Coordinator service
│   ├── node/                    # Node service
│   └── proto/                   # Protobuf definitions
├── docker-compose.yml           # Deployment configurations
├── Dockerfile                   # Multi-stage build
└── README.md                    # This file
```

---

## 🚢 Production Deployment

1. **Clone to `/opt/apps`** (or your preferred directory):

   ```bash
   sudo mkdir -p /opt/apps && cd /opt/apps
   sudo git clone https://github.com/DaDaDaTheoryNow/docklord.git
   cd docklord
   sudo chown -R $USER:$USER .
   ```

2. **Set up `.env` and run**:

   ```bash
   cp env.example .env
   # Edit .env as needed
   docker-compose up -d
   ```

3. **Enable Docker on boot** (Systemd example):

   ```bash
   sudo systemctl enable docker
   sudo systemctl start docker
   ```

---

## 🆘 Troubleshooting

### Node Fails to Connect

```bash
docker-compose ps
docker-compose logs docklord-coordinator
```

### Port Conflicts

```bash
API_PORT=8080 GRPC_PORT=50052 \
  docker-compose up docklord-self-hosted
```

### Docker Socket Permissions

```bash
sudo chmod 666 /var/run/docker.sock
```

---

## 📄 License

MIT © [DaDaDaTheoryNow](https://github.com/DaDaDaTheoryNow)
