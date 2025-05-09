#include "sha3.h"

#include "util.h"

#define SHA3_ASSERT(x)
#define SHA3_TRACE(format, ...)
#define SHA3_TRACE_BUF(format, buf, l)

/*
 * This flag is used to configure "pure" Keccak, as opposed to NIST SHA3.
 */
#define SHA3_USE_KECCAK_FLAG 0x80000000
#define SHA3_CW(x) ((x) & (~SHA3_USE_KECCAK_FLAG))

#if defined(_MSC_VER)
#define SHA3_CONST(x) x
#else
#define SHA3_CONST(x) x##L
#endif

#ifndef SHA3_ROTL64
#define SHA3_ROTL64(x, y) \
	(((x) << (y)) | ((x) >> ((sizeof(unsigned long long) * 8) - (y))))
#endif

static const unsigned long long keccakf_rndc[24] = {
    SHA3_CONST(0x0000000000000001UL), SHA3_CONST(0x0000000000008082UL),
    SHA3_CONST(0x800000000000808aUL), SHA3_CONST(0x8000000080008000UL),
    SHA3_CONST(0x000000000000808bUL), SHA3_CONST(0x0000000080000001UL),
    SHA3_CONST(0x8000000080008081UL), SHA3_CONST(0x8000000000008009UL),
    SHA3_CONST(0x000000000000008aUL), SHA3_CONST(0x0000000000000088UL),
    SHA3_CONST(0x0000000080008009UL), SHA3_CONST(0x000000008000000aUL),
    SHA3_CONST(0x000000008000808bUL), SHA3_CONST(0x800000000000008bUL),
    SHA3_CONST(0x8000000000008089UL), SHA3_CONST(0x8000000000008003UL),
    SHA3_CONST(0x8000000000008002UL), SHA3_CONST(0x8000000000000080UL),
    SHA3_CONST(0x000000000000800aUL), SHA3_CONST(0x800000008000000aUL),
    SHA3_CONST(0x8000000080008081UL), SHA3_CONST(0x8000000000008080UL),
    SHA3_CONST(0x0000000080000001UL), SHA3_CONST(0x8000000080008008UL)};

static const unsigned keccakf_rotc[24] = {1,  3,  6,  10, 15, 21, 28, 36,
					  45, 55, 2,  14, 27, 41, 56, 8,
					  25, 43, 62, 18, 39, 61, 20, 44};

static const unsigned keccakf_piln[24] = {10, 7,  11, 17, 18, 3,  5,  16,
					  8,  21, 24, 4,  15, 23, 19, 13,
					  12, 2,  20, 14, 22, 9,  6,  1};

/* generally called after SHA3_KECCAK_SPONGE_WORDS-ctx->capacityWords words
 * are XORed into the state s
 */
