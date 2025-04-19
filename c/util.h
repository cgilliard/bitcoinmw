#ifndef _UTIL_H__
#define _UTIL_H__

void copy_bytes(unsigned char *dest, const unsigned char *src,
		unsigned long long n);
void set_bytes(unsigned char *dst, unsigned char b, unsigned long long n);
unsigned long long cstring_len(const char *S);
int cstring_compare(const char *s1, const char *s2);
int cstring_compare_n(const char *s1, const char *s2, unsigned long long n);
void cstring_cat_n(char *s1, char *s2, unsigned long long n);
const char *cstring_strstr(const char *X, const char *Y);
void reverse(unsigned char *str, long long end);
unsigned long long cstring_itoau64(unsigned long long num, char *str, int base,
				   unsigned long long capacity);
unsigned long long cstring_itoai64(long long num, char *str, int base,
				   unsigned long long capacity);
unsigned long long cstring_strtoull(const char *str, int base);
int cstring_is_alpha_numeric(const char *str);

#endif	// _UTIL_H__
