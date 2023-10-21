FROM rust:latest as build

WORKDIR /proj
COPY . /proj

RUN curl -sSfL https://just.systems/install.sh | bash -s -- --to /usr/local/bin
RUN just build --release

FROM ubuntu:latest

RUN apt update && \
    apt install -y gh git curl ca-certificates && \
    curl -sSfL -o get_helm.sh https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 && chmod +x get_helm.sh && ./get_helm.sh && \
    apt clean

COPY --from=build /proj/target/release/renote /bin/renote
ENTRYPOINT ["/bin/renote"]
