#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sha3.h"

// sha3_384 wrapper function
void sha3_384(void* out, const void* in, size_t len) {
	Sha3Context ctx;
	sha3_init384(&ctx);
	sha3_setflags(&ctx, SHA3_FLAGS_NONE);
	sha3_update(&ctx, in, len);
	const void* tmp_out = sha3_finalize(&ctx);
	memcpy(out, tmp_out, 48);
}

void heavyhash(const uint16_t matrix[64][64], void* pdata, size_t pdata_len,
	       void* output) {
	uint8_t hash_first[48] __attribute__((aligned(32)));
	uint8_t hash_second[32] __attribute__((aligned(32)));
	uint8_t hash_xored[32] __attribute__((aligned(32)));

	uint16_t vector[64] __attribute__((aligned(64)));
	uint16_t product[64] __attribute__((aligned(64)));

	sha3_384(hash_first, pdata, pdata_len);

	// Use first 32 bytes of hash_first for 64 nibbles
	// note: truncate the sha384 output from 48 bytes -> 32 bytes
	// so we maintain the heavyhash matrix algorithm (designed for sha3_256)
	for (int i = 0; i < 32; ++i) {
		vector[2 * i] = (hash_first[i] >> 4);
		vector[2 * i + 1] = hash_first[i] & 0xF;
	}

	for (int i = 0; i < 64; ++i) {
		uint16_t sum = 0;
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

	uint8_t temp_output[48] __attribute__((aligned(32)));
	sha3_384(temp_output, hash_xored, 32);
	memcpy(output, temp_output, 32);  // Truncate final output to 32 bytes
}

