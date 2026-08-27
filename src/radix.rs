use crate::control::WorkControl;
use crate::IncompleteReason;

const RADIX_BITS: u32 = 11;
pub(crate) const RADIX_BUCKET_COUNT: usize = 1 << RADIX_BITS;
const RADIX_MASK: u64 = (RADIX_BUCKET_COUNT as u64) - 1;
const RADIX_PASSES: u32 = 64_u32.div_ceil(RADIX_BITS);

pub(crate) fn encode_pair(source: u32, target: u32) -> u64 {
    (u64::from(source) << 32) | u64::from(target)
}

pub(crate) fn pair_source(key: u64) -> u32 {
    let bytes = key.to_be_bytes();
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(crate) fn pair_target(key: u64) -> u32 {
    let bytes = key.to_be_bytes();
    u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
}

pub(crate) const fn logical_work(candidate_count: u64) -> u64 {
    if candidate_count < 2 {
        0
    } else {
        14 * candidate_count + 13 * (RADIX_BUCKET_COUNT as u64)
    }
}

#[derive(Debug, Default)]
pub(crate) struct RadixWorkspace {
    scratch: Vec<u64>,
    counts: Vec<usize>,
}

impl RadixWorkspace {
    pub(crate) const fn new() -> Self {
        Self {
            scratch: Vec::new(),
            counts: Vec::new(),
        }
    }

    pub(crate) fn sort_dedup<C: WorkControl>(
        &mut self,
        values: &mut Vec<u64>,
        control: &mut C,
    ) -> Result<(), IncompleteReason> {
        if values.len() < 2 {
            return Ok(());
        }

        let value_count = values.len() as u64;
        let bucket_count = RADIX_BUCKET_COUNT as u64;
        control.consume(value_count + bucket_count)?;
        self.scratch.resize(values.len(), 0);
        self.counts.resize(RADIX_BUCKET_COUNT, 0);

        for pass in 0..RADIX_PASSES {
            control.consume(bucket_count)?;
            self.counts.fill(0);
            let shift = pass * RADIX_BITS;

            control.consume(value_count)?;
            if pass % 2 == 0 {
                for &key in values.iter() {
                    let bucket = ((key >> shift) & RADIX_MASK) as usize;
                    self.counts[bucket] += 1;
                }
            } else {
                for &key in &self.scratch {
                    let bucket = ((key >> shift) & RADIX_MASK) as usize;
                    self.counts[bucket] += 1;
                }
            }

            control.consume(bucket_count)?;
            let mut prefix = 0usize;
            for count in &mut self.counts {
                let current = *count;
                *count = prefix;
                prefix += current;
            }

            control.consume(value_count)?;
            if pass % 2 == 0 {
                for &key in values.iter() {
                    let bucket = ((key >> shift) & RADIX_MASK) as usize;
                    let position = self.counts[bucket];
                    self.scratch[position] = key;
                    self.counts[bucket] += 1;
                }
            } else {
                for &key in &self.scratch {
                    let bucket = ((key >> shift) & RADIX_MASK) as usize;
                    let position = self.counts[bucket];
                    values[position] = key;
                    self.counts[bucket] += 1;
                }
            }
        }

        control.consume(value_count)?;
        values.dedup();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Unbounded;

    #[test]
    fn fixed_width_radix_sort_is_canonical_and_stack_safe() {
        let mut values = vec![
            encode_pair(u32::MAX, 0),
            encode_pair(1, 9),
            encode_pair(0, u32::MAX),
            encode_pair(1, 2),
            encode_pair(1, 2),
            encode_pair(0, 0),
        ];
        let mut workspace = RadixWorkspace::default();
        let result = workspace.sort_dedup(&mut values, &mut Unbounded);
        assert_eq!(result, Ok(()));
        assert_eq!(
            values,
            vec![
                encode_pair(0, 0),
                encode_pair(0, u32::MAX),
                encode_pair(1, 2),
                encode_pair(1, 9),
                encode_pair(u32::MAX, 0),
            ]
        );
        assert_eq!(logical_work(6), 14 * 6 + 13 * 2_048);
    }

    #[test]
    fn zero_and_singleton_inputs_take_the_zero_work_fast_path() {
        let mut workspace = RadixWorkspace::default();
        let mut empty = Vec::new();
        let mut singleton = vec![encode_pair(2, 3)];
        assert_eq!(workspace.sort_dedup(&mut empty, &mut Unbounded), Ok(()));
        assert_eq!(workspace.sort_dedup(&mut singleton, &mut Unbounded), Ok(()));
        assert_eq!(logical_work(0), 0);
        assert_eq!(logical_work(1), 0);
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn encoded_pairs_round_trip_without_aliasing() {
        let source = kani::any::<u32>();
        let target = kani::any::<u32>();
        let key = encode_pair(source, target);

        assert_eq!(pair_source(key), source);
        assert_eq!(pair_target(key), target);
    }

    #[kani::proof]
    fn radix_work_charge_is_exact_and_overflow_free_in_the_graph_domain() {
        let candidate_count = u64::from(kani::any::<u32>());
        let observed = logical_work(candidate_count);

        if candidate_count < 2 {
            assert_eq!(observed, 0);
        } else {
            assert_eq!(observed, 14 * candidate_count + 13 * 2_048);
        }
        assert!(observed <= 14 * u64::from(u32::MAX) + 13 * 2_048);
    }
}
