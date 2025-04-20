#ifndef _SHA3_H__
#define _SHA3_H__

#include "types.h"

/* -------------------------------------------------------------------------
 * Works when compiled for either 32-bit or 64-bit targets, optimized for
 * 64 bit.
 *
 * Canonical implementation of Init/Update/Finalize for SHA-3 byte input.
 *
 * SHA3-256, SHA3-384, SHA-512 are implemented. SHA-224 can easily be added.
 *
 * Based on code from http://keccak.noekeon.org/ .
 *
 * I place the code that I wrote into public domain, free to use.
 *
 * I would appreciate if you give credits to this work if you used it to
 * write or test * your code.
 *
 * Aug 2015. Andrey Jivsov. crypto@brainhub.org
 * ---------------------------------------------------------------------- */

/* 'Words' here refers to unsigned long long */
#define SHA3_KECCAK_SPONGE_WORDS \
	(((1600) / 8 /*bits to byte*/) / sizeof(unsigned long long))
typedef struct sha3_context_ {
	unsigned long long saved; /* the portion of the input message that we
				   * didn't consume yet */
	union {			  /* Keccak's state */
		unsigned long long s[SHA3_KECCAK_SPONGE_WORDS];
		byte sb[SHA3_KECCAK_SPONGE_WORDS * 8];
	} u;
	unsigned byteIndex;	/* 0..7--the next byte after the set one
				 * (starts from 0; 0--none are buffered) */
	unsigned wordIndex;	/* 0..24--the next word to integrate input
				 * (starts from 0) */
	unsigned capacityWords; /* the double size of the hash output in
				 * words (e.g. 16 for Keccak 512) */
} sha3_context;

enum SHA3_FLAGS { SHA3_FLAGS_NONE = 0, SHA3_FLAGS_KECCAK = 1 };
enum SHA3_RETURN { SHA3_RETURN_OK = 0, SHA3_RETURN_BAD_PARAMS = 1 };
typedef enum SHA3_RETURN sha3_return_t;

// Return size of the sha3_context (for allocation purposes)
unsigned long long sha3_context_size();

// For Init or Reset call this function
sha3_return_t sha3_init(void *priv, unsigned bitSize);
void sha3_init256(void *priv);
void sha3_init384(void *priv);
void sha3_init512(void *priv);

// Set flags (keccak or none = NIST)
enum SHA3_FLAGS sha3_setflags(void *priv, enum SHA3_FLAGS);

// Update the hasher with a value
void sha3_update(void *priv, void const *bufIn, unsigned long long len);

// Finalize the hash
void const *sha3_finalize(void *priv);

#endif	// _SHA3_H__
