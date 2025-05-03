#ifndef _B64_H__
#define _B64_H__

int Base64decode(char *bufplain, const char *bufcoded);
int Base64encode(char *encoded, const char *string, int len);

#endif	// _B64_H__
