# Multi-stage build for minimal final image

# Stage 1: Build
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests and source
COPY Cargo.toml ./
COPY src ./src

# Build application
RUN cargo build --release && \
    strip target/release/chatbot-backend

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 chatbot && \
    mkdir -p /etc/chatbot && \
    chown -R chatbot:chatbot /etc/chatbot

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/chatbot-backend /usr/local/bin/chatbot-backend

# Copy configuration to working directory
COPY config.toml /app/config.toml

# Change ownership
RUN chown chatbot:chatbot /app/config.toml

# Switch to non-root user
USER chatbot

# Set environment
ENV RUST_LOG=info

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:8080/health || exit 1

# Run
CMD ["chatbot-backend"]
