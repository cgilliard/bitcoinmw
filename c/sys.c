#include <time.h>

int printf(const char *, ...);
void *malloc(unsigned long);
void *realloc(void *ptr, unsigned long);
void free(void *);
int getentropy(void *buf, unsigned long long length);
long long __alloc_count = 0;
void _exit(int);

void *alloc(unsigned long size) {
	void *ptr = malloc(size);
	// printf("malloc %p (%lu) (alloc=%lli)\n", ptr, size, __alloc_count);
#ifdef TEST
	__atomic_fetch_add(&__alloc_count, 1, __ATOMIC_SEQ_CST);
#endif	// TEST
	return ptr;
}

void release(void *ptr) {
	// printf("free %p,alloc=%lli\n", ptr, __alloc_count);
#ifdef TEST
	__atomic_fetch_sub(&__alloc_count, 1, __ATOMIC_SEQ_CST);
#endif	// TEST
	free(ptr);
}

void *resize(void *ptr, unsigned long long len) {
	void *ret = realloc(ptr, len);
	// printf("realloc size=%llu [%p -> %p]\n", len, ptr, ret);
	return ret;
}

unsigned long long getmicros() {
	struct timespec now;
	clock_gettime(CLOCK_REALTIME, &now);
	return (unsigned long long)((__int128_t)now.tv_sec * 1000000) +
	       (unsigned long long)(now.tv_nsec / 1000);
}

int sleep_millis(unsigned long long millis) {
	struct timespec ts;
	ts.tv_sec = millis / 1000;
	ts.tv_nsec = (millis % 1000) * 1000000;
	int ret = nanosleep(&ts, 0);
	return ret;
}

int rand_bytes(unsigned char *buf, unsigned long long length) {
	return getentropy(buf, length);
}

long long getalloccount() { return __alloc_count; }

