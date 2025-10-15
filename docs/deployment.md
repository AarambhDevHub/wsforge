# Deployment Guide

This guide covers deploying WsForge WebSocket applications to production environments.

## Table of Contents

- [Building for Production](#building-for-production)
- [Docker Deployment](#docker-deployment)
- [Cloud Platforms](#cloud-platforms)
- [Reverse Proxy Setup](#reverse-proxy-setup)
- [TLS/SSL Configuration](#tlsssl-configuration)
- [Systemd Service](#systemd-service)
- [Environment Configuration](#environment-configuration)
- [Monitoring & Logging](#monitoring--logging)
- [Load Balancing](#load-balancing)
- [Security Best Practices](#security-best-practices)
- [Performance Tuning](#performance-tuning)

## Building for Production

### Optimized Build

Build with release optimizations:

```
cargo build --release
```

The binary will be in `target/release/`.

### Cargo.toml Optimizations

Add production optimizations to `Cargo.toml`:

```
[profile.release]
opt-level = 3          # Maximum optimizations
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
strip = true           # Strip symbols from binary
panic = 'abort'        # Smaller binary size
```

### Binary Size Reduction

For smaller binaries:

```
[profile.release]
opt-level = 'z'        # Optimize for size
lto = true
codegen-units = 1
strip = true
```

Build size can be further reduced:

```
cargo install cargo-bloat
cargo bloat --release
```

## Docker Deployment

### Dockerfile

Create an optimized multi-stage Dockerfile:

```
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY wsforge ./wsforge
COPY wsforge-core ./wsforge-core
COPY wsforge-macros ./wsforge-macros

# Build dependencies (cached layer)
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/my-app .

# Copy static files if needed
COPY ./static ./static

# Expose port
EXPOSE 8080

# Run the application
CMD ["./my-app"]
```

### Docker Compose

`docker-compose.yml`:

```
version: '3.8'

services:
  websocket-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - HOST=0.0.0.0
      - PORT=8080
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### Building and Running

```
# Build image
docker build -t wsforge-app .

# Run container
docker run -d \
  -p 8080:8080 \
  -e RUST_LOG=info \
  --name websocket-server \
  wsforge-app

# With docker-compose
docker-compose up -d
```

## Cloud Platforms

### AWS EC2

1. **Launch EC2 Instance**:
   - Ubuntu 22.04 LTS
   - t3.medium or larger
   - Open port 8080 (or 443 for WSS)

2. **Deploy Application**:

```
# SSH into instance
ssh ubuntu@your-ec2-ip

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/your-repo/wsforge-app.git
cd wsforge-app
cargo build --release

# Run with systemd (see below)
```

### Heroku

`Procfile`:

```
web: ./target/release/my-app
```

`rust-toolchain.toml`:

```
[toolchain]
channel = "1.75.0"
```

Deploy:

```
heroku create my-websocket-app
heroku buildpacks:set emk/rust
git push heroku main
```

Heroku requires binding to `$PORT`:

```
let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
let addr = format!("0.0.0.0:{}", port);
router.listen(&addr).await?;
```

### DigitalOcean App Platform

`app.yaml`:

```
name: wsforge-app
services:
  - name: web
    github:
      repo: your-username/your-repo
      branch: main
    build_command: cargo build --release
    run_command: ./target/release/my-app
    http_port: 8080
    instance_size_slug: basic-xs
```

### Railway

Railway automatically detects Rust projects:

```
railway login
railway init
railway up
```

### Fly.io

`fly.toml`:

```
app = "my-wsforge-app"

[build]
  builder = "paketobuildpacks/builder:base"

[[services]]
  internal_port = 8080
  protocol = "tcp"

  [[services.ports]]
    handlers = ["http"]
    port = 80

  [[services.ports]]
    handlers = ["tls", "http"]
    port = 443
```

Deploy:

```
fly launch
fly deploy
```

## Reverse Proxy Setup

### Nginx

`/etc/nginx/sites-available/websocket`:

```
upstream websocket_backend {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://websocket_backend;
        proxy_http_version 1.1;

        # WebSocket headers
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Standard proxy headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 7d;
        proxy_send_timeout 7d;
        proxy_read_timeout 7d;
    }
}
```

Enable and restart:

```
sudo ln -s /etc/nginx/sites-available/websocket /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

### Caddy

`Caddyfile`:

```
your-domain.com {
    reverse_proxy localhost:8080
}
```

Caddy automatically handles WebSocket upgrades and SSL.

## TLS/SSL Configuration

### Let's Encrypt with Certbot

```
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d your-domain.com

# Auto-renewal (certbot sets this up automatically)
sudo certbot renew --dry-run
```

Updated Nginx config for SSL:

```
server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;

    # SSL security settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}
```

## Systemd Service

Create `/etc/systemd/system/wsforge-app.service`:

```
[Unit]
Description=WsForge WebSocket Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/wsforge-app
Environment="RUST_LOG=info"
Environment="HOST=127.0.0.1"
Environment="PORT=8080"
ExecStart=/opt/wsforge-app/target/release/my-app
Restart=always
RestartSec=10

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/wsforge-app/logs

[Install]
WantedBy=multi-user.target
```

Enable and start:

```
sudo systemctl daemon-reload
sudo systemctl enable wsforge-app
sudo systemctl start wsforge-app
sudo systemctl status wsforge-app
```

View logs:

```
sudo journalctl -u wsforge-app -f
```

## Environment Configuration

### Environment Variables

Create `.env` file (don't commit to Git):

```
RUST_LOG=info
HOST=0.0.0.0
PORT=8080
DATABASE_URL=postgres://user:pass@localhost/dbname
REDIS_URL=redis://localhost:6379
MAX_CONNECTIONS=10000
```

Load in application:

```
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok(); // Load .env file

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{}:{}", host, port);

    router.listen(&addr).await?;
    Ok(())
}
```

Add to `Cargo.toml`:

```
[dependencies]
dotenv = "0.15"
```

## Monitoring & Logging

### Structured Logging

```
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Your application code
}
```

### Prometheus Metrics

Add metrics collection:

```
[dependencies]
prometheus = "0.13"
```

```
use prometheus::{Counter, Encoder, Registry, TextEncoder};

lazy_static::lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref CONNECTIONS: Counter = Counter::new(
        "websocket_connections_total",
        "Total WebSocket connections"
    ).unwrap();
    static ref MESSAGES: Counter = Counter::new(
        "websocket_messages_total",
        "Total messages processed"
    ).unwrap();
}

// Register metrics
REGISTRY.register(Box::new(CONNECTIONS.clone())).unwrap();
REGISTRY.register(Box::new(MESSAGES.clone())).unwrap();

// Metrics endpoint
async fn metrics_handler() -> Result<String> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(String::from_utf8(buffer).unwrap())
}
```

### Health Check Endpoint

```
async fn health_handler() -> Result<String> {
    Ok("OK".to_string())
}

let router = Router::new()
    .route("/health", handler(health_handler))
    .route("/metrics", handler(metrics_handler))
    .default_handler(handler(ws_handler));
```

## Load Balancing

### Nginx Load Balancing

```
upstream websocket_backend {
    ip_hash; # Sticky sessions
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
    server 127.0.0.1:8083;
}

server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://websocket_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

### HAProxy Configuration

```
frontend websocket_front
    bind *:80
    default_backend websocket_back

backend websocket_back
    balance source # Sticky sessions
    option http-server-close
    option forwardfor
    server ws1 127.0.0.1:8081 check
    server ws2 127.0.0.1:8082 check
    server ws3 127.0.0.1:8083 check
```

### Running Multiple Instances

```
# Instance 1
PORT=8081 ./target/release/my-app &

# Instance 2
PORT=8082 ./target/release/my-app &

# Instance 3
PORT=8083 ./target/release/my-app &
```

## Security Best Practices

### Rate Limiting

Implement at application level:

```
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    async fn check(&self, client_id: &str) -> bool {
        let mut requests = self.requests.write().await;
        let now = Instant::now();

        let client_requests = requests.entry(client_id.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests
        client_requests.retain(|&time| now.duration_since(time) < self.window);

        if client_requests.len() < self.max_requests {
            client_requests.push(now);
            true
        } else {
            false
        }
    }
}
```

### Input Validation

```
async fn validate_message(msg: &Message) -> Result<()> {
    // Size limit
    if msg.as_bytes().len() > 1024 * 1024 { // 1MB
        return Err(Error::custom("Message too large"));
    }

    // Text validation
    if msg.is_text() {
        let text = msg.as_text().ok_or(Error::custom("Invalid UTF-8"))?;
        if text.is_empty() {
            return Err(Error::custom("Empty message"));
        }
    }

    Ok(())
}
```

### CORS Configuration

If serving web clients:

```
add_header Access-Control-Allow-Origin "https://your-frontend.com" always;
add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always;
add_header Access-Control-Allow-Headers "Authorization, Content-Type" always;
```

## Performance Tuning

### OS-Level Tuning

Increase file descriptor limits:

```
# /etc/security/limits.conf
* soft nofile 65535
* hard nofile 65535
```

Kernel parameters (`/etc/sysctl.conf`):

```
# Increase connection tracking
net.netfilter.nf_conntrack_max = 1000000

# TCP tuning
net.ipv4.tcp_max_syn_backlog = 8192
net.core.somaxconn = 8192
net.core.netdev_max_backlog = 5000

# Buffer sizes
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
```

Apply changes:

```
sudo sysctl -p
```

### Application Tuning

Tokio runtime configuration:

```
#[tokio::main(worker_threads = 8)]
async fn main() -> Result<()> {
    // Application code
}
```

Or with custom runtime:

```
use tokio::runtime::Builder;

fn main() -> Result<()> {
    let runtime = Builder::new_multi_thread()
        .worker_threads(8)
        .thread_name("wsforge-worker")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_all()
        .build()?;

    runtime.block_on(async {
        // Your async code
    })
}
```

### Connection Limits

```
struct Config {
    max_connections: usize,
}

async fn connection_guard(
    conn_count: Arc<AtomicUsize>,
    max: usize,
) -> Result<ConnectionGuard> {
    let current = conn_count.fetch_add(1, Ordering::SeqCst);
    if current >= max {
        conn_count.fetch_sub(1, Ordering::SeqCst);
        return Err(Error::custom("Connection limit reached"));
    }
    Ok(ConnectionGuard { conn_count })
}

struct ConnectionGuard {
    conn_count: Arc<AtomicUsize>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.conn_count.fetch_sub(1, Ordering::SeqCst);
    }
}
```

## Troubleshooting

### Common Issues

**Connection drops:**
- Check firewall settings
- Verify timeout configurations
- Monitor system resources

**High memory usage:**
- Profile with `cargo-flamegraph`
- Check for connection leaks
- Review buffer sizes

**Poor performance:**
- Enable release optimizations
- Check network latency
- Review OS-level limits

### Debug Mode

Enable detailed logging:

```
RUST_LOG=debug ./target/release/my-app
```

## Checklist

Pre-deployment checklist:

- [ ] Build with `--release` flag
- [ ] Configure environment variables
- [ ] Set up reverse proxy (Nginx/Caddy)
- [ ] Enable TLS/SSL
- [ ] Configure systemd service
- [ ] Set up monitoring and logging
- [ ] Configure health checks
- [ ] Test failover scenarios
- [ ] Document deployment process
- [ ] Set up automated backups
- [ ] Configure log rotation
- [ ] Test load balancing
- [ ] Security audit complete

## Next Steps

- [Monitoring Guide](monitoring.md)
- [Security Best Practices](security.md)
- [Performance Tuning](performance.md)
- [Troubleshooting](troubleshooting.md)
