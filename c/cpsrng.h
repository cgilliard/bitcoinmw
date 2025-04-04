#ifndef _CPSRNG_H__
#define _CPSRNG_H__

#include "aes.h"
#include "types.h"

typedef struct CsprngCtx {
	struct AES_ctx ctx;
} CsprngCtx;

void cpsrng_reseed();
CsprngCtx *cpsrng_context_create();
void cpsrng_context_destroy(CsprngCtx *);
void cpsrng_rand_bytes(CsprngCtx *, void *v, unsigned long long size);

#ifdef TEST
void cpsrng_test_seed(CsprngCtx *ctx, byte iv[16], byte key[32]);
#endif	// TEST

#endif	// _CPSRNG_H__
