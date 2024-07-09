mod channel;
mod message;

pub use channel::ChannelId;
pub use id::Id;
pub use message::{Message, MessageId, PartialMessage};
pub use timestamp::Timestamp;

#[macro_use]
pub(crate) mod id {
    use super::timestamp;
    use essence::snowflake::SnowflakeReader;
    use std::fmt::{Debug, Display};
    use std::hash::Hash;
    use std::ops::Deref;

    /// Represents a snowflake, typically identifying an Adapt object.
    pub trait Id:
        Copy
        + Clone
        + Display
        + Debug
        + Deref<Target = u64>
        + PartialEq
        + PartialEq<u64>
        + Eq
        + PartialOrd
        + PartialOrd<u64>
        + Ord
        + Hash
        + From<u64>
        + Into<u64>
    {
        /// Creates a new ID from a [`u64`].
        #[must_use]
        fn new(id: u64) -> Self {
            Self::new_unchecked(id)
        }

        /// Creates a new ID from a [`u64`] without checking the model type.
        fn new_unchecked(id: u64) -> Self;

        /// Returns the ID as a [`u64`].
        fn get(&self) -> u64;

        /// Returns a [`SnowflakeReader`] over the ID.
        fn reader(&self) -> SnowflakeReader {
            SnowflakeReader::new(self.get())
        }

        /// Returns the creation timestamp of the ID.
        fn timestamp(&self) -> timestamp::Timestamp {
            timestamp::from_millis(self.reader().timestamp_millis())
        }
    }

    #[macro_export]
    macro_rules! id_type {
        (
            $(#[$meta:meta])*
            $vis:vis struct $name:ident $(: $model_type:ident)?;
        ) => {
            $(#[$meta])*
            #[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
            $vis struct $name(u64);

            impl $crate::models::Id for $name {
                fn new(id: u64) -> Self {
                    $(
                        #[cfg(debug_assertions)]
                        {
                            use $crate::essence;

                            let src = essence::snowflake::SnowflakeReader::new(id).model_type();
                            let expected = essence::models::ModelType::$model_type;
                            assert_eq!(
                                src,
                                expected,
                                "ID should have model type {expected:?}, but got {src:?}",
                            );
                        }
                    )?
                    Self::new_unchecked(id)
                }

                fn new_unchecked(id: u64) -> Self {
                    Self(id)
                }

                fn get(&self) -> u64 {
                    self.0 as u64
                }
            }
            impl From<u64> for $name {
                fn from(id: u64) -> Self {
                    Self(id)
                }
            }

            impl From<$name> for u64 {
                fn from(id: $name) -> u64 {
                    id.0
                }
            }

            impl PartialEq<u64> for $name {
                fn eq(&self, other: &u64) -> bool {
                    self.0 == *other
                }
            }

            impl PartialOrd<u64> for $name {
                fn partial_cmp(&self, other: &u64) -> Option<::std::cmp::Ordering> {
                    self.0.partial_cmp(other)
                }
            }

            impl ::std::fmt::Display for $name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    ::std::fmt::Display::fmt(&self.0, f)
                }
            }

            impl ::std::ops::Deref for $name {
                type Target = u64;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        };
    }
}

/// Utility types and functions for working with timestamps.
pub mod timestamp {
    /// The timestamp type, which is different depending on the features enabled.
    ///
    /// This is [`chrono::DateTime`] if the `chrono` feature is enabled, otherwise it is
    /// [`std::time::SystemTime`].
    #[cfg(feature = "chrono")]
    pub type Timestamp = chrono::DateTime<chrono::Utc>;
    /// The timestamp type, which is different depending on the features enabled.
    ///
    /// This is [`chrono::DateTime`] if the `chrono` feature is enabled, otherwise it is
    /// [`std::time::SystemTime`].
    #[cfg(not(feature = "chrono"))]
    pub type Timestamp = std::time::SystemTime;

    /// Creates a new timestamp from a number of milliseconds since the Unix epoch.
    #[must_use]
    pub fn from_millis(millis: u64) -> Timestamp {
        #[cfg(feature = "chrono")]
        #[allow(clippy::cast_possible_wrap)]
        {
            chrono::TimeZone::timestamp_millis_opt(&chrono::Utc, millis as i64).unwrap()
        }
        #[cfg(not(feature = "chrono"))]
        {
            std::time::UNIX_EPOCH + std::time::Duration::from_millis(millis)
        }
    }

    /// The error type for parsing an ISO8601 timestamp.
    ///
    /// This is [`chrono::ParseError`] if the `chrono` feature is enabled, otherwise it is
    /// [`std::time::SystemTimeError`].
    #[cfg(feature = "chrono")]
    type Error = chrono::ParseError;
    /// The error type for parsing an ISO8601 timestamp.
    ///
    /// This is [`chrono::ParseError`] if the `chrono` feature is enabled, otherwise it is
    /// [`std::time::SystemTimeError`].
    #[cfg(not(feature = "chrono"))]
    type Error = std::time::SystemTimeError;

    /// Parses an ISO8601 timestamp into a [`Timestamp`].
    pub fn from_iso(iso: &str) -> Result<Timestamp, Error> {
        #[cfg(feature = "chrono")]
        {
            Ok(chrono::DateTime::parse_from_rfc3339(iso)?.with_timezone(&chrono::Utc))
        }
        #[cfg(not(feature = "chrono"))]
        {
            let _ = iso;
            unimplemented!("ISO8601 parsing is not supported without the `chrono` feature");
        }
    }
}

#[macro_export]
macro_rules! impl_common_traits {
    ($t:ty) => {
        impl PartialEq for $t {
            fn eq(&self, other: &Self) -> bool {
                self.id == other.id
            }
        }

        impl Eq for $t {}

        impl ::std::hash::Hash for $t {
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.id.hash(state)
            }
        }
    };
}
