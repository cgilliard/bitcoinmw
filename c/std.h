#ifndef _STD_H__
#define _STD_H__

#include <stdio.h>

void *malloc(unsigned long);
void *realloc(void *ptr, unsigned long);
void free(void *);
int getentropy(void *buf, unsigned long long length);
// int snprintf(char *s, unsigned long n, const char *format, ...);

void *alloc(unsigned long size);
void release(void *ptr);
void *resize(void *ptr, unsigned long long len);
unsigned long long getmicros();
int sleep_millis(unsigned long long millis);
int rand_bytes(unsigned char *buf, unsigned long long length);

#endif	// _STD_H__
