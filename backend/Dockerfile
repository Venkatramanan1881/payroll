# API image for Cloud Run (TSD §12). Build from the repository root:
#   docker build -f backend/Dockerfile -t payroll-api .
# Migrations are embedded at compile time (sqlx::migrate!); no database is needed to build.

# The toolchain the test suite runs on; bump together.
ARG RUST_VERSION=1.100
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
RUN cargo build --release --locked -p api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 app
COPY --from=build /src/target/release/api /usr/local/bin/api
USER app
ENV PORT=8080
EXPOSE 8080
# `api` serves; `api migrate` applies migrations and exits (run as the schema owner, never by the serving instance).
ENTRYPOINT ["/usr/local/bin/api"]
