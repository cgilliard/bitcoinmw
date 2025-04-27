#include "backtrace.h"

#include <dlfcn.h>
#include <execinfo.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>

#ifdef __APPLE__
#include <mach-o/dyld.h>
#include <mach/mach.h>
#endif	// __APPLE__

#include "std.h"

#ifdef __APPLE__
char *get_binary_path() {
	static char path[PATH_MAX];
	uint32_t size = sizeof(path);
	if (_NSGetExecutablePath(path, &size) != 0) {
		fprintf(stderr, "Buffer too small or error occurred\n");
		return NULL;
	}
	return path;
}

char *gen_backtrace() {
	if (getenv("RUST_BACKTRACE") == NULL) return NULL;

	void *bt_entries[MAX_ENTRIES];
	int size = backtrace(bt_entries, MAX_ENTRIES);
	char *ret = NULL;

	char command[20 * MAX_ENTRIES + 128] = {0};

	snprintf(command, sizeof(command),
		 "atos -fullPath -o %s -l 0x100000000 ", get_binary_path());
	for (int i = 0; i < size; i++) {
		char address[256];
		Dl_info info;
		dladdr(bt_entries[i], &info);
		unsigned long long addr =
		    0x0000000100000000 + info.dli_saddr - info.dli_fbase;
		unsigned long long offset = (unsigned long long)bt_entries[i] -
					    (unsigned long long)info.dli_saddr;
		addr += offset;
		addr -= 4;
		int len = snprintf(address, sizeof(address), "0x%llx", addr);
		strcat(command, address);
		if (i != size - 1) strcat(command, " ");
	}

	FILE *fp = popen(command, "r");
	if (fp == NULL)
		return NULL;
	char buffer[MAX_LINE_LEN] = {0};
	int allocated = 0;
	const char *stack_backtrace_message = "stack backtrace: ";
	while (fgets(buffer, sizeof(buffer), fp) != NULL) {
		int needed = strlen(buffer);
		if (ret == NULL) {
			allocated = needed + 1 + strlen(stack_backtrace_message);
			ret = alloc(allocated);
			if (ret == NULL) {
				pclose(fp);
				return NULL;
			}
			strcpy(ret, stack_backtrace_message);
			strcat(ret, buffer);
		} else {
			allocated += needed + 1;
			char *tmp = resize(ret, allocated);
			if (tmp == NULL) {
				release(ret);
				pclose(fp);
				return NULL;
			}
			ret = tmp;
			strcat(ret, buffer);
		}
	}

	pclose(fp);

	return ret;
}
#endif	// __APPLE__
#ifdef __linux__
char *gen_backtrace() {
	char *ret = alloc(1000);
	if (ret != NULL) {
		strcpy(ret, "Backtrace not enaabled on linux yet");
	}
	return ret;
}
#endif	// __linux__
