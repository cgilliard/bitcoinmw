#include "atomic.h"

void atomic_store_u64(unsigned long long *ptr, unsigned long long value) {
	__atomic_store_n(ptr, value, __ATOMIC_RELEASE);
}
unsigned long long atomic_load_u64(unsigned long long *ptr) {
	return __atomic_load_n(ptr, __ATOMIC_ACQUIRE);
}
unsigned long long atomic_fetch_add_u64(unsigned long long *ptr,
					unsigned long long value) {
	return __atomic_fetch_add(ptr, value, __ATOMIC_SEQ_CST);
}
unsigned long long atomic_fetch_sub_u64(unsigned long long *ptr,
					unsigned long long value) {
	return __atomic_fetch_sub(ptr, value, __ATOMIC_SEQ_CST);
}
unsigned long long cas_release(unsigned long long *ptr,
			       unsigned long long *expect,
			       unsigned long long desired) {
	unsigned long long ret = __atomic_compare_exchange_n(
	    ptr, expect, desired, 0, __ATOMIC_RELEASE, __ATOMIC_RELAXED);

	return ret;
}
