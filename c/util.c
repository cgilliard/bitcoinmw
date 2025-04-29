#include "util.h"

unsigned long long cstring_len(const char *X) {
        const char *Y = X;
        while (*X) X++;
        return X - Y;
}

