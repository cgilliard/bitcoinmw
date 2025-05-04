#include <signal.h>

int printf(const char *, ...);
void panic_impl() { printf("Rust panic occurred\n"); }
void rust_eh_personality() {}

extern int real_main(int argc, char *argv[]);
int main(int argc, char *argv[]) {
	signal(SIGPIPE, SIG_IGN);
	return real_main(argc, argv);
}
