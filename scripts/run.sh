#!/bin/sh

qemu-system-riscv64 \
	-machine virt \
	-bios target/bin/bmw \
	-nographic \
	-serial mon:stdio
