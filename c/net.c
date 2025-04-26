#include "net.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "nettypes.h"

#ifdef TEST
long long __fd_count = 0;
long long getfdcount() { return __fd_count; }
#endif	// TEST

int close_impl(int fd) {
	int ret = close(fd);
#ifdef TEST
	if (ret == 0) __atomic_fetch_sub(&__fd_count, 1, __ATOMIC_SEQ_CST);
#endif	// TEST
	return ret;
}

unsigned long long int socket_size() { return sizeof(Socket); }

int socket_connect(Socket* s, unsigned char addr[4], int port) {
	s->fd = socket(AF_INET, SOCK_STREAM, 0);
	if (s->fd < 0) return ERROR_SOCKET;
#ifdef TEST
	__atomic_fetch_add(&__fd_count, 1, __ATOMIC_SEQ_CST);
#endif	// TEST

	struct sockaddr_in serv_addr;
	memset(&serv_addr, 0, sizeof(serv_addr));
	serv_addr.sin_family = AF_INET;
	serv_addr.sin_port = htons(port);
	memcpy(&serv_addr.sin_addr.s_addr, addr, 4);

	if (connect(s->fd, (struct sockaddr*)&serv_addr, sizeof(serv_addr)) <
	    0) {
		close_impl(s->fd);
		return ERROR_CONNECT;
	}

	int flags = fcntl(s->fd, F_GETFL, 0);
	if (flags < 0) {
		close_impl(s->fd);
		return ERROR_FCNTL;
	}

	if (fcntl(s->fd, F_SETFL, flags | O_NONBLOCK) < 0) {
		close_impl(s->fd);
		return ERROR_FCNTL;
	}
	return 0;
}

int socket_listen(Socket* s, unsigned char addr[4], int port, int backlog) {
	int opt = 1;
	struct sockaddr_in address;

	s->fd = socket(AF_INET, SOCK_STREAM, 0);
	if (s->fd < 0) return ERROR_SOCKET;
#ifdef TEST
	__atomic_fetch_add(&__fd_count, 1, __ATOMIC_SEQ_CST);
#endif	// TEST
	if (setsockopt(s->fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt))) {
		close_impl(s->fd);
		return ERROR_SETSOCKOPT;
	}

	if (setsockopt(s->fd, SOL_SOCKET, SO_REUSEPORT, &opt, sizeof(opt))) {
		close_impl(s->fd);
		return ERROR_SETSOCKOPT;
	}
	int flags = fcntl(s->fd, F_GETFL, 0);
	if (flags < 0) {
		close_impl(s->fd);
		return ERROR_FCNTL;
	}

	if (fcntl(s->fd, F_SETFL, flags | O_NONBLOCK) < 0) {
		close_impl(s->fd);
		return ERROR_FCNTL;
	}

	address.sin_family = AF_INET;
	address.sin_addr.s_addr = INADDR_ANY;
	address.sin_port = htons(port);

	if (bind(s->fd, (struct sockaddr*)&address, sizeof(address)) < 0) {
		close_impl(s->fd);
		return ERROR_BIND;
	}

	if (listen(s->fd, backlog) < 0) {
		close_impl(s->fd);
		return ERROR_LISTEN;
	}

	socklen_t addr_len = sizeof(address);
	if (getsockname(s->fd, (struct sockaddr*)&address, &addr_len) < 0) {
		close_impl(s->fd);
		return ERROR_GETSOCKNAME;
	}
	port = ntohs(address.sin_port);
	return port;
}

int socket_accept(Socket* s, Socket* accepted) {
	struct sockaddr_in client_addr;
	socklen_t client_len = sizeof(client_addr);
	accepted->fd =
	    accept(s->fd, (struct sockaddr*)&client_addr, &client_len);
	if (accepted->fd < 0) {
		if (errno == EAGAIN) {
			return ERROR_EAGAIN;
		}
		return ERROR_ACCEPT;
	}

#ifdef TEST
	__atomic_fetch_add(&__fd_count, 1, __ATOMIC_SEQ_CST);
#endif	// TEST

	int flags = fcntl(accepted->fd, F_GETFL, 0);

	if (fcntl(accepted->fd, F_SETFL, flags | O_NONBLOCK) < 0) {
		close_impl(accepted->fd);
		return ERROR_FCNTL;
	}

	return 0;
}

long long socket_recv(Socket* s, void* buf, unsigned long long capacity) {
	int ret = read(s->fd, buf, capacity);
	if (ret < 0) {
		if (errno == EAGAIN) {
			return ERROR_EAGAIN;
		}
		return ERROR_SOCKET;
	}
	return ret;
}

long long socket_send(Socket* s, const void* buf, unsigned long long len) {
	long long ret = write(s->fd, buf, len);
	if (ret < 0) {
		if (errno == EAGAIN) {
			return ERROR_EAGAIN;
		}
		return ERROR_SOCKET;
	}
	return ret;
}

int socket_shutdown(Socket* s) { return shutdown(s->fd, SHUT_RDWR); }
int socket_close(Socket* s) { return close_impl(s->fd); }
