#ifndef _TYPES_H__
#define _TYPES_H__

#include "limits.h"

// primitives
typedef signed long long int64;
typedef unsigned char byte;
#define float64 double

// booleans
#define bool _Bool
#define true (_Bool)1
#define false (_Bool)0

// NULL
#ifndef NULL
#define NULL ((void *)0)
#endif	// NULL

#endif	// _TYPES_H__
