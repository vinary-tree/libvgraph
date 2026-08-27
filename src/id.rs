use core::fmt;

/// Dense zero-based vertex identifier within one [`crate::CsrGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
pub struct DenseId(u32);

impl DenseId {
    /// Creates an identifier from its raw representation.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw zero-based identifier.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for DenseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Dense zero-based identifier within one SCC decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
pub struct ComponentId(u32);

impl ComponentId {
    /// Creates an identifier from its raw representation.
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw zero-based identifier.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

pub(crate) trait IndexId: Copy + Ord {
    fn from_raw(raw: u32) -> Self;
    fn get(self) -> u32;
}

impl IndexId for DenseId {
    fn from_raw(raw: u32) -> Self {
        Self::from_raw(raw)
    }

    fn get(self) -> u32 {
        self.get()
    }
}

impl IndexId for ComponentId {
    fn from_raw(raw: u32) -> Self {
        Self::from_raw(raw)
    }

    fn get(self) -> u32 {
        self.get()
    }
}
