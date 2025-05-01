#include <signal.h>

extern int real_main(int argc, char *argv[]);
int main(int argc, char *argv[]) {
	signal(SIGPIPE, SIG_IGN);
	return real_main(argc, argv);
}
