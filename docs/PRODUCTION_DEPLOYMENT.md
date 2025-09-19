# Production Deployment Guide

## 🚀 **Leptos WebSocket Pro - Production Deployment**

This guide provides comprehensive instructions for deploying Leptos WebSocket Pro in production environments.

## 📋 **Prerequisites**

### System Requirements

- **Rust**: 1.70+ (stable)
- **Memory**: 4GB+ RAM (8GB+ recommended)
- **CPU**: 2+ cores (4+ cores recommended)
- **Storage**: 10GB+ available space
- **Network**: Stable internet connection

### Dependencies

- **Tokio**: Async runtime
- **Leptos**: Web framework
- **Serde**: Serialization
- **Rkyv**: Zero-copy serialization

## 🏗️ **Architecture Overview**

### Production Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer │    │   WebSocket     │    │   Database      │
│   (nginx/HAProxy)│────│   Pro Server    │────│   (PostgreSQL)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Monitoring    │    │   Redis Cache   │    │   Message Queue │
│   (Prometheus)  │    │   (Optional)    │    │   (Optional)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Component Responsibilities

1. **Load Balancer**: Distributes traffic across multiple instances
2. **WebSocket Pro Server**: Handles WebSocket connections and RPC calls
3. **Database**: Stores persistent data and session information
4. **Cache**: Improves performance with in-memory caching
5. **Message Queue**: Handles asynchronous message processing
6. **Monitoring**: Tracks performance and health metrics

## 🔧 **Configuration**

### Environment Variables

```bash
# Server Configuration
WS_PRO_HOST=0.0.0.0
WS_PRO_PORT=8080
WS_PRO_WORKERS=4

# Database Configuration
DATABASE_URL=postgresql://user:password@localhost:5432/leptos_ws_pro
DATABASE_POOL_SIZE=20

# Redis Configuration (Optional)
REDIS_URL=redis://localhost:6379
REDIS_POOL_SIZE=10

# Security Configuration
JWT_SECRET=your-super-secret-jwt-key
RATE_LIMIT_REQUESTS_PER_MINUTE=1000
RATE_LIMIT_BURST_SIZE=100

# Performance Configuration
CONNECTION_POOL_SIZE=1000
MESSAGE_BATCH_SIZE=100
MESSAGE_BATCH_TIMEOUT_MS=10
ENABLE_COMPRESSION=true
ENABLE_ZERO_COPY=true

# Monitoring Configuration
ENABLE_METRICS=true
METRICS_PORT=9090
LOG_LEVEL=info
```

### Configuration File

```toml
# config/production.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4
max_connections = 10000

[database]
url = "postgresql://user:password@localhost:5432/leptos_ws_pro"
pool_size = 20
timeout = 30

[redis]
url = "redis://localhost:6379"
pool_size = 10
timeout = 5

[security]
jwt_secret = "your-super-secret-jwt-key"
rate_limit_requests_per_minute = 1000
rate_limit_burst_size = 100
enable_csrf_protection = true
enable_input_validation = true

[performance]
connection_pool_size = 1000
message_batch_size = 100
message_batch_timeout_ms = 10
enable_compression = true
enable_zero_copy = true
memory_limit_mb = 1024
cpu_throttle_threshold = 0.8

[monitoring]
enable_metrics = true
metrics_port = 9090
log_level = "info"
enable_health_checks = true
```

## 🚀 **Deployment Methods**

### 1. Docker Deployment

#### Dockerfile

```dockerfile
FROM rust:1.70-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/leptos-ws-pro .
COPY --from=builder /app/config ./config

EXPOSE 8080
CMD ["./leptos-ws-pro"]
```

#### Docker Compose

```yaml
version: "3.8"

services:
  leptos-ws-pro:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://user:password@postgres:5432/leptos_ws_pro
      - REDIS_URL=redis://redis:6379
    depends_on:
      - postgres
      - redis
    restart: unless-stopped

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=leptos_ws_pro
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - leptos-ws-pro
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
```

### 2. Kubernetes Deployment

#### Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: leptos-ws-pro
  labels:
    app: leptos-ws-pro
spec:
  replicas: 3
  selector:
    matchLabels:
      app: leptos-ws-pro
  template:
    metadata:
      labels:
        app: leptos-ws-pro
    spec:
      containers:
        - name: leptos-ws-pro
          image: leptos-ws-pro:latest
          ports:
            - containerPort: 8080
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: leptos-ws-pro-secrets
                  key: database-url
            - name: JWT_SECRET
              valueFrom:
                secretKeyRef:
                  name: leptos-ws-pro-secrets
                  key: jwt-secret
          resources:
            requests:
              memory: "512Mi"
              cpu: "250m"
            limits:
              memory: "1Gi"
              cpu: "500m"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
```

#### Service Manifest

```yaml
apiVersion: v1
kind: Service
metadata:
  name: leptos-ws-pro-service
spec:
  selector:
    app: leptos-ws-pro
  ports:
    - protocol: TCP
      port: 80
      targetPort: 8080
  type: LoadBalancer
```

### 3. Systemd Service

#### Service File

```ini
[Unit]
Description=Leptos WebSocket Pro
After=network.target

