FROM rust:1.95-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p smorg

FROM python:3.12-slim-bookworm AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends git ca-certificates libssl3 \
  && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/smorg-rs /usr/local/bin/smorg-rs
COPY bench/file-corpus/run_cases.py /opt/smorg-bench/run_cases.py
ENTRYPOINT ["python", "/opt/smorg-bench/run_cases.py"]
