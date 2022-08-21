# STAGE 1
FROM rust:1.62 AS builder
# Install dependencies
RUN apt update
# Copy source code.
COPY src /src
COPY Cargo.toml Cargo.lock /
# Build rust binaries.
RUN cargo build --release --manifest-path /Cargo.toml

# STAGE 2
FROM ubuntu:20.04 as base
ARG DEBIAN_FRONTEND=noninteractive
ENV TZ=America/New_York
# Install dependencies
RUN apt update
# Copy only the worker binary and execute.
COPY --from=builder /target/release/algo3_backend /algo3_backend
# Expose port.
EXPOSE 80
# Run binaries
CMD ["/algo3_backend"]
