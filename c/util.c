#include "util.h"

int snprintf(char *s, unsigned long n, const char *format, ...);

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

void set_bytes(unsigned char *X, unsigned char x, unsigned long long y) {
	while (y--) *(X++) = x;
}

int f64_to_str(double d, char *buf, unsigned long long capacity) {
	return snprintf(buf, capacity, "%.5f", d);
}
