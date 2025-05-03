#include "channel.h"

int channel_pending(Channel *handle) { return handle->head != 0; }

int channel_init(Channel *handle) {
	if (pthread_mutex_init(&handle->lock, NULL)) return -1;
	if (pthread_cond_init(&handle->cond, NULL)) return -1;
	handle->head = handle->tail = NULL;
	return 0;
}
int channel_send(Channel *handle, Message *msg) {
	if (pthread_mutex_lock(&handle->lock)) {
		return -1;
	}

	msg->next = NULL;
	if (handle->tail)
		handle->tail->next = msg;
	else
		handle->head = msg;
	handle->tail = msg;

	if (pthread_cond_signal(&handle->cond)) {
		if (pthread_mutex_unlock(&handle->lock)) {
			perror("pthread_mutex_unlock");
			_exit(-1);
		}
		return -1;
	}

	if (pthread_mutex_unlock(&handle->lock)) {
		perror("pthread_mutex_unlock");
		_exit(-1);
	}

	return 0;
}
Message *channel_recv(Channel *handle) {
	if (pthread_mutex_lock(&handle->lock)) {
		return NULL;
	}

	while (!handle->head) {
		if (pthread_cond_wait(&handle->cond, &handle->lock)) {
			if (pthread_mutex_unlock(&handle->lock)) {
				perror("pthread_mutex_unlock");
				_exit(1);
			}
			return NULL;
		}
	}

	Message *ret = handle->head;
	handle->head = handle->head->next;
	if (!handle->head) handle->tail = NULL;

	if (pthread_mutex_unlock(&handle->lock)) {
		perror("pthread_mutex_unlock");
		_exit(1);
	}

	return ret;
}
unsigned long long channel_handle_size() { return sizeof(Channel); }
int channel_destroy(Channel *handle) {
	if (pthread_mutex_destroy(&handle->lock)) {
		perror("pthread_mutex_destroy");
		_exit(-1);
	}
	if (pthread_cond_destroy(&handle->cond)) {
		perror("pthread_cond_destroy");
		_exit(-1);
	}
	return 0;
}
