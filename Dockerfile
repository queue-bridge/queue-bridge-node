FROM rust:1.88 AS builder

WORKDIR /usr/src/app

# 安装 protoc
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

# 复制实际源码并构建发布版本
COPY . .
RUN cargo build --release

FROM ubuntu:questing

RUN apt-get update \
  && apt-get install -y ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /root/
COPY --from=builder /usr/src/app/target/release/queue-bridge-node .
ENTRYPOINT ["./queue-bridge-node"]
