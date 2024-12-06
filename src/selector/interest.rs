
use std::num::NonZeroU8;
use std::{fmt, ops};


#[derive(Copy, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct Interest(NonZeroU8);

const READABLE: u8 = 0b0001u8;
const WRITABLE: u8 = 0b0010u8;
const PRIORITY: u8 = 0b0100u8;

impl Interest {
    /// Returns a `Interest` set representing readable interests.
    pub const READABLE: Interest = Interest(unsafe { NonZeroU8::new_unchecked(READABLE) });

    /// Returns a `Interest` set representing writable interests.
    pub const WRITABLE: Interest = Interest(unsafe { NonZeroU8::new_unchecked(WRITABLE) });

    /// Returns a `Interest` set representing priority completion interests.
    pub const PRIORITY: Interest = Interest(unsafe { NonZeroU8::new_unchecked(PRIORITY) });

    /// Add together two `Interest`.
    ///
    /// This does the same thing as the `BitOr` implementation, but is a
    /// constant function.
    ///
    /// ```
    /// use mio::Interest;
    ///
    /// const INTERESTS: Interest = Interest::READABLE.add(Interest::WRITABLE);
    /// # fn silent_dead_code_warning(_: Interest) { }
    /// # silent_dead_code_warning(INTERESTS)
    /// ```
    #[allow(clippy::should_implement_trait)]
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub const fn add(self, other: Interest) -> Interest {
        Interest(unsafe { NonZeroU8::new_unchecked(self.0.get() | other.0.get()) })
    }

    /// Removes `other` `Interest` from `self`.
    ///
    /// Returns `None` if the set would be empty after removing `other`.
    ///
    /// ```
    /// use mio::Interest;
    ///
    /// const RW_INTERESTS: Interest = Interest::READABLE.add(Interest::WRITABLE);
    ///
    /// // As long a one interest remain this will return `Some`.
    /// let w_interest = RW_INTERESTS.remove(Interest::READABLE).unwrap();
    /// assert!(!w_interest.is_readable());
    /// assert!(w_interest.is_writable());
    ///
    /// // Removing all interests from the set will return `None`.
    /// assert_eq!(w_interest.remove(Interest::WRITABLE), None);
    ///
    /// // Its also possible to remove multiple interests at once.
    /// assert_eq!(RW_INTERESTS.remove(RW_INTERESTS), None);
    /// ```
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn remove(self, other: Interest) -> Option<Interest> {
        NonZeroU8::new(self.0.get() & !other.0.get()).map(Interest)
    }
    
    /// Returns true if the value includes readable readiness.
    #[must_use]
    pub const fn is_readable(self) -> bool {
        (self.0.get() & READABLE) != 0
    }

    /// Returns true if the value includes writable readiness.
    #[must_use]
    pub const fn is_writable(self) -> bool {
        (self.0.get() & WRITABLE) != 0
    }

    /// Returns true if `Interest` contains priority readiness.
    #[must_use]
    pub const fn is_priority(self) -> bool {
        (self.0.get() & PRIORITY) != 0
    }
}

impl ops::BitOr for Interest {
    type Output = Self;

    #[inline]
    fn bitor(self, other: Self) -> Self {
        self.add(other)
    }
}

impl ops::BitOrAssign for Interest {
    #[inline]
    fn bitor_assign(&mut self, other: Self) {
        self.0 = (*self | other).0;
    }
}

impl fmt::Debug for Interest {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut one = false;
        if self.is_readable() {
            if one {
                write!(fmt, " | ")?
            }
            write!(fmt, "READABLE")?;
            one = true
        }
        if self.is_writable() {
            if one {
                write!(fmt, " | ")?
            }
            write!(fmt, "WRITABLE")?;
            one = true
        }
        if self.is_priority() {
            if one {
                write!(fmt, " | ")?
            }
            write!(fmt, "PRIORITY")?;
            one = true
        }
        debug_assert!(one, "printing empty interests");
        Ok(())
    }
}
