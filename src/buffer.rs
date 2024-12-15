struct Buffer {
    group_id: u16,
    buffer_id: u16,
    len: u32,
    base: *mut u8,
}

struct FreeBuffer {
    next: *mut FreeBuffer,
}

struct BufferGroup {
    group_id: u32,
    buffer_count: u32,
    buffer_size: usize,
    memory: *mut libc::c_char,
    free_buffers: *mut FreeBuffer,
    free_buffers_count: usize,
}
