use core::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

/// Reason a bounded computation stopped without producing an exact result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IncompleteReason {
    /// The deterministic logical-work limit could not admit the next operation.
    WorkLimitExceeded {
        /// Configured work limit.
        limit: u64,
        /// Logical work fully consumed before stopping.
        consumed: u64,
    },
    /// A caller-owned atomic cancellation flag was observed.
    Cancelled {
        /// Logical work fully consumed before cancellation was observed.
        consumed: u64,
    },
}

impl fmt::Display for IncompleteReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkLimitExceeded { limit, consumed } => write!(
                formatter,
                "logical work limit {limit} cannot admit more work after {consumed} steps"
            ),
            Self::Cancelled { consumed } => {
                write!(formatter, "computation cancelled after {consumed} steps")
            }
        }
    }
}

impl std::error::Error for IncompleteReason {}

/// Deterministic work and cancellation controls for one graph computation.
///
/// Logical work is counted by algorithm-defined operations rather than wall
/// time. A batch is admitted atomically: if its full logical cost does not fit,
/// the operation returns incomplete before executing that batch. The optional
/// atomic flag carries cancellation only; its relaxed load does not synchronize
/// caller data.
#[derive(Debug, Clone, Copy)]
pub struct ExecutionControl<'a> {
    work_limit: u64,
    cancellation: Option<&'a AtomicBool>,
}

impl ExecutionControl<'static> {
    /// Returns a control with the largest logical-work budget and no flag.
    #[must_use]
    pub const fn unlimited() -> Self {
        Self {
            work_limit: u64::MAX,
            cancellation: None,
        }
    }
}

impl<'a> ExecutionControl<'a> {
    /// Creates a control with an exact logical-work limit.
    #[must_use]
    pub const fn with_work_limit(work_limit: u64) -> Self {
        Self {
            work_limit,
            cancellation: None,
        }
    }

    /// Adds a caller-owned cancellation flag.
    #[must_use]
    pub const fn with_cancellation(mut self, cancellation: &'a AtomicBool) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Returns the configured logical-work limit.
    #[must_use]
    pub const fn work_limit(self) -> u64 {
        self.work_limit
    }

    pub(crate) const fn meter(self) -> WorkMeter<'a> {
        WorkMeter {
            control: self,
            consumed: 0,
        }
    }
}

pub(crate) trait WorkControl {
    fn consume(&mut self, amount: u64) -> Result<(), IncompleteReason>;

    fn consumed(&self) -> Option<u64>;

    fn step(&mut self) -> Result<(), IncompleteReason> {
        self.consume(1)
    }
}

pub(crate) struct Unbounded;

impl WorkControl for Unbounded {
    #[inline]
    fn consume(&mut self, _amount: u64) -> Result<(), IncompleteReason> {
        Ok(())
    }

    #[inline]
    fn consumed(&self) -> Option<u64> {
        None
    }
}

pub(crate) struct WorkMeter<'a> {
    control: ExecutionControl<'a>,
    consumed: u64,
}

impl WorkControl for WorkMeter<'_> {
    #[inline]
    fn consume(&mut self, amount: u64) -> Result<(), IncompleteReason> {
        if self
            .control
            .cancellation
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            return Err(IncompleteReason::Cancelled {
                consumed: self.consumed,
            });
        }
        let Some(next) = self.consumed.checked_add(amount) else {
            return Err(IncompleteReason::WorkLimitExceeded {
                limit: self.control.work_limit,
                consumed: self.consumed,
            });
        };
        if next > self.control.work_limit {
            return Err(IncompleteReason::WorkLimitExceeded {
                limit: self.control.work_limit,
                consumed: self.consumed,
            });
        }
        self.consumed = next;
        Ok(())
    }

    #[inline]
    fn consumed(&self) -> Option<u64> {
        Some(self.consumed)
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn bounded_work_admission_is_fail_atomic_and_overflow_safe() {
        let limit = kani::any::<u64>();
        let initial = kani::any::<u64>();
        let amount = kani::any::<u64>();
        kani::assume(initial <= limit);

        let mut meter = WorkMeter {
            control: ExecutionControl::with_work_limit(limit),
            consumed: initial,
        };
        let expected = initial.checked_add(amount);
        let result = meter.consume(amount);

        match expected {
            Some(next) if next <= limit => {
                assert_eq!(result, Ok(()));
                assert_eq!(meter.consumed, next);
            }
            _ => {
                assert_eq!(
                    result,
                    Err(IncompleteReason::WorkLimitExceeded {
                        limit,
                        consumed: initial,
                    })
                );
                assert_eq!(meter.consumed, initial);
            }
        }
    }
}