[Service]
Type=simple
User=leptos-ws-pro
Group=leptos-ws-pro
WorkingDirectory=/opt/leptos-ws-pro
ExecStart=/opt/leptos-ws-pro/leptos-ws-pro
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=DATABASE_URL=postgresql://user:password@localhost:5432/leptos_ws_pro

[Install]
WantedBy=multi-user.target
```

## 🔒 **Security Configuration**

### SSL/TLS Setup

#### Nginx Configuration

```nginx
upstream leptos_ws_pro {
    server 127.0.0.1:8080;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    ssl_prefer_server_ciphers off;

    location / {
        proxy_pass http://leptos_ws_pro;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
    }
}
```

### Firewall Configuration

```bash
# UFW Configuration
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow 8080/tcp
ufw enable

# iptables Configuration
iptables -A INPUT -p tcp --dport 22 -j ACCEPT
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
iptables -A INPUT -j DROP
```

## 📊 **Monitoring & Observability**

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: "leptos-ws-pro"
    static_configs:
      - targets: ["localhost:9090"]
    metrics_path: /metrics
    scrape_interval: 5s
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Leptos WebSocket Pro",
    "panels": [
      {
        "title": "Active Connections",
        "type": "stat",
        "targets": [
          {
            "expr": "leptos_ws_pro_active_connections"
          }
        ]
      },
      {
        "title": "Message Throughput",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(leptos_ws_pro_messages_total[5m])"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(leptos_ws_pro_errors_total[5m])"
          }
        ]
      }
    ]
  }
}
```

### Health Checks

```rust
// Health check endpoints
#[get("/health")]
async fn health_check() -> impl Responder {
    // Check database connection
    // Check Redis connection
    // Check system resources
    "OK"
}

#[get("/ready")]
async fn readiness_check() -> impl Responder {
    // Check if service is ready to accept traffic
    "READY"
}
```

## 🔧 **Performance Tuning**

### System Optimization

```bash
# Increase file descriptor limits
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# Optimize network settings
echo "net.core.somaxconn = 65536" >> /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 65536" >> /etc/sysctl.conf
echo "net.core.netdev_max_backlog = 5000" >> /etc/sysctl.conf
sysctl -p
```

### Application Tuning

```rust
// Performance configuration
let config = PerformanceConfig {
    enable_connection_pooling: true,
    max_pool_size: 1000,
    enable_message_batching: true,
    batch_size: 100,
    batch_timeout: Duration::from_millis(10),
    enable_caching: true,
    cache_size: 10000,
    cache_ttl: Duration::from_secs(300),
    enable_compression: true,
    compression_threshold: 1024,
    enable_metrics: true,
};
```

## 🚨 **Troubleshooting**

### Common Issues

#### High Memory Usage

```bash
# Check memory usage
ps aux | grep leptos-ws-pro
free -h

# Monitor memory leaks
valgrind --tool=memcheck ./leptos-ws-pro
```

#### Connection Issues

```bash
# Check network connectivity
netstat -tulpn | grep 8080
ss -tulpn | grep 8080

# Test WebSocket connection
wscat -c ws://localhost:8080
```

#### Performance Issues

```bash
# Monitor CPU usage
top -p $(pgrep leptos-ws-pro)
htop -p $(pgrep leptos-ws-pro)

# Profile performance
perf record -p $(pgrep leptos-ws-pro)
perf report
```

### Log Analysis

```bash
# View logs
journalctl -u leptos-ws-pro -f

# Filter error logs
journalctl -u leptos-ws-pro | grep ERROR

# Monitor real-time logs
tail -f /var/log/leptos-ws-pro.log
```

## 🔄 **Backup & Recovery**

### Database Backup

```bash
# Create backup
pg_dump -h localhost -U user -d leptos_ws_pro > backup.sql

# Restore backup
psql -h localhost -U user -d leptos_ws_pro < backup.sql
```

### Configuration Backup

```bash
# Backup configuration
tar -czf config-backup.tar.gz config/
cp config-backup.tar.gz /backup/location/
```

## 📈 **Scaling**

### Horizontal Scaling

```yaml
# Kubernetes HPA
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: leptos-ws-pro-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: leptos-ws-pro
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

### Load Balancing

```nginx
upstream leptos_ws_pro {
    least_conn;
    server 127.0.0.1:8080 weight=1;
    server 127.0.0.1:8081 weight=1;
    server 127.0.0.1:8082 weight=1;
}
```

## 🎯 **Best Practices**

### Security

- Use strong JWT secrets
- Enable rate limiting
- Implement input validation
- Use HTTPS in production
- Regular security audits

### Performance

- Monitor resource usage
- Use connection pooling
- Enable message batching
- Implement caching
- Regular performance testing

### Reliability

- Implement health checks
- Use circuit breakers
- Monitor error rates
- Implement retry logic
- Regular backups

### Monitoring

- Set up comprehensive monitoring
- Configure alerting
- Monitor key metrics
- Regular log analysis
- Performance benchmarking

---

**This production deployment guide ensures your Leptos WebSocket Pro instance is secure, performant, and reliable in production environments.** 🚀
