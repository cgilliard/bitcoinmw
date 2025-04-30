#include "util.h"

unsigned long long cstring_len(const char *X) {
	const char *Y = X;
	while (*X) X++;
	return X - Y;
}

void ptr_add(void **p, long long v) { *p = (void *)((char *)*p + v); }

void copy_bytes(unsigned char *X, const unsigned char *Y,
		unsigned long long x) {
	while (x--) *(X)++ = *(Y)++;
}

