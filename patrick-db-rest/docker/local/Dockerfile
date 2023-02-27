FROM rust:1.66
WORKDIR /app
RUN cargo install cargo-watch
COPY . .
RUN cargo build

# cmd cargo watch -x run
CMD ["cargo", "watch", "-x", "run"]