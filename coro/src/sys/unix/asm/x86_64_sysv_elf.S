
  .globl  __xaio_uctx_asm_swap
  .type __xaio_uctx_asm_swap, @function
  .align 16
__xaio_uctx_asm_swap:
		/* Prototype: __xaio_uctx_asm_swap(void **src, const void *dst) */
		/* Pushes the callee-saved registers to the caller stack */
		pushq %rbp
		pushq %rbx
		pushq %r12
		pushq %r13
		pushq %r14
		pushq %r15
		/* Saves the stack pointer to `*src` */
		movq %rsp, (%rdi)
		/* Set the stack pointer to `dst` */
		movq %rsi, %rsp
		/* Pops the callee-saved registers from the stack */
		popq %r15
		popq %r14
		popq %r13
		popq %r12
		popq %rbx
		popq %rbp
		/* Returns: We don't use `ret` here as the return stack predictor will be wrong */
		popq %rax
		jmpq *%rax
  .size __xaio_uctx_asm_swap, .-__xaio_uctx_asm_swap

  .globl  __xaio_uctx_asm_boot
  .type __xaio_uctx_asm_boot, @function
  .align 16
__xaio_uctx_asm_boot:
		popq %rax  /* pops task_start_cb from the stack */
		popq %rdi  /* pops task_start_arg into the rdi */
		jmpq *%rax /* call task_start_cb(rdi) */
  .size __xaio_uctx_asm_boot, .-__xaio_uctx_asm_boot
  .section	.note.GNU-stack,"",@progbits
