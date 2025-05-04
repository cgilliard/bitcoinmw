#ifndef _UTIL_H__
#define _UTIL_H__

unsigned long long cstring_len(const char *S);
void ptr_add(void **p, long long v);
void copy_bytes(unsigned char *X, const unsigned char *Y, unsigned long long x);
void set_bytes(unsigned char *X, unsigned char x, unsigned long long y);
int f64_to_str(double d, char *buf, unsigned long long capacity);

#endif	// _UTIL_H__
