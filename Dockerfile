#-------------------------------------------------------------------
# Stage 1: Builder
#-------------------------------------------------------------------
# Use a specific Rust version for reproducibility
FROM rust AS builder

# --- Install Build Dependencies ---
# Define Bazelisk version as an argument
ARG BAZELISK_VERSION=v1.26.0 # Check for latest stable version if needed

# Added tzdata for potential timezone issues during package installs
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    make \
    protobuf-compiler \
    git \
    tzdata \
 # Clean up apt cache
 && rm -rf /var/lib/apt/lists/* \
 # Download and install Bazelisk
 && curl -Lo /usr/local/bin/bazel https://github.com/bazelbuild/bazelisk/releases/download/${BAZELISK_VERSION}/bazelisk-linux-amd64 \
 && chmod +x /usr/local/bin/bazel

# --- Create non-root user ---
# Create appuser (uid 1001) and appgroup (gid 1001)
RUN groupadd --system --gid 1001 appgroup && \
    useradd --system --uid 1001 --gid appgroup --shell /bin/bash --create-home appuser

# Set working directory
WORKDIR /app
RUN chown -R appuser:appgroup /app

# --- Copy only Makefile first with proper ownership ---
COPY --chown=appuser:appgroup Makefile ./

# --- Switch user for fetch-deps ---
USER appuser

# --- Run fetch-deps in its own layer ---
# This clones Envoy and runs 'bazel fetch'. It creates envoy_repo/ and bazel_output_base.txt
# This layer will be cached if Makefile, .bazelversion, WORKSPACE haven't changed.
RUN make fetch-deps

# --- Copy remaining files with proper ownership ---
COPY --chown=appuser:appgroup Cargo.toml Cargo.lock build.rs ./
COPY --chown=appuser:appgroup src src

# --- Build the Application using Makefile ---
# This now runs as 'appuser' and owns all necessary source/config files.
# It will re-run 'bazel info output_base' and 'protoc' steps from the Makefile,
# but the expensive 'git clone' and 'bazel fetch' should be cached from the layer above.
RUN make release

#-------------------------------------------------------------------
# Stage 2: Runtime
#-------------------------------------------------------------------
# Use a minimal base image
FROM debian:bookworm-slim AS runtime

# Recreate the same non-root user/group as in the builder stage
RUN groupadd --system --gid 1001 appgroup && \
    useradd --system --uid 1001 --gid appgroup appuser

WORKDIR /app

# Copy the compiled binary from the builder stage
# Use --chown to set the correct owner directly during copy
# Adjust 'my_ext_proc_server' if your Rust package/binary name is different
COPY --from=builder --chown=appuser:appgroup /app/target/release/my_ext_proc_server /app/my_ext_proc_server

# Switch to the non-root user for running the application
USER appuser

# Expose the gRPC port the application listens on
EXPOSE 50051

# Define the command to run the application
CMD ["./my_ext_proc_server"]

