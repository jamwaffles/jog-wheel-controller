# LinuxCNC HAL Rust bindings

Provides **non-realtime** Rust bindings for the LinuxCNC `hal` module. Useful for writing drivers for external hardware.

## Setup

[`bindgen`](https://github.com/rust-lang/rust-bindgen) must be set up correctly. Follow the [requirements section of its docs](https://rust-lang.github.io/rust-bindgen/requirements.html).

LinuxCNC is included as a submodule under `./linuxcnc-hal-sys`. It must be compiled for files to be in the right places.

At minimum (on Linux Mint 19.3):

```bash
apt install \
    libmodbus-dev \
    libgtk2.0-dev \
    yapps2 \
    intltool \
    tk-dev \
    bwidget \
    libtk-img \
    tclx \
    python-tk \
    libboost-python-dev \
    libxmu-dev

cd linuxcnc/src

./autogen.sh

./configure --with-realtime=uspace --enable-non-distributable=yes

make -j $(nproc)
```
