/* main.c */
#define UART_BASE 0x10000000UL
#define UART_THR (UART_BASE + 0) /* Transmit Holding Register */
#define UART_LSR (UART_BASE + 5) /* Line Status Register */
#define LSR_THRE (1U << 5)	 /* Transmit Holding Register Empty */

static inline void uart_putc(char c) {
	while (!(*(volatile unsigned char *)UART_LSR & LSR_THRE)); /* wait */
	*(volatile unsigned char *)UART_THR = c;
}

static void uart_puts(const char *s) {
	while (*s) {
		if (*s == '\n') uart_putc('\r'); /* CR before LF */
		uart_putc(*s++);
	}
}

/* --------------------------------------------------------------- */
/*  C entry point – called from _start				   */
/* --------------------------------------------------------------- */
void main(void) {
	uart_puts("=== RISC-V UART demo ===\n");
	uart_puts("First line\n");
	uart_puts("Second line\n");
	uart_puts("All done.\n");

	for (;;); /* hang */
}
