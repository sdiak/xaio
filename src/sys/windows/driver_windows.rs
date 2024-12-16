use super::IoCompletionPort;

#[derive(Debug)]
pub(crate) struct Driver {
    iocp: IoCompletionPort,
    config: crate::capi::xconfig_s,
}
