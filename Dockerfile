FROM messense/rust-musl-cross:x86_64-musl AS chef
ENV SQLX_OFFLINE=true
RUN cargo install cargo-chef
# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /dockerlord

FROM chef AS planner
# Copy source code from previous stage
COPY . .
# Generate info for caching dependencies
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /dockerlord/recipe.json recipe.json
# Build & cache dependencies
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Copy source code from previous stage
COPY . .
# Build application
RUN cargo build --release --target x86_64-unknown-linux-musl

# Create a new stage with a minimal image
FROM scratch
COPY --from=builder /dockerlord/target/x86_64-unknown-linux-musl/release/docklord-runner /dockerlord
ENTRYPOINT ["/dockerlord"]

# Expose default ports (can be overridden with -p flag)
EXPOSE 3000 50051