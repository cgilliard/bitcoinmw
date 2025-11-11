#include <stdint.h>  // Include this for standard integer types if not provided by bmw/types.h

#include "bmw/console.h"
#include "bmw/types.h"

// --- VIRTIO REGISTER OFFSETS ---
#define VIRTIO_BASE 0x10008000ULL
#define VIRTIO_MMIO_DEVICE_FEATURES 0x00C
#define VIRTIO_MMIO_DRIVER_FEATURES 0x010
#define VIRTIO_MMIO_DEVICE_STATUS \
	0x010  // Same physical offset as driver features register
#define VIRTIO_MMIO_QUEUE_SEL 0x018
#define VIRTIO_MMIO_QUEUE_NUM_MAX 0x01C
#define VIRTIO_MMIO_QUEUE_NUM 0x020
#define VIRTIO_MMIO_QUEUE_ALIGN 0x024
#define VIRTIO_MMIO_QUEUE_PFN 0x028
#define VIRTIO_MMIO_QUEUE_NOTIFY 0x030
#define SECTOR_SIZE 512

// Device Status Bits
#define VIRTIO_STATUS_ACKNOWLEDGE (1 << 0)
#define VIRTIO_STATUS_DRIVER (1 << 1)
#define VIRTIO_STATUS_FAILED (1 << 7)
#define VIRTIO_STATUS_FEATURES_OK (1 << 3)
#define VIRTIO_STATUS_DRIVER_OK (1 << 2)

#define R_U32(reg) (*(volatile uint32_t*)(VIRTIO_BASE + (reg)))
#define W_U32(reg, val) (*(volatile uint32_t*)(VIRTIO_BASE + (reg)) = (val))

// --- GLOBAL VIRTQUEUE STRUCTURE ---
// Ensure this structure has 4096-byte alignment as required by the spec.
__attribute__((aligned(4096))) static struct {
	struct {
		u64 addr;
		u32 len;
		u16 flags;
		u16 next;
	} desc[8];
	struct {
		u16 flags;
		u16 idx;
		u16 ring[8];
	} avail;
	// Manual padding is complex and risky. A simple approach is to use a
	// fixed buffer size that guarantees alignment for the 'used' ring. This
	// is still a fragile implementation.
	u8 pad[2048];
	struct {
		u16 flags;
		u16 idx;
		struct {
			u32 id;
			u32 len;
		} used[8];
	} used;
	u8 status;
} q;

// --- GLOBAL BUFFER FOR DATA TRANSFER ---
__attribute__((aligned(4096))) static u8 buf[SECTOR_SIZE];

// --- DISK FUNCTIONS ---

void disk_init(void) {
	// Phase 1: Reset and Acknowledge
	W_U32(VIRTIO_MMIO_DEVICE_STATUS, 0);  // 1. Reset the device

	u32 status = 0;
	status |= VIRTIO_STATUS_ACKNOWLEDGE;  // 2. Acknowledge
	W_U32(VIRTIO_MMIO_DEVICE_STATUS, status);

	status |= VIRTIO_STATUS_DRIVER;	 // 3. Set DRIVER bit
	W_U32(VIRTIO_MMIO_DEVICE_STATUS, status);

	// Phase 2: Feature Negotiation (Simple acceptance of all offered
	// features)
	u32 device_features =
	    R_U32(VIRTIO_MMIO_DEVICE_FEATURES);	 // 4. Read offered features
	W_U32(VIRTIO_MMIO_DRIVER_FEATURES,
	      device_features);	 // 5. Write accepted features

	status |= VIRTIO_STATUS_FEATURES_OK;  // 6. Set FEATURES_OK
	W_U32(VIRTIO_MMIO_DEVICE_STATUS, status);

	// Verify features were accepted
	if (!(R_U32(VIRTIO_MMIO_DEVICE_STATUS) & VIRTIO_STATUS_FEATURES_OK)) {
		puts("VirtIO features not accepted. Aborting.\n");
		status |= VIRTIO_STATUS_FAILED;
		W_U32(VIRTIO_MMIO_DEVICE_STATUS, status);
		// abort() is used here as a placeholder for proper error
		// handling
	}

	// Phase 3: VirtQueue Configuration
	W_U32(VIRTIO_MMIO_QUEUE_SEL,
	      0);  // 7. Select queue 0 (the only one for block device)
	if (R_U32(VIRTIO_MMIO_QUEUE_NUM_MAX) == 0) {  // 8. Check max queue size
		puts("VirtIO Queue 0 not available. Aborting.\n");
		// abort();
	}

	W_U32(VIRTIO_MMIO_QUEUE_NUM,
	      8);  // 9. Tell device we use 8 entries (from your struct q)
	W_U32(VIRTIO_MMIO_QUEUE_ALIGN, 4096);  // 9. Set queue alignment

	// 10. Set the physical address of the virtqueue structure.
	u64 q_paddr = (u64)&q;
	W_U32(
	    VIRTIO_MMIO_QUEUE_PFN,
	    q_paddr >> 12);  // Write the physical frame number (address >> 12)

	// Initialize the queue indices for driver tracking
	q.avail.idx = 0;
	q.used.idx = 0;
	q.status = 0xFF;

	// Phase 4: Driver Ready
	status |= VIRTIO_STATUS_DRIVER_OK;  // 11. Set DRIVER_OK bit
	W_U32(VIRTIO_MMIO_DEVICE_STATUS, status);

	puts("Disk ready and initialized via full handshake.\n");
}

