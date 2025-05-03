#ifndef _CHANNEL_H__
#define _CHANNEL_H__

#include <pthread.h>

void _exit(int);
int perror(const char *msg);

typedef struct Message {
	struct Message *next;
	unsigned char buffer[];
} Message;

typedef struct Channel {
	pthread_mutex_t lock;
	pthread_cond_t cond;
	Message *head;
	Message *tail;
} Channel;

int channel_pending(Channel *handle);
int channel_init(Channel *handle);
int channel_send(Channel *handle, Message *msg);
Message *channel_recv(Channel *handle);
unsigned long long channel_handle_size();
int channel_destroy(Channel *handle);

#endif	// _CHANNEL_H__
