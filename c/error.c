#include <execinfo.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Custom allocator
void *alloc(unsigned long long size);
void release(void *);

// Format error with backtrace
char *format_err(const char *kind, int len) {
	if (kind == NULL || len < 0) {
		return NULL;
	}
	int actual_len = strnlen(kind, len);
	if (actual_len >= len && kind[len] != '\0') {
		return NULL;
	}

	// Compute length for base error message (excluding null terminator)
	int klen = snprintf(NULL, 0, "ErrorKind=%s\nBacktrace:\n", kind);
	if (klen < 0) {
		return NULL;
	}

	// Capture backtrace
	void *buffer[100];  // Limit to 100 frames
	int nptrs = backtrace(buffer, 100);
	char **symbols = backtrace_symbols(buffer, nptrs);
	if (symbols == NULL) {
		// Fallback if backtrace_symbols fails
		symbols = alloc(nptrs * sizeof(char *));
		if (symbols == NULL) {
			return NULL;
		}
		for (int i = 0; i < nptrs; i++) {
			symbols[i] = "Unknown";
		}
	}

	// Compute total length for backtrace
	int backtrace_len = 0;
	for (int i = 0; i < nptrs; i++) {
		backtrace_len += snprintf(NULL, 0, "%s\n",
					  symbols[i] ? symbols[i] : "Unknown");
	}

	// Allocate buffer (base message + backtrace + null terminator)
	int total_len = klen + backtrace_len;
	char *ret = alloc(total_len + 1);
	if (ret == NULL) {
		if (symbols != NULL && symbols != buffer) {
			free(symbols);	// Free if not stack-allocated
		}
		return NULL;
	}

	// Format base message
	int written =
	    snprintf(ret, klen + 1, "ErrorKind=%s\nBacktrace:\n", kind);
	if (written != klen) {
		release(ret);
		if (symbols != NULL && symbols != buffer) {
			free(symbols);
		}
		return NULL;
	}

	// Append backtrace
	int offset = klen;
	for (int i = 0; i < nptrs; i++) {
		int sym_len =
		    snprintf(ret + offset, total_len + 1 - offset, "%s\n",
			     symbols[i] ? symbols[i] : "Unknown");
		if (sym_len < 0) {
			release(ret);
			if (symbols != NULL && symbols != buffer) {
				free(symbols);
			}
			return NULL;
		}
		offset += sym_len;
	}

	ret[total_len] = 0;  // Ensure null termination

	if (symbols != NULL && symbols != buffer) {
		free(symbols);	// Free symbols allocated by backtrace_symbols
	}

	return ret;
}
/*
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void *alloc(unsigned long long size);
void release(void *);

char *format_err(const char *kind, int len) {
	if (kind == NULL || len < 0) {
		return NULL;
	}
	int actual_len = strnlen(kind, len);
	if (actual_len >= len && kind[len] != '\0') {
		return NULL;
	}

	// Compute required length (excluding null terminator)
	int klen = snprintf(NULL, 0, "ErrorKind=%s", kind);
	if (klen < 0) {
		return NULL;  // snprintf failed
	}

	// Allocate buffer (including null terminator)
	char *ret = alloc(klen + 1);
	if (ret == NULL) {
		return NULL;  // Allocation failed
	}

	// Format string
	int written = snprintf(ret, klen + 1, "ErrorKind=%s", kind);
	if (written != klen) {
		release(ret);
		return NULL;  // Formatting failed
	}

	ret[klen] = 0;

	return ret;
}
*/
