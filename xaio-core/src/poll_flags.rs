use bitflags::bitflags;

bitflags! {
    /// Represents a set of poll flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PollFlag: u16 {
        /// Interest interests or event.
        const READABLE = 0x001u16;
        /// Writable interests or event.
        const WRITABLE = 0x004u16;
        /// Priority interests or event.
        const PRIORITY = 0x002u16;

        /// Error event.
        const ERROR = 0x008u16;
        /// Hang-up event (peer closed its end of the channel).
        const HANG_UP = 0x010u16;
        /// Stream socket peer closed connection, or shut down writing half of connection.
        const RDHANG_UP = 0x2000u16;

        // Removes the registration after the first event
        const ONESHOT = 0x8000u16;
    }
}
