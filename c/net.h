#ifndef _NET_H__
#define _NET_H__

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

unsigned long long int socket_size();
int socket_connect(Socket* s, unsigned char addr[4], int port);
int socket_listen(Socket* s, unsigned char addr[4], int port, int backlog);
int socket_accept(Socket* s, Socket* accepted);
long long socket_recv(Socket* s, void* buf, unsigned long long capacity);
long long socket_send(Socket* s, const void* buf, unsigned long long len);

#endif	// _NET_H__
