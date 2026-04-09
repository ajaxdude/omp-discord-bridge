# Stage 1: Build the Rust binary
FROM rust:1.80-slim-bookworm as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: Setup the runtime environment
FROM debian:bookworm-slim

# Install dependencies for Bun, curl, unzip
RUN apt-get update && apt-get install -y \
    curl \
    unzip \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Bun
ENV BUN_INSTALL=/usr/local
RUN curl -fsSL https://bun.sh/install | bash

# Install OMP globally using Bun
RUN bun install -g oh-my-pi

# Copy the Rust binary from the builder stage
COPY --from=builder /app/target/release/omp_discord_bridge /usr/local/bin/omp_discord_bridge

# Set environment variables
# OMP_PATH will be the global bun installation of omp
ENV OMP_PATH="omp"
ENV RUST_LOG="info"

# Run the discord bridge
CMD ["omp_discord_bridge"]