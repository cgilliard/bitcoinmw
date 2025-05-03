#include "backtrace.h"

#include <dlfcn.h>
#include <execinfo.h>
#include <limits.h>
#include <stdio.h>
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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char *format_output(const char *in) {
	// Calculate required size for output
	size_t len = strlen(in);
	size_t out_size = len * 2;  // Estimate: enough for reformatting
	char *ret = malloc(out_size);
	if (!ret) return NULL;

	// Initialize output buffer
	char *out = ret;
	out[0] = '\0';

	// Add header
	strcat(out, "stack backtrace:\n");
	out += strlen(out);

	// Process input line by line
	const char *line = in;
	int frame_num = 0;

	while (line && *line) {
		// Find end of current line
		const char *eol = strchr(line, '\n');
		size_t line_len = eol ? (size_t)(eol - line) : strlen(line);

		// Skip empty lines and header
		if (line_len == 0 ||
		    strncmp(line, "stack backtrace:", 16) == 0) {
			line = eol ? eol + 1 : NULL;
			continue;
		}

		// Create temporary buffer for line
		char *temp = malloc(line_len + 1);
		if (!temp) {
			free(ret);
			return NULL;
		}
		strncpy(temp, line, line_len);
		temp[line_len] = '\0';

		// Extract function name
		char *func_start = temp;
		char *module_start = strstr(temp, " (");
		char *module_end =
		    module_start ? strstr(module_start, ")") : NULL;
		if (module_start && module_end) {
			*module_start = '\0';  // Terminate function name
		} else {
			// Handle lines without module info (e.g., raw
			// addresses)
			snprintf(out, out_size - (out - ret), "#%d: %s\n",
				 frame_num++, func_start);
			out += strlen(out);
			free(temp);
			line = eol ? eol + 1 : NULL;
			continue;
		}

		// Extract file path and line number if present
		char *path = NULL;
		char *path_start = strstr(line, " (/");
		char *path_end = path_start ? strstr(path_start, ")") : NULL;
		if (path_start && path_end && path_start + 2 < path_end) {
			path_start += 2;  // Skip " (/"
			size_t path_len = path_end - path_start;
			path = malloc(path_len + 1);
			if (path) {
				strncpy(path, path_start, path_len);
				path[path_len] = '\0';
			}
		}

		// Format output
		if (path) {
			snprintf(out, out_size - (out - ret),
				 "#%d: %s\n    %s\n", frame_num++, func_start,
				 path);
			free(path);
		} else {
			snprintf(out, out_size - (out - ret), "#%d: %s\n",
				 frame_num++, func_start);
		}
		out += strlen(out);

		free(temp);
		line = eol ? eol + 1 : NULL;
	}

	return ret;
}

char *gen_backtrace(void **bt_entries, int size) {
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
	if (fp == NULL) return NULL;
	char buffer[MAX_LINE_LEN] = {0};
	int allocated = 0;
	const char *stack_backtrace_message = "stack backtrace: ";
	while (fgets(buffer, sizeof(buffer), fp) != NULL) {
		int needed = strlen(buffer);
		if (ret == NULL) {
			allocated =
			    needed + 1 + strlen(stack_backtrace_message);
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

	if (ret) {
		char *tmp = format_output(ret);
		release(ret);
		ret = tmp;
	}

	return ret;
}
#endif	// __APPLE__
#ifdef __linux__
char *gen_backtrace(void **bt_entries, int size) {
	char *ret = alloc(1000);
	if (ret != NULL) {
		strcpy(ret, "Backtrace not enaabled on linux yet");
	}
	return ret;
}
#endif	// __linux__
