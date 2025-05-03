#ifndef _STD_H__
#define _STD_H__

void *alloc(unsigned long size);
void release(void *ptr);
void *resize(void *ptr, unsigned long long len);
unsigned long long getmicros();
int sleep_millis(unsigned long long millis);
int rand_bytes(unsigned char *buf, unsigned long long length);

#endif	// _STD_H__
