#!/bin/sh

if [ ! -e ./target/storage.img ]; then
	qemu-img create -f raw ./target/storage.img 128M
fi

qemu-system-riscv64 \
	-machine virt \
	-bios target/bin/bmw \
	-drive file=./target/storage.img,format=raw,if=none,id=storage0 \
	-device virtio-blk-device,drive=storage0 \
	-nographic \
	-serial mon:stdio
