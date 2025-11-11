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

#include <bmw/console.h>
#include <bmw/io.h>

u8 x[128] = {0};
u8 data[BLK_SIZE] = {0};

void main(void) {
	puts("=== RISC-V UART demo ===\n");
	puthex(0x1111, 4);
	blk_init();
	puts("\n");
	x[0] = 1;
	u64 v = *(u64 *)x;
	puthex(v, 24);
	puts("\nFirst line\n");
	puts("Second line\n");
	puts("All done!\n");

	data[0] = '4';
	data[1] = '5';
	data[2] = '6';
	blk_write(1, data);
	puts("write complete\n");

	u8 data2[BLK_SIZE];
	for (u32 i = 0; i < BLK_SIZE; i++) data2[i] = 0;
	blk_read(1, data2);
	putc(data[0]);
	putc(data[1]);
	putc(data[2]);
	puts("\n");

	abort();
}
