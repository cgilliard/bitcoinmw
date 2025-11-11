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

#ifndef _CONSOLE_H
#define _CONSOLE_H

#include <bmw/types.h>

#define UART_BASE 0x10000000UL
#define UART_THR (UART_BASE + 0)
#define UART_LSR (UART_BASE + 5)
#define LSR_THRE (1U << 5)
#define SHUTDOWN_ADDR 0x100000
#define SHUTDOWN_VAL 0x5555

static inline void putc(u8 c) {
	while (!(*(volatile u8 *)UART_LSR & LSR_THRE));
	*(volatile u8 *)UART_THR = c;
}

static inline void puts(const u8 *s) {
	while (*s) putc(*s++);
}

static inline void idle(void) { asm volatile("wfi" ::: "memory"); }

static inline void abort(void) {
	volatile u32 *shutdown_reg = (volatile u32 *)SHUTDOWN_ADDR;
	*shutdown_reg = SHUTDOWN_VAL;
	while (1) idle();
}

static inline void puthex(u64 value, i32 digits) {
	const u8 *hex = "0123456789abcdef";
	u8 buf[16] = {0};
	i32 i = 0;

	if (value == 0)
		for (i32 j = 0; j < digits && j < 16; j++) putc('0');
	else {
		for (; value > 0 && i < 16; value >>= 4)
			buf[i++] = hex[value & 0xF];
		while (i < digits) buf[i++] = '0';
		while (i > 0) putc(buf[--i]);
	}
}

#endif /* _CONSOLE_H */
