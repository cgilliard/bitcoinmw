#include <bmw/console.h>
#include <bmw/io.h>

#define VIRTIO_BASE 0x10001000

#define VIRTIO_STATUS (*(volatile u32 *)(VIRTIO_BASE + 0x70))
#define VIRTIO_FEATURES (*(volatile u32 *)(VIRTIO_BASE + 0x20))
#define VIRTIO_QUEUE_PFN (*(volatile u32 *)(VIRTIO_BASE + 0x30))
#define VIRTIO_QUEUE_NUM (*(volatile u16 *)(VIRTIO_BASE + 0x38))
#define VIRTIO_QUEUE_READY (*(volatile u16 *)(VIRTIO_BASE + 0x44))
#define VIRTIO_QUEUE_NOTIFY (*(volatile u16 *)(VIRTIO_BASE + 0x50))

#define VRING_DESC_F_NEXT 1
#define VRING_DESC_F_WRITE 2

#define QUEUE_SIZE 8

static u64 desc[QUEUE_SIZE * 16 / 8] __attribute__((aligned(4096)));
static u16 avail[2 + QUEUE_SIZE] __attribute__((aligned(2)));
static u16 used[2 + QUEUE_SIZE * 4] __attribute__((aligned(4)));

static u16 used_idx = 0;

void blk_init(void) {
	VIRTIO_STATUS = 4;
	VIRTIO_FEATURES = 0;
	VIRTIO_STATUS |= 8;

	for (u32 i = 0; i < QUEUE_SIZE * 16 / 8; i++) desc[i] = 0;
	for (u32 i = 0; i < 2 + QUEUE_SIZE; i++) avail[i] = 0;
	for (u32 i = 0; i < 2 + QUEUE_SIZE * 4; i++) used[i] = 0;

	VIRTIO_QUEUE_PFN = (u32)((u64)desc >> 12);
	VIRTIO_QUEUE_NUM = QUEUE_SIZE;
	VIRTIO_QUEUE_READY = 1;
	VIRTIO_STATUS |= 128;
}

static void wait_for_completion(void) {
	u32 timeout = 1000000;
	while (used_idx == ((volatile u16 *)used)[0] && timeout--) {
	}
	if (timeout == 0) return;
	used_idx = ((volatile u16 *)used)[0];
}

void blk_write(u64 sector, const void *buf) {
	u32 idx = 0;

	desc[0] = (u64)&sector;
	desc[1] = 8;
	desc[2] = VRING_DESC_F_NEXT;
	desc[3] = 4;

	desc[4] = (u64)buf;
	desc[5] = BLK_SIZE;
	desc[6] = 0;
	desc[7] = 0;

	avail[2] = 0;
	avail[1] = 1;

	VIRTIO_QUEUE_NOTIFY = 0;
	wait_for_completion();
}

void blk_read(u64 sector, void *buf) {
	u32 idx = 0;

	desc[0] = (u64)&sector;
	desc[1] = 8;
	desc[2] = VRING_DESC_F_NEXT;
	desc[3] = 4;

	desc[4] = (u64)buf;
	desc[5] = BLK_SIZE;
	desc[6] = VRING_DESC_F_WRITE;
	desc[7] = 0;

	avail[2] = 0;
	avail[1] = 1;

	VIRTIO_QUEUE_NOTIFY = 0;
	wait_for_completion();
}
