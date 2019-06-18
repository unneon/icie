FROM ubuntu:16.04

RUN apt-get update
RUN apt-get install -y curl g++ git libdbus-1-dev

RUN curl -sL https://deb.nodesource.com/setup_8.x | bash -
RUN apt-get -y install nodejs
RUN npm install -g vsce

RUN curl https://sh.rustup.rs > /root/rustup.sh
RUN chmod +x /root/rustup.sh
RUN /root/rustup.sh -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /icie
COPY . /icie
RUN cargo build --release
RUN cargo run --release -- --package

EXPOSE 8080
CMD ["python", "-m", "SimpleHTTPServer", "8080"]
