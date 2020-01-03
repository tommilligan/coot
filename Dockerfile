FROM messense/rust-musl-cross:arm-musleabihf

ENV PKG_CONFIG_ALLOW_CROSS=1

RUN apt-get update && apt-get install -qy                                      \
  pkg-config                                                                   \
  libusb-dev                                                                   \
  libusb-1.0-0-dev

ENV CARGO_FEATURE_LINUX_SHARED_LIBUSB=1

CMD cargo build --release --target arm-unknown-linux-musleabihf
