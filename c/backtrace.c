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
	char buffer[MAX_LINE_LEN] = {0};
	while (fgets(buffer, sizeof(buffer), fp) != NULL) {
		int len = strlen(buffer);
		if (ret == NULL) {
			ret = alloc(len + 30);
			if (ret == NULL) {
				return NULL;
			}
			strcpy(ret, "stack backtrace: ");
			strcat(ret, buffer);
		} else {
			int nsize = strlen(ret) + len + 2;
			char *tmp = resize(ret, nsize);
			if (tmp == NULL) {
				release(ret);
				return NULL;
			}
			ret = tmp;
			strcat(ret, buffer);
		}
	}

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
