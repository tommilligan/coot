# coot

CO2 IoT monitor.

## Cross-compiling with `raspi-toolchain`

Setup `cargo` to use an external linker:

```toml
# ~/.cargo/config.toml

[target.arm-unknown-linux-gnueabihf]
linker = "/opt/cross-pi-gcc/bin/arm-linux-gnueabihf-gcc"
```

Install the `raspi-toolchain` compiler.

```bash
cd /tmp
wget https://github.com/Pro/raspi-toolchain/releases/latest/download/raspi-toolchain.tar.gz
sudo tar xfz raspi-toolchain.tar.gz --strip-components=1 -C /opt
rm /tmp/raspi-toolchain.tar.gz
```

Pull system libs from the target machine for linking:

```bash
PI_HOSTNAME=dewberry
export ROOTFS=/home/tom/$PI_HOSTNAME/rootfs

mkdir -p $ROOTFS
rsync -vR --progress -rl --delete-after --safe-links pi@$PI_HOSTNAME:/{lib,usr,opt/vc/lib} $ROOTFS
# Add a symlink here to match an expected filepath from libhidapi
(cd /home/tom/dewberry/rootfs/usr/lib/arm-linux-gnueabihf; ln -s libhidapi-libusb.so.0 libhidapi-libusb.so)
```

Compile the crate, pointing the linker to the correct locations for system libs.

```bash
export TOOLBIN=/opt/cross-pi-gcc/bin
export STAGING=/opt/cross-pi-gcc/arm-linux-gnueabihf
export HOST=arm-linux-gnueabihf
export CC=$TOOLBIN/arm-linux-gnueabihf-gcc
export CXX=$TOOLBIN/arm-linux-gnueabihf-g++

PKG_CONFIG_ALLOW_CROSS=1 cargo rustc -vv --target arm-unknown-linux-gnueabihf --  \
  -C linker=$CC/arm-linux-gnueabihf-gcc -L $STAGING/lib   \
  -C link-arg=-Wl,-rpath-link,$STAGING/lib  -L $ROOTFS/lib \
  -C link-arg=-Wl,-rpath-link,$ROOTFS/lib  -L $ROOTFS/usr/lib/$HOST \
  -C link-arg=-Wl,-rpath-link,$ROOTFS/usr/lib/$HOST -L $ROOTFS/lib/$HOST   \
  -C link-arg=-Wl,-rpath-link,$ROOTFS/lib/$HOST
```

Copy the output to the target machine.

```bash
scp target/arm-unknown-linux-gnueabihf/debug/coot pi@dewberry:~
```

Notes:

- See example cmake file for complete list of linking locations: https://github.com/Pro/raspi-toolchain/blob/master/Toolchain-rpi.cmake
