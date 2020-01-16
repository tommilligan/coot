# coot

CO2 IoT monitor.

## Cross-compiling with `raspi-toolchain`

### On the target device (Raspberry Pi 1, arm6)

Install required system libs:

```bash
sudo apt-get install libhidapi-libusb0 libhidapi-dev libc
```

### On the compiling machine

#### Sync target libs

Pull system libs from the target machine for linking:

```bash
export PI_HOSTNAME=dewberry
export ROOTFS=/home/tom/$PI_HOSTNAME/rootfs

mkdir -p $ROOTFS
rsync -vR --progress -rl --delete-after --safe-links pi@$PI_HOSTNAME:/{lib,usr,opt/vc/lib} $ROOTFS
# Add a symlink here to match an expected filepath from libhidapi
(cd /home/tom/dewberry/rootfs/usr/lib/arm-linux-gnueabihf; ln -s libhidapi-libusb.so.0 libhidapi-libusb.so)
```

#### Configure cross-compile toolchain

Install the `raspi-toolchain` compiler.

```bash
cd /tmp
wget https://github.com/Pro/raspi-toolchain/releases/latest/download/raspi-toolchain.tar.gz
sudo tar xfz raspi-toolchain.tar.gz --strip-components=1 -C /opt
rm /tmp/raspi-toolchain.tar.gz
```

Setup `cargo` to use an external linker:

```toml
# ~/.cargo/config.toml

[target.arm-unknown-linux-gnueabihf]
linker = "/opt/cross-pi-gcc/bin/arm-linux-gnueabihf-gcc"
```

#### Compile!

Compile the crate, pointing the linker to the correct locations for system libs.

```bash
export TOOLBIN=/opt/cross-pi-gcc/bin
export STAGING=/opt/cross-pi-gcc/arm-linux-gnueabihf
export HOST=arm-linux-gnueabihf
export CC=$TOOLBIN/arm-linux-gnueabihf-gcc
export CXX=$TOOLBIN/arm-linux-gnueabihf-g++

PKG_CONFIG_ALLOW_CROSS=1 cargo rustc --release --target arm-unknown-linux-gnueabihf --  \
  -C linker=$CC/arm-linux-gnueabihf-gcc -L $STAGING/lib   \
  -C link-arg=-Wl,-rpath-link,$STAGING/lib  -L $ROOTFS/lib \
  -C link-arg=-Wl,-rpath-link,$ROOTFS/lib  -L $ROOTFS/usr/lib/$HOST \
  -C link-arg=-Wl,-rpath-link,$ROOTFS/usr/lib/$HOST -L $ROOTFS/lib/$HOST   \
  -C link-arg=-Wl,-rpath-link,$ROOTFS/lib/$HOST
```

> See example cmake file for complete list of linking locations: https://github.com/Pro/raspi-toolchain/blob/master/Toolchain-rpi.cmake

#### Deploy

Copy the output to the target machine.

```bash
scp target/arm-unknown-linux-gnueabihf/debug/coot pi@dewberry:~
```

## Installation

### udev Rules

See guide on how to setup udev rules [here](https://github.com/lnicola/co2mon). Essentially:

```
sudo echo 'ACTION=="add|change", SUBSYSTEMS=="usb", ATTRS{idVendor}=="04d9", ATTRS{idProduct}=="a052", MODE:="0666"' > /etc/udev/rules.d/60-co2mon.rules
sudo udevadm control --reload
sudo udevadm trigger
```

### Run

Ensure you have a configuration file `coot.yml` in the working directory. See [example](./example) for a sample.

Run like `./coot >> coot.jsonl 2>> coot.log`. Data will be output to `coot.jsonl` as well as uploaded to InfluxDB.

### Log rotation

You should set up log rotation like:

```conf
# /etc/logrotate.d/coot

/home/pi/coot.jsonl {
  daily
  missingok
  rotate 3650
  compress
  copytruncate
}

/home/pi/coot.log {
  weekly
  missingok
  rotate 4
  compress
  copytruncate
}
```

## Other resources

> TODO: pull these together to be more than just a tabdump

- [Cross compile hidapi](https://www.raspberrypi.org/forums/viewtopic.php?t=143377)
- [Run cross-compiled code on RPi Zero](https://www.reddit.com/r/rust/comments/9io0z8/run_crosscompiled_code_on_rpi_0/)
- [Installing RPi cross-compiler toolchain on Linux x86_64](https://stackoverflow.com/questions/19162072/how-to-install-the-raspberry-pi-cross-compiler-on-my-linux-host-machine/58559140#58559140)
- [RPi toolchain cmake](https://github.com/Pro/raspi-toolchain/blob/master/Toolchain-rpi.cmake)
- [RPi toolchain gcc version mismatch](https://github.com/raspberrypi/tools/issues/102)
- [GCC version not upgraded](https://github.com/raspberrypi/tools/issues/81)
- [New/Recent RPi toolchain](https://github.com/Pro/raspi-toolchain)
- [Cross compiling a toolchain from scratch](https://www.raspberrypi.org/forums/viewtopic.php?t=7493)
