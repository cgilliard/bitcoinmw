#include "sha3.h"
#include "types.h"
#include "util.h"

// sha3_256 wrapper function
void sha3_256(void* out, const void* in, size_t len) {
	Sha3Context ctx;
	sha3_init256(&ctx);
	// IMPORTANT: We use SHA3_FLAGS_KECCAK flag instead of
	// SHA3_FLAGS_NONE (NIST). This is to differentiate from existing
	// implementations.
	sha3_setflags(&ctx, SHA3_FLAGS_KECCAK);
	sha3_update(&ctx, in, len);
	const void* tmp_out = sha3_finalize(&ctx);
	copy_bytes(out, tmp_out, 32);
}

void heavyhash(const uint16 matrix[64][64], void* pdata, size_t pdata_len,
	       void* output) {
	byte hash_first[32] __attribute__((aligned(32)));
	byte hash_second[32] __attribute__((aligned(32)));
	byte hash_xored[32] __attribute__((aligned(32)));

	uint16 vector[64] __attribute__((aligned(64)));
	uint16 product[64] __attribute__((aligned(64)));

	// note: slight change from original (remove first length param). For
	// compatibility with sha3 interface.
	sha3_256((byte*)hash_first, (const byte*)pdata, pdata_len);

	for (int i = 0; i < 32; ++i) {
		vector[2 * i] = (hash_first[i] >> 4);
		vector[2 * i + 1] = hash_first[i] & 0xF;
	}

	for (int i = 0; i < 64; ++i) {
		uint16 sum = 0;
		for (int j = 0; j < 64; ++j) {
			sum += matrix[i][j] * vector[j];
		}
		product[i] = (sum >> 10);
	}

	for (int i = 0; i < 32; ++i) {
		hash_second[i] = (product[2 * i] << 4) | (product[2 * i + 1]);
	}

	for (int i = 0; i < 32; ++i) {
		hash_xored[i] = hash_first[i] ^ hash_second[i];
	}

	// note: slight change from original (remove first length param). For
	// compatibility with sha3 interface.
	sha3_256((byte*)output, (const byte*)hash_xored, 32);
}
