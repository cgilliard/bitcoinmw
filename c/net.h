#ifndef _NET_H__
#define _NET_H__

#define MULTIPLEX_REGISTER_TYPE_NONE 0
#define MULTIPLEX_REGISTER_TYPE_FLAG_READ 0x1
#define MULTIPLEX_REGISTER_TYPE_FLAG_WRITE (0x1 << 1)

#define ERROR_SOCKET -1
#define ERROR_CONNECT -2
#define ERROR_SETSOCKOPT -3
#define ERROR_BIND -4
#define ERROR_LISTEN -5
#define ERROR_ACCEPT -6
#define ERROR_FCNTL -7
#define ERROR_REGISTER -8
#define ERROR_MULTIPLEX_INIT -9
#define ERROR_GETSOCKNAME -10
#define ERROR_EAGAIN -11

typedef struct Socket Socket;
typedef struct Multiplex Multiplex;
typedef struct Event Event;

unsigned long long int socket_size();
int socket_connect(Socket *s, unsigned char addr[4], int port);
int socket_listen(Socket *s, unsigned char addr[4], int port, int backlog);
int socket_accept(Socket *s, Socket *accepted);
long long socket_recv(Socket *s, void *buf, unsigned long long capacity);
long long socket_send(Socket *s, const void *buf, unsigned long long len);
int socket_close(Socket *s);
int socket_shutdown(Socket *s);

unsigned long long multiplex_size();
unsigned long long event_size();
int multiplex_init(Multiplex *multiplex);
int multiplex_register(Multiplex *multiplex, Socket *s, int flags, void *ptr);
int multiplex_unregister_write(Multiplex *multiplex, Socket *s, void *ptr);
int multiplex_wait(Multiplex *multiplex, void *events, int max_events,
		   long long timeout_millis);
int multiplex_close(Multiplex *m);

void event_handle(Socket *s, Event *event);
_Bool event_is_read(Event *event);
_Bool event_is_write(Event *event);

#ifdef TEST
long long getfdcount();
#endif	// TEST

#endif	// _NET_H__
