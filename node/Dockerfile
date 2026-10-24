FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p sunmint-verifier -p sunmint-simulator

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/sunmint-verifier /src/target/release/sunmint-simulator /usr/local/bin/
COPY start.sh /usr/local/bin/start.sh
ENV PORT=8080 DB_PATH=/data/sunmint.db VERIFIER_URL=http://127.0.0.1:8080
EXPOSE 8080
CMD ["/usr/local/bin/start.sh"]
