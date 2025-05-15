FROM lukemathwalker/cargo-chef:0.1.71-rust-bookworm AS chef
WORKDIR /app
# Change mirror to KKU to speed up resolving
RUN sed -i 's/deb\.debian\.org/mirror.kku.ac.th/g' /etc/apt/sources.list.d/debian.sources
# KKU don't have debian-security
RUN sed -i 's/mirror\.kku\.ac\.th\/debian-security/mirror.sg.gs\/debian-security/g' /etc/apt/sources.list.d/debian.sources
# RUN cat /etc/apt/sources.list.d/debian.sources
RUN apt-get update && apt-get install lld clang -y


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
# Build project
RUN cargo build --release --bin telegram-bot

FROM debian:stable-slim AS runtime

# Set the working directory
WORKDIR /app

# Change mirror to KKU to speed up resolving
RUN sed -i 's/deb\.debian\.org/mirror.kku.ac.th/g' /etc/apt/sources.list.d/debian.sources
# KKU don't have debian-security
RUN sed -i 's/mirror\.kku\.ac\.th\/debian-security/mirror.sg.gs\/debian-security/g' /etc/apt/sources.list.d/debian.sources
# Install runtime dependencies
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates \
    # Clean up to keep the image size small
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/telegram-bot telegram-bot

# Set environment variables
ENV APP_ENVIRONMENT=production
ENV RUST_LOG=telegram-bot=info,tower_http=info,sqlx=info

# Run the binary
ENTRYPOINT ["./telegram-bot"]
