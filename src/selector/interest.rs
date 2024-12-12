use bitflags::bitflags;

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Interest: u32 {
        /// Interest interests or event.
        const READABLE = 0x001u32;
        /// Writable interests or event.
        const WRITABLE = 0x004u32;
        /// Priority interests or event.
        const PRIORITY = 0x002u32;

        /// Error event.
        const ERROR = 0x008u32;
        /// Hang-up event (peer closed its end of the channel).
        const HANG_UP = 0x010u32;
        /// Stream socket peer closed connection, or shut down writing half of connection.
        const RDHANG_UP = 0x2000u32;
    }
}
