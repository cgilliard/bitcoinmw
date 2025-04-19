#include <stdio.h>
#include <ctype.h>
#include <execinfo.h>
#include <stdlib.h>
#include <string.h>

// Custom allocator
void *alloc(unsigned long long size);
void release(void *);

// Robust Rust symbol demangler
static char *demangle_rust_symbol(const char *input) {
	if (input == NULL) {
		char *ret = alloc(8);
		if (ret) strcpy(ret, "Unknown");
		return ret;
	}

	// Extract mangled name from macOS/Linux backtrace_symbols output
	const char *mangled = input;
	// Skip prefix (e.g., "test_bmw 0x... " or "./bin/test_bmw(+0x...)")
	while (*mangled &&
	       !(*mangled == '_' && mangled[1] == 'Z' && mangled[2] == 'N')) {
		mangled++;
	}
	if (!(*mangled)) {
		// Not a Rust symbol, return copy
		size_t len = strlen(input) + 1;
		char *ret = alloc(len);
		if (ret) strcpy(ret, input);
		return ret;
	}

	// Allocate buffer (heuristic: 4x mangled length)
	size_t mangled_len = strlen(mangled);
	char *demangled = alloc(mangled_len * 4 + 1);
	if (!demangled) {
		char *ret = alloc(mangled_len + 1);
		if (ret) strcpy(ret, mangled);
		return ret;
	}

	size_t pos = 0;		 // Position in demangled
	size_t mangled_pos = 3;	 // Skip _ZN

	// Parse Rust mangled name
	while (mangled_pos < mangled_len) {
		// Handle special tokens
		if (mangled[mangled_pos] == '$') {
			if (strncmp(&mangled[mangled_pos], "$LT$", 4) == 0) {
				demangled[pos++] = '<';
				mangled_pos += 4;
			} else if (strncmp(&mangled[mangled_pos], "$GT$", 4) ==
				   0) {
				demangled[pos++] = '>';
				mangled_pos += 4;
			} else if (strncmp(&mangled[mangled_pos], "$u20$", 5) ==
				   0) {
				demangled[pos++] = ' ';
				mangled_pos += 5;
			} else if (strncmp(&mangled[mangled_pos], "$BP$", 4) ==
				   0) {
				demangled[pos++] = '*';
				mangled_pos += 4;
			} else if (strncmp(&mangled[mangled_pos], "$RF$", 4) ==
				   0) {
				demangled[pos++] = '&';
				mangled_pos += 4;
			} else {
				demangled[pos++] = mangled[mangled_pos++];
			}
			continue;
		}

		// Read length prefix
		if (mangled[mangled_pos] < '0' || mangled[mangled_pos] > '9') {
			break;	// End or invalid
		}
		int seg_len = 0;
		while (mangled_pos < mangled_len &&
		       mangled[mangled_pos] >= '0' &&
		       mangled[mangled_pos] <= '9') {
			seg_len = seg_len * 10 + (mangled[mangled_pos] - '0');
			mangled_pos++;
		}
		if (mangled_pos + seg_len >= mangled_len) {
			break;	// Malformed
		}

		// Add separator
		if (pos > 0) {
			demangled[pos++] = ':';
			demangled[pos++] = ':';
		}

		// Copy segment
		for (int i = 0; i < seg_len && mangled_pos < mangled_len; i++) {
			demangled[pos++] = mangled[mangled_pos++];
		}
	}

	// Skip trailing hash (e.g., 17h9a14b977883bdcf3E)
	if (mangled_pos < mangled_len && mangled[mangled_pos] >= '0' &&
	    mangled[mangled_pos] <= '9') {
		int hash_len = 0;
		while (mangled_pos < mangled_len &&
		       isalnum(mangled[mangled_pos])) {
			hash_len++;
			mangled_pos++;
		}
	}

	demangled[pos] = '\0';

	// Shrink buffer
	char *final = alloc(pos + 1);
	if (final) {
		strcpy(final, demangled);
		release(demangled);
		return final;
	}
	return demangled;
}

// Format error with enhanced backtrace
char *format_err(const char *kind, int len) {
	if (kind == NULL || len < 0) {
		return NULL;
	}
	int actual_len = strnlen(kind, len);
	if (actual_len >= len && kind[len] != '\0') {
		return NULL;
	}

	int klen = snprintf(NULL, 0, "ErrorKind=%s\nBacktrace:\n", kind);
	if (klen < 0) {
		return NULL;
	}

	void *buffer[100];
	int nptrs = backtrace(buffer, 100);
	char **symbols = backtrace_symbols(buffer, nptrs);
	if (symbols == NULL) {
		symbols = alloc(nptrs * sizeof(char *));
		if (symbols == NULL) {
			return NULL;
		}
		for (int i = 0; i < nptrs; i++) {
			symbols[i] = "Unknown";
		}
	}

	int backtrace_len = 0;
	char **demangled_symbols = alloc(nptrs * sizeof(char *));
	if (!demangled_symbols) {
		if (symbols != NULL && symbols != buffer) {
			free(symbols);
		}
		return NULL;
	}
	for (int i = 0; i < nptrs; i++) {
		demangled_symbols[i] =
		    demangle_rust_symbol(symbols[i] ? symbols[i] : "Unknown");
		if (!demangled_symbols[i]) {
			demangled_symbols[i] =
			    strdup(symbols[i] ? symbols[i] : "Unknown");
		}
		backtrace_len += snprintf(NULL, 0, "#%d %p %s\n", i, buffer[i],
					  demangled_symbols[i]);
	}

	int total_len = klen + backtrace_len;
	char *ret = alloc(total_len + 1);
	if (ret == NULL) {
		for (int i = 0; i < nptrs; i++) {
			if (demangled_symbols[i]) {
				release(demangled_symbols[i]);
			}
		}
		release(demangled_symbols);
		if (symbols != NULL && symbols != buffer) {
			free(symbols);
		}
		return NULL;
	}

	int written =
	    snprintf(ret, klen + 1, "ErrorKind=%s\nBacktrace:\n", kind);
	if (written != klen) {
		release(ret);
		for (int i = 0; i < nptrs; i++) {
			if (demangled_symbols[i]) {
				release(demangled_symbols[i]);
			}
		}
		release(demangled_symbols);
		if (symbols != NULL && symbols != buffer) {
			free(symbols);
		}
		return NULL;
	}

	int offset = klen;
	for (int i = 0; i < nptrs; i++) {
		int sym_len =
		    snprintf(ret + offset, total_len + 1 - offset,
			     "#%d %p %s\n", i, buffer[i], demangled_symbols[i]);
		if (sym_len < 0) {
			release(ret);
			for (int i = 0; i < nptrs; i++) {
				if (demangled_symbols[i]) {
					release(demangled_symbols[i]);
				}
			}
			release(demangled_symbols);
			if (symbols != NULL && symbols != buffer) {
				free(symbols);
			}
			return NULL;
		}
		offset += sym_len;
	}

	ret[total_len] = 0;

	for (int i = 0; i < nptrs; i++) {
		if (demangled_symbols[i]) {
			release(demangled_symbols[i]);
		}
	}
	release(demangled_symbols);
	if (symbols != NULL && symbols != buffer) {
		free(symbols);
	}

	return ret;
}

/*
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
*/
