#!/bin/bash

set -xe

LINUXCNC_DEPS=

# linuxcnc-hal-sys setup
# LinuxCNC dependencies
sudo apt install -qq \
    bwidget \
    intltool \
    kmod \
    libboost-python-dev \
    libglu-dev \
    libgtk2.0-dev \
    libmodbus-dev \
    libtk-img \
    libudev-dev \
    libusb-1.0-0-dev \
    libx11-dev \
    libxinerama-dev \
    libxmu-dev \
    mesa-common-dev \
    python \
    python-tk \
    tclx \
    tk-dev \
    yapps2

# Bindgen deps
sudo apt install -qq libclang-dev llvm-dev libclang-dev clang

pushd ./linuxcnc-hal-sys/linuxcnc-src/src
./autogen.sh
./configure \
  --with-realtime=uspace \
  --enable-non-distributable=yes \
  --disable-userspace-pci \
  --disable-check-runtime-deps
make -j $(nproc)
popd
