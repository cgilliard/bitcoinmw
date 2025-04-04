#include "util.h"

#include "limits.h"

void copy_bytes(unsigned char *X, const unsigned char *Y,
		unsigned long long x) {
	while (x--) *(X)++ = *(Y)++;
}

void set_bytes(unsigned char *X, unsigned char x, unsigned long long y) {
	while (y--) *(X++) = x;
}

unsigned long long cstring_len(const char *X) {
	const char *Y = X;
	while (*X) X++;
	return X - Y;
}

int cstring_compare(const char *X, const char *Y) {
	while (*X == *Y && *X) {
		X++;
		Y++;
	}
	if (*X > *Y) return 1;
	if (*Y > *X) return -1;
	return 0;
}

int cstring_compare_n(const char *X, const char *Y, unsigned long long n) {
	while (*X == *Y && n && *X) {
		n--;
		X++;
		Y++;
	}
	if (n == 0) return 0;
	if (*X > *Y) return 1;
	return -1;
}

const char *cstring_strstr(const char *X, const char *Y) {
	for (; *X; X++) {
		const char *tmpX = X, *tmpY = Y;
		while (*tmpX == *tmpY && *tmpX) {
			tmpX++;
			tmpY++;
		}
		if (!*tmpY) return X;
	}
	return 0;
}

void swap(unsigned char *X, unsigned long long x, unsigned long long y) {
	unsigned char t = X[x];
	X[x] = X[y];
	X[y] = t;
}

void reverse(unsigned char *X, long long length) {
	long long start = 0, end = length - 1;
	while (start < end) swap(X, start++, end--);
}

unsigned long long cstring_itoau64(unsigned long long num, char *X, int base,
				   unsigned long long capacity) {
	unsigned long long length = 0, i = 1;
	for (unsigned long long num_copy = num; num_copy; num_copy /= base)
		length++;
	while (capacity && num) {
		unsigned long long rem = num % base;
		X[length - i++] = (rem > 9) ? (rem - 10) + 'A' : rem + '0';
		num /= base;
		capacity--;
	}
	if (length == 0 && capacity) X[length++] = '0';
	if (length == 0) length++;
	return length;
}

unsigned long long cstring_itoai64(long long num, char *X, int base,
				   unsigned long long capacity) {
	if (num < 0) {
		if (num == INT64_MIN)
			num = INT64_MAX;
		else
			num *= -1;
		if (capacity) {
			capacity -= 1;
			X[0] = '-';
		}
		return cstring_itoau64(num, X + 1, base, capacity) + 1;
	} else
		return cstring_itoau64(num, X, base, capacity);
}

unsigned long long cstring_strtoull(const char *X, int base) {
	unsigned long long ret = 0, mul = 1, len = cstring_len(X);
	while (len-- && X[len] != 'x') {
		ret += X[len] > '9' ? ((X[len] - 'a') + 10) * mul
				    : (X[len] - '0') * mul;
		mul *= base;
	}
	return ret;
}

void cstring_cat_n(char *X, char *Y, unsigned long long n) {
	X += cstring_len(X);
	while (n-- && *Y) {
		*X = *Y;
		X++;
		Y++;
	}
	*X = 0;
}

int cstring_char_is_alpha_numeric(char ch) {
	if (ch >= 'a' && ch <= 'z') return 1;
	if (ch >= 'A' && ch <= 'Z') return 1;
	if (ch >= '0' && ch <= '9') return 1;
	if (ch == '_' || ch == '\n') return 1;
	return 0;
}
int cstring_is_alpha_numeric(const char *X) {
	if (*X >= '0' && *X <= '9') return 0;
	while (*X)
		if (!cstring_char_is_alpha_numeric(*X++)) return 0;

	return 1;
}

typedef unsigned long size_t;
int snprintf(char *s, size_t n, const char *format, ...);

int f64_to_str(double d, char *buf, unsigned long long capacity) {
	return snprintf(buf, capacity, "%.5f", d);
}

void ptr_add(void **p, long long v) { *p = (void *)((char *)*p + v); }
