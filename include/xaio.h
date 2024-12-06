#ifndef XAIO_H
#define XAIO_H

#include <stdint.h>
// #include <sys/epoll.h>

struct xaiocp_s;

int32_t xaiocp_new(struct xaiocp_s **result);

#endif /* !defined(XAIO_H) */