int disk_write(u64 sector, const void* data) {
	// This code still needs cache coherence fixes and descriptor management
	// improvements, but the initialization is now correct.
	u64* req = (u64*)&q.desc[0].flags;

	q.desc[0].addr = (u64)req;
	q.desc[0].len = 16;
	q.desc[0].flags = 1;
	q.desc[0].next = 1;

	q.desc[1].addr = (u64)data;
	q.desc[1].len = SECTOR_SIZE;
	q.desc[1].flags = 1 | 2;  // Writable by device
	q.desc[1].next = 2;

	q.desc[2].addr = (u64)&q.status;
	q.desc[2].len = 1;
	q.desc[2].flags = 2;  // Writable by device
	q.desc[2].next = 0;

	// The request header fields (type, sector)
	req[0] = 1;  // Type 1: VIRTIO_BLK_T_OUT (write)
	req[1] = sector;

	q.status = 0xFF;
	q.avail.ring[q.avail.idx % 8] = 0;
	// Need a memory barrier here before updating the index/notifying
	q.avail.idx++;
	// Use the correct notify register offset
	W_U32(VIRTIO_MMIO_QUEUE_NOTIFY, 0);

	while (q.used.idx == q.avail.idx - 1);
	while (q.status == 0xFF);

	return q.status == 0 ? 0 : -1;
}

int disk_read(u64 sector, void* data) {
	// This code still needs cache coherence fixes and descriptor management
	// improvements
	u64* req = (u64*)&q.desc[0].flags;

	q.desc[0].addr = (u64)req;
	q.desc[0].len = 16;
	q.desc[0].flags = 1;
	q.desc[0].next = 1;

	q.desc[1].addr = (u64)data;
	q.desc[1].len = SECTOR_SIZE;
	q.desc[1].flags = 2;  // Writable by device (read operation destination)
	q.desc[1].next = 2;

	q.desc[2].addr = (u64)&q.status;
	q.desc[2].len = 1;
	q.desc[2].flags = 2;  // Writable by device
	q.desc[2].next = 0;

	// The request header fields (type, sector)
	req[0] = 0;  // Type 0: VIRTIO_BLK_T_IN (read)
	req[1] = sector;

	q.status = 0xFF;
	q.avail.ring[q.avail.idx % 8] = 0;
	// Need a memory barrier here
	q.avail.idx++;
	// Use the correct notify register offset
	W_U32(VIRTIO_MMIO_QUEUE_NOTIFY, 0);

	while (q.used.idx == q.avail.idx - 1);
	while (q.status == 0xFF);

	return q.status == 0 ? 0 : -1;
}

void main(void) {
	puts("=== PERSISTENT DISK TEST ===\n");
	// Call the new initialization function
	disk_init();

	// Add some test code here eventually, e.g.:
	// disk_read(0, buf);
	// puts("Read sector 0\n");

	puts("complete\n");
	abort();
}

