# Docker Guide for Rust Backend

## 🐳 Build Docker Image

```bash
cd /mnt/d/iSU/thakur_2.0/ssi-ui/ssi-ui/rust-backend
docker build -t ssi-ui-backend:latest .
```

## 🚀 Run Docker Container

### Option 1: Basic Run
```bash
docker run -p 8080:8080 ssi-ui-backend:latest
```

### Option 2: With Environment Variables
```bash
docker run -p 8080:8080 \
  -e RUST_LOG=debug \
  -e SERVER__HOST=0.0.0.0 \
  -e SERVER__PORT=8080 \
  -e OLLAMA__API_URL=http://172.18.0.111:11434 \
  -e OLLAMA__MODEL=deepseek-r1:8b \
  ssi-ui-backend:latest
```

### Option 3: With Custom Config File
```bash
docker run -p 8080:8080 \
  -v $(pwd)/config.toml:/app/config.toml \
  ssi-ui-backend:latest
```

### Option 4: Detached Mode
```bash
docker run -d -p 8080:8080 --name chatbot-backend ssi-ui-backend:latest
```

## 🔍 Check Container Status

```bash
# View logs
docker logs ssi-ui-backend

# Follow logs
docker logs -f ssi-ui-backend

# Check if running
docker ps

# Health check
curl http://localhost:8080/health
```

## 🛑 Stop Container

```bash
docker stop chatbot-backend
docker rm chatbot-backend
```

## 🔧 What Was Fixed

### Issue: "Error: missing field `server`"
**Cause:** Config file was not found in the expected location

**Solutions Applied:**
1. ✅ Updated Dockerfile to copy `config.toml` to `/app/config.toml`
2. ✅ Modified config loader to check multiple locations
3. ✅ Added support for `CONFIG_PATH` environment variable
4. ✅ Config can now be loaded from:
   - Environment variable `CONFIG_PATH`
   - `/app/config.toml` (default in Docker)
   - `./config.toml` (current directory)
   - Environment variables (e.g., `SERVER__HOST=0.0.0.0`)

## 📝 Environment Variable Format

All config values can be set via environment variables using double underscore `__` as separator:

```bash
# Server config
SERVER__HOST=0.0.0.0
SERVER__PORT=8080
SERVER__WORKERS=4

# Ollama config
OLLAMA__API_URL=http://172.18.0.111:11434
OLLAMA__MODEL=deepseek-r1:8b
OLLAMA__SYSTEM_PROMPT="Format all responses in markdown."
OLLAMA__KEEP_ALIVE=15m
OLLAMA__TIMEOUT_SECONDS=300

# Cache config
CACHE__MAX_SIZE_MB=256
CACHE__TTL_SECONDS=3600
CACHE__ENABLED=true

# Queue config
QUEUE__MAX_CONCURRENT=1
QUEUE__ESTIMATED_TIME_PER_REQUEST_MS=30000

# Batch config
BATCH__MAX_BATCH_SIZE=3
BATCH__BATCH_TIMEOUT_MS=2000
BATCH__ENABLE_DEDUPLICATION=true
```

## 🐳 Docker Compose (Optional)

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  backend:
    image: ssi-ui-backend:latest
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - SERVER__HOST=0.0.0.0
      - SERVER__PORT=8080
    volumes:
      - ./config.toml:/app/config.toml
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--quiet", "--tries=1", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 5s
```

Run with:
```bash
docker-compose up -d
```

## ✅ Verification

After starting the container:

```bash
# 1. Check health
curl http://localhost:8080/health

# 2. Test chat endpoint
curl -X POST http://localhost:8080/api/chat-optimized \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Hello"}],
    "model": "deepseek-r1:8b",
    "systemPrompt": "You are a helpful assistant"
  }'

# 3. Check stats
curl http://localhost:8080/api/cache-stats
```

## 🎯 Summary

- ✅ Dockerfile fixed and optimized
- ✅ Config loading issue resolved
- ✅ Multiple config sources supported
- ✅ Environment variables work
- ✅ Health checks configured
- ✅ Non-root user for security
- ✅ Ready for production deployment