static void keccakf(unsigned long long s[25]) {
	int i, j, round;
	unsigned long long t, bc[5];
#define KECCAK_ROUNDS 24

	for (round = 0; round < KECCAK_ROUNDS; round++) {
		/* Theta */
		for (i = 0; i < 5; i++)
			bc[i] =
			    s[i] ^ s[i + 5] ^ s[i + 10] ^ s[i + 15] ^ s[i + 20];

		for (i = 0; i < 5; i++) {
			t = bc[(i + 4) % 5] ^ SHA3_ROTL64(bc[(i + 1) % 5], 1);
			for (j = 0; j < 25; j += 5) s[j + i] ^= t;
		}

		/* Rho Pi */
		t = s[1];
		for (i = 0; i < 24; i++) {
			j = keccakf_piln[i];
			bc[0] = s[j];
			s[j] = SHA3_ROTL64(t, keccakf_rotc[i]);
			t = bc[0];
		}

		/* Chi */
		for (j = 0; j < 25; j += 5) {
			for (i = 0; i < 5; i++) bc[i] = s[j + i];
			for (i = 0; i < 5; i++)
				s[j + i] ^=
				    (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
		}

		/* Iota */
		s[0] ^= keccakf_rndc[round];
	}
}

/* *************************** Public Inteface ************************ */

sha3_return_t sha3_Squeeze(void *priv, void *out, unsigned outBytes) {
	Sha3Context *ctx = (Sha3Context *)priv;
	unsigned char *output = (unsigned char *)out;
	unsigned bytesGenerated = 0;

	// Ensure Finalize was called and state is ready for squeezing
	if (ctx->byteIndex != 0 || ctx->wordIndex != 0) {
		return SHA3_RETURN_BAD_PARAMS;	// Not ready for squeezing
	}

	while (bytesGenerated < outBytes) {
		// Calculate how many bytes to copy in this iteration
		unsigned chunkSize = outBytes - bytesGenerated;
		if (chunkSize > sizeof(ctx->u.sb)) {
			chunkSize = sizeof(ctx->u.sb);
		}

		// Copy the current state to output
		copy_bytes(output + bytesGenerated, ctx->u.sb, chunkSize);
		bytesGenerated += chunkSize;

		// Permute the state for the next chunk
		keccakf(ctx->u.s);
	}

	return SHA3_RETURN_OK;
}

unsigned long long sha3_context_size() { return sizeof(Sha3Context); }

/* For Init or Reset call these: */
sha3_return_t sha3_init(void *priv, unsigned bitSize) {
	Sha3Context *ctx = (Sha3Context *)priv;
	if (bitSize != 256 && bitSize != 384 && bitSize != 512)
		return SHA3_RETURN_BAD_PARAMS;
	set_bytes((byte *)ctx, 0, sizeof(*ctx));
	ctx->capacityWords = 2 * bitSize / (8 * sizeof(unsigned long long));
	return SHA3_RETURN_OK;
}

void sha3_init256(void *priv) { sha3_init(priv, 256); }

void sha3_init384(void *priv) { sha3_init(priv, 384); }

void sha3_init512(void *priv) { sha3_init(priv, 512); }

enum SHA3_FLAGS sha3_setflags(void *priv, enum SHA3_FLAGS flags) {
	Sha3Context *ctx = (Sha3Context *)priv;
	flags &= SHA3_FLAGS_KECCAK;
	ctx->capacityWords |=
	    (flags == SHA3_FLAGS_KECCAK ? SHA3_USE_KECCAK_FLAG : 0);
	return flags;
}

void sha3_update(void *priv, void const *bufIn, unsigned long long len) {
	Sha3Context *ctx = (Sha3Context *)priv;

	/* 0...7 -- how much is needed to have a word */
	unsigned old_tail = (8 - ctx->byteIndex) & 7;

	unsigned long long words;
	unsigned tail;
	unsigned long long i;

	const byte *buf = bufIn;

	SHA3_TRACE_BUF("called to update with:", buf, len);

	SHA3_ASSERT(ctx->byteIndex < 8);
	SHA3_ASSERT(ctx->wordIndex < sizeof(ctx->u.s) / sizeof(ctx->u.s[0]));

	if (len < old_tail) { /* have no complete word or haven't started
			       * the word yet */
		SHA3_TRACE("because %d<%d, store it and return", (unsigned)len,
			   (unsigned)old_tail);
		/* endian-independent code follows: */
		while (len--)
			ctx->saved |= (unsigned long long)(*(buf++))
				      << ((ctx->byteIndex++) * 8);
		SHA3_ASSERT(ctx->byteIndex < 8);
		return;
	}

	if (old_tail) { /* will have one word to process */
		SHA3_TRACE("completing one word with %d bytes",
			   (unsigned)old_tail);
		/* endian-independent code follows: */
		len -= old_tail;
		while (old_tail--)
			ctx->saved |= (unsigned long long)(*(buf++))
				      << ((ctx->byteIndex++) * 8);

		/* now ready to add saved to the sponge */
		ctx->u.s[ctx->wordIndex] ^= ctx->saved;
		SHA3_ASSERT(ctx->byteIndex == 8);
		ctx->byteIndex = 0;
		ctx->saved = 0;
		if (++ctx->wordIndex ==
		    (SHA3_KECCAK_SPONGE_WORDS - SHA3_CW(ctx->capacityWords))) {
			keccakf(ctx->u.s);
			ctx->wordIndex = 0;
		}
	}

	/* now work in full words directly from input */

	SHA3_ASSERT(ctx->byteIndex == 0);

	words = len / sizeof(unsigned long long);
	tail = len - words * sizeof(unsigned long long);

	SHA3_TRACE("have %d full words to process", (unsigned)words);

	for (i = 0; i < words; i++, buf += sizeof(unsigned long long)) {
		const unsigned long long t =
		    (unsigned long long)(buf[0]) |
		    ((unsigned long long)(buf[1]) << 8 * 1) |
		    ((unsigned long long)(buf[2]) << 8 * 2) |
		    ((unsigned long long)(buf[3]) << 8 * 3) |
		    ((unsigned long long)(buf[4]) << 8 * 4) |
		    ((unsigned long long)(buf[5]) << 8 * 5) |
		    ((unsigned long long)(buf[6]) << 8 * 6) |
		    ((unsigned long long)(buf[7]) << 8 * 7);
#if defined(__x86_64__) || defined(__i386__)
		SHA3_ASSERT(memcmp(&t, buf, 8) == 0);
#endif
		ctx->u.s[ctx->wordIndex] ^= t;
		if (++ctx->wordIndex ==
		    (SHA3_KECCAK_SPONGE_WORDS - SHA3_CW(ctx->capacityWords))) {
			keccakf(ctx->u.s);
			ctx->wordIndex = 0;
		}
	}

	SHA3_TRACE("have %d bytes left to process, save them", (unsigned)tail);

	/* finally, save the partial word */
	SHA3_ASSERT(ctx->byteIndex == 0 && tail < 8);
	while (tail--) {
		SHA3_TRACE("Store byte %02x '%c'", *buf, *buf);
		ctx->saved |= (unsigned long long)(*(buf++))
			      << ((ctx->byteIndex++) * 8);
	}
	SHA3_ASSERT(ctx->byteIndex < 8);
	SHA3_TRACE("Have saved=0x%016" PRIx64 " at the end", ctx->saved);
}

/* This is simply the 'update' with the padding block.
 * The padding block is 0x01 || 0x00* || 0x80. First 0x01 and last 0x80
 * bytes are always present, but they can be the same byte.
 */
void const *sha3_finalize(void *priv) {
	Sha3Context *ctx = (Sha3Context *)priv;

	SHA3_TRACE("called with %d bytes in the buffer", ctx->byteIndex);

	/* Append 2-bit suffix 01, per SHA-3 spec. Instead of 1 for padding we
	 * use 1<<2 below. The 0x02 below corresponds to the suffix 01.
	 * Overall, we feed 0, then 1, and finally 1 to start padding. Without
	 * M || 01, we would simply use 1 to start padding. */

	unsigned long long t;

	if (ctx->capacityWords & SHA3_USE_KECCAK_FLAG) {
		/* Keccak version */
		t = (unsigned long long)(((unsigned long long)1)
					 << (ctx->byteIndex * 8));
	} else {
		/* SHA3 version */
		t = (unsigned long long)(((unsigned long long)(0x02 | (1 << 2)))
					 << ((ctx->byteIndex) * 8));
	}

	ctx->u.s[ctx->wordIndex] ^= ctx->saved ^ t;

	ctx->u.s[SHA3_KECCAK_SPONGE_WORDS - SHA3_CW(ctx->capacityWords) - 1] ^=
	    SHA3_CONST(0x8000000000000000UL);
	keccakf(ctx->u.s);

	/* Return first bytes of the ctx->s. This conversion is not needed for
	 * little-endian platforms e.g. wrap with #if !defined(__BYTE_ORDER__)
	 * || !defined(__ORDER_LITTLE_ENDIAN__) ||
	 * __BYTE_ORDER__!=__ORDER_LITTLE_ENDIAN__
	 *    ... the conversion below ...
	 * #endif */
	{
		unsigned i;
		for (i = 0; i < SHA3_KECCAK_SPONGE_WORDS; i++) {
			const unsigned t1 = (unsigned int)ctx->u.s[i];
			const unsigned t2 =
			    (unsigned int)((ctx->u.s[i] >> 16) >> 16);
			ctx->u.sb[i * 8 + 0] = (byte)(t1);
			ctx->u.sb[i * 8 + 1] = (byte)(t1 >> 8);
			ctx->u.sb[i * 8 + 2] = (byte)(t1 >> 16);
			ctx->u.sb[i * 8 + 3] = (byte)(t1 >> 24);
			ctx->u.sb[i * 8 + 4] = (byte)(t2);
			ctx->u.sb[i * 8 + 5] = (byte)(t2 >> 8);
			ctx->u.sb[i * 8 + 6] = (byte)(t2 >> 16);
			ctx->u.sb[i * 8 + 7] = (byte)(t2 >> 24);
		}
	}

	SHA3_TRACE_BUF("Hash: (first 32 bytes)", ctx->u.sb, 256 / 8);

	return (ctx->u.sb);
}

sha3_return_t sha3_HashBuffer(unsigned bitSize, enum SHA3_FLAGS flags,
			      const void *in, unsigned inBytes, void *out,
			      unsigned outBytes) {
	sha3_return_t err;
	Sha3Context c;

	err = sha3_init(&c, bitSize);
	if (err != SHA3_RETURN_OK) return err;
	if (sha3_setflags(&c, flags) != flags) {
		return SHA3_RETURN_BAD_PARAMS;
	}
	sha3_update(&c, in, inBytes);
	const void *h = sha3_finalize(&c);

	if (outBytes > bitSize / 8) outBytes = bitSize / 8;
	copy_bytes(out, h, outBytes);
	return SHA3_RETURN_OK;
}

sha3_return_t sha3_HashBuffer_sq(unsigned bitSize, enum SHA3_FLAGS flags,
				 const void *in, unsigned inBytes, void *out,
				 unsigned outBytes) {
	Sha3Context ctx;
	if (sha3_init(&ctx, bitSize) != SHA3_RETURN_OK) {
		return SHA3_RETURN_BAD_PARAMS;
	}

	sha3_update(&ctx, in, inBytes);

	// Finalize the state
	if (sha3_finalize(&ctx) == NULL) {
		return SHA3_RETURN_BAD_PARAMS;
	}

	// Squeeze output if SHAKE mode
	if (flags == SHA3_FLAGS_KECCAK) {
		return sha3_Squeeze(&ctx, out, outBytes);
	} else {
		// Standard SHA3 with truncation
		copy_bytes(out, ctx.u.sb, outBytes);
		return SHA3_RETURN_OK;
	}
}

