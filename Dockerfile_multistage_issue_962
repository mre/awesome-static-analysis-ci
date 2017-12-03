FROM rust as builder
COPY . /usr/src/app 
WORKDIR /usr/src/app 
RUN cargo build --release

FROM debian:stretch
COPY --from=builder target/release/check .
ENTRYPOINT ["./check"]
CMD ["--help"]
