/********************************************************************************
 * MIT License
 *
 * Copyright (c) 2025 Christopher Gilliard
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 *******************************************************************************/

#include <bmw/hw.h>
#include <bmw/io.h>

// VirtIO MMIO base (QEMU virt machine)
#define VIRTIO_BLK_BASE 0x10008000

// MMIO registers
#define VIRTIO_MAGIC (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x000))
#define VIRTIO_VERSION (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x004))
#define VIRTIO_DEVICE_ID (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x008))
#define VIRTIO_VENDOR_ID (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x00C))
#define VIRTIO_DEVICE_FEATURES (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x010))
#define VIRTIO_DRIVER_FEATURES (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x020))
#define VIRTIO_QUEUE_ADDR (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x030))
#define VIRTIO_QUEUE_NUM (*(volatile u16 *)(VIRTIO_BLK_BASE + 0x038))
#define VIRTIO_QUEUE_READY (*(volatile u16 *)(VIRTIO_BLK_BASE + 0x044))
#define VIRTIO_QUEUE_NOTIFY (*(volatile u16 *)(VIRTIO_BLK_BASE + 0x050))
#define VIRTIO_INTERRUPT_STATUS (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x060))
#define VIRTIO_INTERRUPT_ACK (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x064))
#define VIRTIO_STATUS (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x070))
#define VIRTIO_QUEUE_PFN (*(volatile u32 *)(VIRTIO_BLK_BASE + 0x030))

// Queue 0
#define QUEUE_SIZE 8
static u64 desc[QUEUE_SIZE * 16 / 8] __attribute__((aligned(4096)));
static u16 avail[2 + QUEUE_SIZE] __attribute__((aligned(2)));
static u16 used[2 + QUEUE_SIZE * 4] __attribute__((aligned(4)));

static u16 used_idx = 0;

void virtio_blk_init(void) {
	if (VIRTIO_MAGIC != 0x74726976) return;
	if (VIRTIO_VERSION != 1 && VIRTIO_VERSION != 2) return;
	if (VIRTIO_DEVICE_ID != 2) return;
	puts("end4\n");

	VIRTIO_STATUS = 0;
	VIRTIO_STATUS = 4;  // ACKNOWLEDGE
	VIRTIO_DRIVER_FEATURES = 0;
	VIRTIO_STATUS |= 8;  // DRIVER

	for (u32 i = 0; i < QUEUE_SIZE * 16 / 8; i++) desc[i] = 0;
	u64 v = *(u64 *)desc;
	// u64 v = 10;
	puthex(v, 24);
	puts("\n");
	/*VIRTIO_QUEUE_PFN = (u32)((u64)desc >> 12);*/
	VIRTIO_QUEUE_NUM = QUEUE_SIZE;
	VIRTIO_QUEUE_READY = 1;
	VIRTIO_STATUS |= 128;
}

static void wait_for_completion(void) {
	while (used_idx == ((volatile u16 *)used)[0]) {
	}
	used_idx = ((volatile u16 *)used)[0];
}

void virtio_blk_read(u64 sector, void *buf) {
	u32 idx = avail[1] % QUEUE_SIZE;

	desc[idx * 4 + 0] = (u64)&sector;
	desc[idx * 4 + 1] = 8;
	desc[idx * 4 + 2] = 2;
	desc[idx * 4 + 3] = idx * 4 + 1;

	desc[idx * 4 + 4] = (u64)buf;
	desc[idx * 4 + 5] = BLK_SIZE;
	desc[idx * 4 + 6] = 2;	// VRING_DESC_F_WRITE
	desc[idx * 4 + 7] = 0;

	avail[2 + idx] = idx * 4;
	avail[1]++;

	VIRTIO_QUEUE_NOTIFY = 0;
	wait_for_completion();
}

void virtio_blk_write(u64 sector, const void *buf) {
	u32 idx = avail[1] % QUEUE_SIZE;

	desc[idx * 4 + 0] = (u64)&sector;
	desc[idx * 4 + 1] = 8;
	desc[idx * 4 + 2] = 2;
	desc[idx * 4 + 3] = idx * 4 + 1;

	desc[idx * 4 + 4] = (u64)buf;
	desc[idx * 4 + 5] = BLK_SIZE;
	desc[idx * 4 + 6] = 0;
	desc[idx * 4 + 7] = 0;

	avail[2 + idx] = idx * 4;
	avail[1]++;

	VIRTIO_QUEUE_NOTIFY = 0;
	wait_for_completion();
}
