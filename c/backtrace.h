#ifndef _BACKTRACE_H__
#define _BACKTRACE_H__

#define MAX_ENTRIES 128
#define MAX_LINE_LEN 2048

char *gen_backtrace(void **bt_entries, int size);

#endif	// _BACKTRACE_H__

