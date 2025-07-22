# Stage 1: Build the application
FROM --platform=linux/amd64 ubuntu:20.04 as builder

ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev



# Install Rust
# RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup.sh \
    && chmod +x rustup.sh \
    && ./rustup.sh -y \
    && rm rustup.sh

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /usr/src/app

# Copy only what's needed to build dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p ./src && \
    echo "fn main() {}" > ./src/main.rs && \
    cargo build --release && \
    rm -rf ./src

# Now copy the actual source code
COPY src ./src

# Touch main.rs to force rebuild
RUN touch ./src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-gnu 

# Stage 2: Create the runtime image
FROM ubuntu:20.04

# Install runtime dependencies if needed
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl-dev \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m myuser
USER myuser

RUN  ls

# Copy the built binary from builder
COPY --from=builder  /usr/src/app/target/x86_64-unknown-linux-gnu/release/kiosk_versioning /app/

WORKDIR /app

# Set the entry point
ENTRYPOINT ["./kiosk_versioning"]