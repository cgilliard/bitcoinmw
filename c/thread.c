#include <pthread.h>

typedef struct ThreadHandle {
	pthread_t handle;
} ThreadHandle;

int printf(const char *, ...);
int thread_create(void *(*start_routine)(void *), void *arg) {
	pthread_t th;
	pthread_attr_t attr;
	pthread_attr_init(&attr);
	pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
	int ret = pthread_create(&th, &attr, start_routine, arg);
	pthread_attr_destroy(&attr);
	return ret;
}

unsigned long long thread_handle_size() { return sizeof(ThreadHandle); }

int thread_create_joinable(ThreadHandle *handle, void *(*start_routine)(void *),
			   void *arg) {
	return pthread_create(&handle->handle, NULL, start_routine, arg);
}

int thread_join(ThreadHandle *handle) {
	return pthread_join(handle->handle, NULL);
}
int thread_detach(ThreadHandle *handle) {
	return pthread_detach(handle->handle);
}
