# Envoy External Processor in Rust

## Disclaimer
This is almost entirely AI-generated and was only used for testing. No guarantees whatsoever.

## Purpose

This is a simple Envoy External Processing gRPC server written in Rust. It demonstrates how to interact with Envoy to modify HTTP traffic.

## Build Process

Makefile has a target that fetches necessary Protocol Buffer (proto) files for Envoy's API (via Bazel). Then, during the build process, `build.rs` uses Bazel cache to add these proto files as Rust code.

### Build

#### Prerequisites
* `make`
* For local build: Rust, Cargo and Bazel
* For Docker-based build: Docker


#### Local build
```bash
make build
```
The binary will typically be located at `target/release/my-ext-processor`.

#### Build with Docker
Build an image:
```bash
make docker-build
```

Push the image:
```bash
make docker-push
```

Build and push combined:
```bash
make build-push
```

Customize image name (which is `igormendix/rust-extproc-server` by default):
```bash
IMAGE_NAME=someone/some-name make build-push
```
(or set `IMAGE_NAME` env var).


## Server Functionality

The gRPC server implemented in `src/main.rs` acts as a dummy processor. Its primary function is to:

* Listen for incoming gRPC requests from Envoy on port `50051`
* For each HTTP request phase (request headers, response headers, request trailers, response trailers), it adds a custom HTTP header. For example, it adds `x-processed-by-rust-request: my-ext-processor` to request headers and `x-processed-by-rust-response: my-ext-processor` to response headers
* It allows the request and response bodies to pass through without modification
* Supports gRPC reflection for easier inspection

## Kubernetes

The `k8s.yaml` is an example on how to deploy this server and configure Envoy Gateway to use it via an `EnvoyExtensionPolicy`.
