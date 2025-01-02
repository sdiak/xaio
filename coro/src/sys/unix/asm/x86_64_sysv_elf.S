
  .globl  __xaio_uctx_asm_swap
  .type __xaio_uctx_asm_swap, @function
  .align 16
__xaio_uctx_asm_swap:
		/* Prototype: __xaio_uctx_asm_swap(void **src, const void *dst) */
		/* Pushes the callee-saved registers to the caller stack */
		push rbp
		push rbx
		push r12
		push r13
		push r14
		push r15
		/* Saves the stack pointer to `*src` */
		mov [rdi], rsp
		/* Set the stack pointer to `dst` */
		mov rsp, rsi
		/* Pops the callee-saved registers from the stack */
		pop r15
		pop r14
		pop r13
		pop r12
		pop rbx
		pop rbp
		/* We do not use `ret` here as the return stack predictor will be wrong */
		pop rax
		jmp rax
  .size __xaio_uctx_asm_swap, .-__xaio_uctx_asm_swap

  .globl  __xaio_uctx_asm_boot
  .type __xaio_uctx_asm_boot, @function
  .align 16
__xaio_uctx_asm_boot:
		pop rax  /* pops task_start_cb from the stack */
		pop rdi  /* pops task_start_arg into the rdi */
		jmp rax  /* call task_start_cb(rdi) */
  .size __xaio_uctx_asm_boot, .-__xaio_uctx_asm_boot

  .globl  __xaio_uctx_asm_prefetch
  .type __xaio_uctx_asm_prefetch, @function
  .align 16
__xaio_uctx_asm_prefetch:
		prefetcht2 [rdi]
		ret
  .size __xaio_uctx_asm_prefetch, .-__xaio_uctx_asm_prefetch


  .globl  __xaio_uctx_asm_get_sp
  .type __xaio_uctx_asm_get_sp, @function
  .align 16
__xaio_uctx_asm_get_sp:
	mov rax, rsp
	ret
  .size __xaio_uctx_asm_get_sp, .-__xaio_uctx_asm_get_sp

  .section	.note.GNU-stack,"",@progbits
