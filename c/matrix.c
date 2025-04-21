#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "aes.h"

// Assume getentropy is available (e.g., via unistd.h or custom syscall)
int getentropy(void* buffer, size_t length);

// GF(2^4) multiplication table for polynomial x^4 + x + 1
// Generated for elements 0–15, where 0x1 represents 1, 0x2 represents x, etc.
static const uint8_t gf16_mult[16][16] = {
    {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0},
    {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
    {0, 2, 4, 6, 8, 10, 12, 14, 3, 1, 7, 5, 11, 9, 15, 13},
    {0, 3, 6, 5, 12, 15, 10, 9, 11, 8, 13, 14, 7, 4, 1, 2},
    {0, 4, 8, 12, 3, 7, 11, 15, 6, 2, 14, 10, 5, 1, 13, 9},
    {0, 5, 10, 15, 7, 2, 13, 8, 14, 11, 4, 1, 9, 12, 3, 6},
    {0, 6, 12, 10, 11, 13, 7, 1, 5, 3, 9, 15, 14, 8, 2, 4},
    {0, 7, 14, 9, 15, 8, 1, 6, 13, 10, 3, 4, 2, 5, 12, 11},
    {0, 8, 3, 11, 6, 14, 5, 13, 12, 4, 15, 7, 10, 2, 9, 1},
    {0, 9, 1, 8, 2, 11, 3, 10, 4, 13, 5, 12, 6, 15, 7, 14},
    {0, 10, 7, 13, 14, 4, 9, 3, 15, 5, 8, 2, 1, 11, 6, 12},
    {0, 11, 5, 14, 10, 1, 15, 4, 7, 12, 2, 9, 13, 6, 8, 3},
    {0, 12, 11, 7, 5, 9, 14, 2, 10, 6, 1, 13, 15, 3, 4, 8},
    {0, 13, 9, 4, 1, 12, 8, 5, 2, 15, 11, 6, 3, 14, 10, 7},
    {0, 14, 15, 1, 13, 3, 2, 12, 9, 7, 6, 8, 4, 10, 11, 5},
    {0, 15, 13, 2, 9, 6, 4, 11, 1, 14, 12, 3, 8, 7, 5, 10}};

// GF(2^4) multiplicative inverse table (for division)
static const uint8_t gf16_inv[16] = {0,	 1, 9,	14, 13, 11, 7, 6,
				     15, 2, 12, 5,  10, 4,  3, 8};

static int is_full_rank(const uint16_t matrix[64][64]) {
	uint8_t temp[64][64];
	for (int i = 0; i < 64; ++i) {
		for (int j = 0; j < 64; ++j) {
			temp[i][j] = matrix[i][j] & 0xF;
		}
	}
	for (int i = 0; i < 64; ++i) {
		int pivot = -1;
		for (int k = i; k < 64; ++k) {
			if (temp[k][i] != 0) {
				pivot = k;
				break;
			}
		}
		if (pivot == -1) return 0;
		if (pivot != i) {
			for (int j = 0; j < 64; ++j) {
				uint8_t t = temp[i][j];
				temp[i][j] = temp[pivot][j];
				temp[pivot][j] = t;
			}
		}
		uint8_t pivot_val = temp[i][i];
		uint8_t inv = gf16_inv[pivot_val];
		for (int j = 0; j < 64; ++j) {
			temp[i][j] = gf16_mult[temp[i][j]][inv];
		}
		for (int k = 0; k < 64; ++k) {
			if (k != i && temp[k][i] != 0) {
				uint8_t factor = temp[k][i];
				for (int j = 0; j < 64; ++j) {
					temp[k][j] ^=
					    gf16_mult[factor][temp[i][j]];
				}
			}
		}
	}
	return 1;
}

// Generates a 64x64 matrix of 4-bit values with full rank over GF(2^4)
void generate_matrix(uint16_t matrix[64][64], struct AESContext* aes) {
	memset(matrix, 0, 64 * 64 * sizeof(uint16_t));
	do {
		for (int i = 0; i < 64; ++i) {
			for (int j = 0; j < 64; j += 16) {
				// Generate 8 bytes (64 bits) of random data
				unsigned char bytes[8] = {0};

				aes_ctr_xcrypt_buffer(
				    aes, (unsigned char*)&bytes, 8);

				uint64_t value = 0;
				for (int k = 0; k < 8; ++k) {
					value |=
					    ((uint64_t)bytes[k] << (k * 8));
				}

				// Extract 16 4-bit values
				for (int shift = 0; shift < 16; ++shift) {
					matrix[i][j + shift] =
					    (value >> (4 * shift)) & 0xF;
				}
			}
		}
	} while (!is_full_rank(matrix));
}

