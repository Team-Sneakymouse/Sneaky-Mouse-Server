FROM rust:1.56

WORKDIR /sneaky-mouse-server

COPY ./Cargo* ./
# Copy and compile dummy first to download and cache the dependencies
COPY ./src/dummy.rs ./src/dummy.rs
RUN cargo build --bin dummy --release

COPY ./src/ ./src/
RUN cargo build --bin sm-server --release

CMD ["./target/release/sm-server"]
