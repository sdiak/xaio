use bitflags::bitflags;

const READABLE: u16 = 0x001u16;
const WRITABLE: u16 = 0x004u16;
const PRIORITY: u16 = 0x002u16;
const ERROR: u16 = 0x008u16;
const HANG_UP: u16 = 0x010u16;
const RDHANG_UP: u16 = 0x2000u16;
const ONESHOT: u16 = 0x8000u16;

bitflags! {
    /// Represents a set of poll flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PollFlag: u16 {
        /// Interest interests or event.
        const READABLE = READABLE;
        /// Writable interests or event.
        const WRITABLE = WRITABLE;
        /// Priority interests or event.
        const PRIORITY = PRIORITY;

        /// Error event.
        const ERROR = ERROR;
        /// Hang-up event (peer closed its end of the channel).
        const HANG_UP = HANG_UP;
        /// Stream socket peer closed connection, or shut down writing half of connection.
        const RDHANG_UP = RDHANG_UP;

        // Removes the registration after the first event
        const ONESHOT = ONESHOT;

        const INTEREST_MASK = READABLE | WRITABLE | PRIORITY | ONESHOT;
    }
}
