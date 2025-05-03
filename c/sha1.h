#ifndef _SHA1_H__
#define _SHA1_H__

#include "types.h"

void sha1(const unsigned char *data, size_t size, unsigned char hash[]);

#endif	// _SHA1_H__
