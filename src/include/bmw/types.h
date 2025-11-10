/********************************************************************************
 * MIT License
 *
 * Copyright (c) 2025 Christopher Gilliard
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 *******************************************************************************/

#ifndef _TYPES_H
#define _TYPES_H

/*
 * Constant: TEST
 * Controls test mode for the library.
 * notes:
 *         Default: 0 (disabled).
 *         Set to 1 to enable test-only features (e.g., rng_test_seed).
 *         Example: gcc -DTEST=1 ...
 */
#ifndef TEST
#define TEST 0
#endif /* TEST */

/*
 * Constant: NULL
 * Null pointer constant.
 * notes:
 *         Defined if not already provided.
 */
#ifndef NULL
#define NULL ((void *)0)
#endif /* NULL */

/*
 * Type: i8
 * 8-bit signed integer.
 */
typedef signed char i8;

/*
 * Type: i16
 * 16-bit signed integer.
 */
typedef short int i16;

/*
 * Type: i32
 * 32-bit signed integer.
 */
typedef int i32;

/*
 * Type: i64
 * 64-bit signed integer.
 */
typedef long i64;

/*
 * Type: i128
 * 128-bit signed integer.
 */
typedef __int128_t i128;

/*
 * Type: u8
 * 8-bit unsigned integer.
 */
typedef unsigned char u8;

/*
 * Type: u16
 * 16-bit unsigned integer.
 */
typedef unsigned short int u16;

/*
 * Type: u32
 * 32-bit unsigned integer.
 */
typedef unsigned int u32;

/*
 * Type: u64
 * 64-bit unsigned integer.
 */
typedef unsigned long u64;

/*
 * Type: u128
 * 128-bit unsigned integer.
 */
typedef __uint128_t u128;

/*
 * Type: f64
 * 64-bit IEEE 754 floating-point.
 */
typedef double f64;

/*
 * Type: bool
 * Boolean type (u8).
 * notes:
 *         Defined if not already provided.
 */
#ifndef bool
#define bool u8
#endif

/*
 * Constant: false
 * Boolean false (0).
 */
#ifndef false
#define false (bool)0
#endif

/*
 * Constant: true
 * Boolean true (1).
 */
#ifndef true
#define true (bool)1
#endif

#endif /* _TYPES_H */
