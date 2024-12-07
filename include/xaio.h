#ifndef XAIO_H
#define XAIO_H

#include <stdint.h>
#include <stdatomic.h>
// #include <sys/epoll.h>

struct xaiocp_s;
// struct xaio_groups_s;
// struct xaioscope_s {
//     struct xaiocp_s *const prv__cp;
//     struct xaioscope_s *prv__prev;
//     uint64_t prv__flags;
//     int64_t prv__deadline;
// };
// struct xaiotask_s {
//     struct xaioscope_s prv__task_scope;
// };
// struct xaio_s {
//     _Atomic(uintptr_t) prv__list_node_next;
//     uint32_t prv__flags_and_opcode;
//     int32_t prv__status;
// };

/**
 * Creates a new completion port bound to the current thread.
 * @param pport @c *pport receives a new completion port address or @c NULL on error.
 * @retval @c 0 on success
 * @retval @c -EINVAL when @c pport==NULL
 * @retval @c -ENOMEM when the system is out of memory
 */
int32_t xaiocp_new(struct xaiocp_s **pport);

// int32_t xaiocp_push_scope(struct xaiocp_s *iocp, struct xaioscope_s *scope, int32_t sequential, int32_t timeout_ms);


// int32_t xaiocp_set_timeout(struct xaiocp_s *porr, int32_t ms, void *token);

// int32_t xaiocp_set_timeout(struct xaiocp_s *iocp, int32_t ms, void *token);


#endif /* !defined(XAIO_H) */
