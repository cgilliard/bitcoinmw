#ifndef _LIMITS_H__
#define _LIMITS_H__

#ifndef INT64_MAX
#define INT64_MAX ((long long)0x7FFFFFFFFFFFFFFFLL)
#endif

#ifndef INT64_MIN
#define INT64_MIN (-INT64_MAX - 1)
#endif

#ifndef INT32_MAX
#define INT32_MAX ((int)0x7FFFFFFF)
#endif

#ifndef INT32_MIN
#define INT32_MIN (-INT32_MAX - 1)
#endif

#ifndef UINT64_MAX
#define UINT64_MAX ((unsigned int)0xFFFFFFFFFFFFFFFFULL)
#endif

#ifndef UINT32_MAX
#define UINT32_MAX ((unsigned int)0xFFFFFFFFULL)
#endif

#endif	// _LIMITS_H__
