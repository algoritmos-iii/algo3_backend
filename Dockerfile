# STAGE 1
FROM rust:alpine3.15 AS builder
# Install dependencies
RUN apk add musl-dev --no-cache
# Copy source code.
COPY src /src
COPY Cargo.toml Cargo.lock /
# Build rust binaries.
RUN cargo build --release --manifest-path /Cargo.toml

# STAGE 2
FROM alpine:3.15 as base
# Copy only the worker binary and execute.
COPY --from=builder /target/release/algo3_backend /algo3_backend
# Expose port.
EXPOSE 80
# Run binaries
CMD ["./algo3_backend"]
