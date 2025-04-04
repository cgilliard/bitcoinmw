#include "cpsrng.h"

#include "aes.h"

void _exit(int);
int printf(const char *, ...);
int rand_bytes(unsigned char *buf, unsigned long long length);
void *alloc(unsigned long size);
void release(void *);

CsprngCtx *cpsrng_context_create() {
	CsprngCtx *ret = alloc(sizeof(CsprngCtx));
	if (ret) {
		byte iv[16];
		byte key[32];
		if (rand_bytes(key, 32)) {
			release(ret);
			return NULL;
		}
		if (rand_bytes(iv, 16)) {
			release(ret);
			return NULL;
		}

		AES_init_ctx_iv(&ret->ctx, key, iv);
	}
	return ret;
}
void cpsrng_context_destroy(CsprngCtx *ctx) { release(ctx); }
void cpsrng_rand_bytes(CsprngCtx *ctx, void *v, unsigned long long size) {
	AES_CTR_xcrypt_buffer(&ctx->ctx, (byte *)v, size);
}

// only available in test mode for tests. Not used in production environments.
#ifdef TEST
void cpsrng_test_seed(CsprngCtx *ctx, byte iv[16], byte key[32]) {
	AES_init_ctx_iv(&ctx->ctx, key, iv);
	unsigned char v0[1] = {0};
	cpsrng_rand_bytes(ctx, &v0, 1);
}
#endif	// TEST
