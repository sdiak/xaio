use super::iocp::IoCompletionPort;

#[derive(Debug)]
pub(crate) struct Driver {
    iocp: IoCompletionPort,
    config: crate::capi::xconfig_s,
}
