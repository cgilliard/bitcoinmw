void copy_bytes(unsigned char *X, const unsigned char *Y,
		unsigned long long x) {
	while (x--) *(X)++ = *(Y)++;
}
