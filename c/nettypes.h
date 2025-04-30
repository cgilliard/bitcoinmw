#ifndef _NETTYPES_H__
#define _NETTYPES_H__

#ifdef __APPLE__
#include <sys/event.h>
#elif defined(__linux__)
#include <sys/epoll.h>
#endif

struct Socket {
	int fd;
};

struct Multiplex {
	int fd;
};

struct Event {
#ifdef __APPLE__
	struct kevent event;
#elif defined(__linux__)
	struct epoll_event event;
#else
#error \
    "Unsupported platform: Event requires macOS (__APPLE__) or Linux (__linux__)"
#endif
};

#endif	// _NETTYPES_H__
