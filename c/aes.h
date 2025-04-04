// From repo: https://github.com/kokke/tiny-AES-c/
// License: https://github.com/kokke/tiny-AES-c/blob/master/unlicense.txt:
// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org/>

#ifndef _AES_H__
#define _AES_H__

#define AES_BLOCKLEN 16
#define AES_KEYLEN 32
#define AES_keyExpSize 240

struct AES_ctx {
	unsigned char RoundKey[AES_keyExpSize];
	unsigned char Iv[AES_BLOCKLEN];
};

unsigned long long AES_ctx_size();

void AES_init_ctx_iv(struct AES_ctx *ctx, const unsigned char *key,
		     const unsigned char *iv);
void AES_ctx_set_iv(struct AES_ctx *ctx, const unsigned char *iv);

void AES_CTR_xcrypt_buffer(struct AES_ctx *ctx, unsigned char *buf,
			   unsigned long long length);

#endif	// _AES_H__
