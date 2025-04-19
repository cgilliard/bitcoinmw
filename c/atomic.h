#ifndef _ATOMIC_H__
#define _ATOMIC_H__

void atomic_store_u64(unsigned long long *ptr, unsigned long long value);
unsigned long long atomic_load_u64(unsigned long long *ptr);
unsigned long long atomic_fetch_add_u64(unsigned long long *ptr,
					unsigned long long value);
unsigned long long atomic_fetch_sub_u64(unsigned long long *ptr,
					unsigned long long value);
unsigned long long cas_release(unsigned long long *ptr,
			       unsigned long long *expect,
			       unsigned long long desired);

#endif	// _ATOMIC_H__
