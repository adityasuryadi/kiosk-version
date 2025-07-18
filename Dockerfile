FROM rust:1.77 as builder

# Install required dependencies for linking to x86_64 Linux
RUN apt update -y && apt install -y build-essential cmake libclang-dev


WORKDIR /usr/src/app
COPY . .

# Install the x86_64 target
RUN rustup target add x86_64-unknown-linux-gnu

# Build for x86_64
RUN cargo build --release --target=x86_64-unknown-linux-gnu

# Strip binary to reduce size (optional)
RUN strip target/x86_64-unknown-linux-gnu/release/my_rust_app || true

FROM debian:buster-slim

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-gnu/release/my_rust_app /usr/local/bin/my_rust_app

EXPOSE 8080
CMD ["my_rust_app"]
