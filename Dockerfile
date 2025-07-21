FROM rust:1.88-slim-trixie AS builder

WORKDIR /noters
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() { println!(\"cache :)\"); }" > src/main.rs
RUN cargo fetch

COPY src ./src
RUN cargo build --release --bin noters


FROM debian:trixie-slim
COPY --from=builder /noters/target/release/noters /usr/local/bin/noters

ENTRYPOINT [ \
    "/usr/local/bin/noters", \
    "--user", "ctf", \
    "--max-note-count", "32", \
    "--max-name-size", "32", \
    "--max-content-size", "256", \
    "sqlite", \
    "--path", "notes.db" \
]
