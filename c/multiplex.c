#include "net.h"
#include "nettypes.h"

#ifndef NULL
#define NULL ((void *)0)
#endif

unsigned long long multiplex_size() { return sizeof(Multiplex); }

unsigned long long event_size() {
#ifdef __APPLE__
	return sizeof(struct kevent);
#endif	// __APPLE__
#ifdef __linux__
	return sizeof(struct epoll_event);
#endif	// __linux__
}

int multiplex_init(Multiplex *multiplex) {
#ifdef __APPLE__
	multiplex->fd = kqueue();
#endif	// __APPLE__
#ifdef __linux__
	multiplex->fd = epoll_create1(0);
#endif	// __linux__
	if (multiplex->fd < 0) return ERROR_MULTIPLEX_INIT;

	return 0;
}

int multiplex_register(Multiplex *multiplex, Socket *s, int flags, void *ptr) {
#ifdef __APPLE__
	struct kevent change_event[2];

	int event_count = 0;

	if (flags & MULTIPLEX_REGISTER_TYPE_FLAG_READ) {
		EV_SET(&change_event[event_count], s->fd, EVFILT_READ,
		       EV_ADD | EV_ENABLE | EV_CLEAR, 0, 0, ptr);
		event_count++;
	}

	if (flags & MULTIPLEX_REGISTER_TYPE_FLAG_WRITE) {
		EV_SET(&change_event[event_count], s->fd, EVFILT_WRITE,
		       EV_ADD | EV_ENABLE | EV_CLEAR, 0, 0, ptr);
		event_count++;
	}

	if (kevent(multiplex->fd, change_event, event_count, NULL, 0, NULL) <
	    0) {
		return ERROR_REGISTER;
	}
	return 0;
#endif	// __APPLE__
#ifdef __linux__
	struct epoll_event ev;
	int event_flags = 0;

	if (flags & MULTIPLEX_REGISTER_TYPE_FLAG_READ) {
		event_flags |= EPOLLIN;
	}

	if (flags & MULTIPLEX_REGISTER_TYPE_FLAG_WRITE) {
		event_flags |= EPOLLOUT;
	}

	ev.events = event_flags;
	if (ptr == NULL)
		ev.data.fd = s->fd;
	else
		ev.data.ptr = ptr;

	if (epoll_ctl(multiplex->fd, EPOLL_CTL_ADD, s->fd, &ev) < 0) {
		if (errno == EEXIST) {
			if (epoll_ctl(multiplex->fd, EPOLL_CTL_MOD, s->fd,
				      &ev) < 0) {
				return ERROR_REGISTER;
			}
		} else
			return ERROR_REGISTER;
	}

	return 0;
#endif	// __linux__
}

int multiplex_unregister_write(Multiplex *multiplex, Socket *s, void *ptr) {
#ifdef __APPLE__
	struct kevent change_event[1];
	int event_count = 1;

	EV_SET(&change_event[0], s->fd, EVFILT_WRITE,
	       EV_DELETE | EV_ENABLE | EV_CLEAR, 0, 0, NULL);

	if (kevent(multiplex->fd, change_event, event_count, NULL, 0, NULL) <
	    0) {
		return ERROR_REGISTER;
	}
	return 0;
#endif	// __APPLE__
#ifdef __linux__
	struct epoll_event event;
	event.data.ptr = ptr;
	event.events = EPOLLIN;

	if (epoll_ctl(multiplex->fd, EPOLL_CTL_MOD, s->fd, &event) < 0)
		return ERROR_REGISTER;

	return 0;
#endif	// __linux__
}

int multiplex_wait(Multiplex *multiplex, void *events, int max_events,
		   long long timeout_millis) {
#ifdef __APPLE__
	struct timespec ts;
	struct timespec *timeout_ptr = NULL;

	if (timeout_millis >= 0) {
		ts.tv_sec = timeout_millis / 1000;
		ts.tv_nsec = (timeout_millis % 1000) * 1000000;
		timeout_ptr = &ts;
	}

	return kevent(multiplex->fd, NULL, 0, (struct kevent *)events,
		      max_events, timeout_ptr);
#endif	// __APPLE__
#ifdef __linux__
	int timeout = (timeout_millis >= 0) ? (int)timeout_millis : -1;

	return epoll_wait(multiplex->fd, (struct epoll_event *)events,
			  max_events, timeout);
#endif	// __linux__
}

int multiplex_close(Multiplex *m) {
	int ret = close(m->fd);
	return ret;
}

void event_handle(Socket *s, Event *event) {
#ifdef __APPLE__
	struct kevent *kv = (struct kevent *)event;
	s->fd = kv->ident;
#endif	// __APPLE__
#ifdef __linux__
	struct epoll_event *epoll_ev = (struct epoll_event *)event;
	s->fd = epoll_ev->data.fd;
#endif	// __linux__
}

_Bool event_is_read(Event *event) {
#ifdef __APPLE__
	struct kevent *kv = (struct kevent *)event;
	return kv->filter == EVFILT_READ;
#endif	// __APPLE__
#ifdef __linux__
	struct epoll_event *epoll_ev = (struct epoll_event *)event;
	return epoll_ev->events & EPOLLIN;
#endif	// __linux__
}

_Bool event_is_write(Event *event) {
#ifdef __APPLE__
	struct kevent *kv = (struct kevent *)event;
	return kv->filter == EVFILT_WRITE;
#endif	// __APPLE__
#ifdef __linux__
	struct epoll_event *epoll_ev = (struct epoll_event *)event;
	return epoll_ev->events & EPOLLOUT;
#endif	// __linux__
}

void *event_ptr(void *event) {
#ifdef __APPLE__
        struct kevent *kv = (struct kevent *)event;
        return kv->udata;
#elif defined(__linux__)
        struct epoll_event *epoll_ev = (struct epoll_event *)event;
        return epoll_ev->data.ptr;
#else
        return NULL;
#endif
}
