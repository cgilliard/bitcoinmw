#ifndef _ATOMIC_H__
#define _ATOMIC_H__

#define u64 unsigned long long

void atomic_store_u64(u64 *ptr, u64 value);
u64 atomic_load_u64(u64 *ptr);
u64 atomic_fetch_add_u64(u64 *ptr, u64 value);
u64 atomic_fetch_sub_u64(u64 *ptr, u64 value);
u64 cas_release(u64 *ptr, u64 *expect, u64 desired);

#endif	// _ATOMIC_H__
