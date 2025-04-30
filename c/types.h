#ifndef _TYPES_H__
#define _TYPES_H__

#include "limits.h"

// primitives
typedef signed long long int64;
typedef unsigned long long uint64;
typedef unsigned short int uint16;
typedef unsigned char byte;
typedef unsigned long size_t;
// end primitives

// For platforms with non-standard types use this (comment out the above
// 'primitives' section): primitives
// #include <stddef.h>
// #include <stdint.h>
// typedef uint64_t uint64;
// typedef int64_t int64;
// typedef uint16_t uint16;
// typedef uint8_t byte;
// end primitives

// booleans
#define bool _Bool
#define true ((_Bool)1)
#define false ((_Bool)0)

// NULL
#ifndef NULL
#define NULL ((void *)0)
#endif	// NULL

#endif	// _TYPES_H__
