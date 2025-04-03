#include <stdio.h>

extern int real_main(int argc, char *argv[]);
int real_main(int argc, char **argv) {
	printf("bitcoinmw...\n");
	return 0;
}
int main(int argc, char *argv[]) { return real_main(argc, argv); }
