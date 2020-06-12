//! Code coverage analysis

mod hooks;

use crate::Feature;
use crate::InstrFeatureWithoutTag;

use std::convert::TryFrom;
use std::mem::MaybeUninit;

use crate::data_structures::HBitSet;

type PC = usize;

static mut SHARED_SENSOR: MaybeUninit<CodeCoverageSensor> = MaybeUninit::<CodeCoverageSensor>::uninit();

/// Returns a reference to the only `CodeCoverageSensor`
pub fn shared_sensor() -> &'static mut CodeCoverageSensor {
    unsafe { &mut *SHARED_SENSOR.as_mut_ptr() }
}

/// Records the code coverage of the program and converts it into `Feature`s
/// that the `pool` can understand.
pub struct CodeCoverageSensor {
    pub is_recording: bool,
    eight_bit_counters: &'static mut [u8],
    features: HBitSet,
}

macro_rules! make_instr_feature_without_tag {
    ($pc:ident, $arg1:ident, $arg2:ident) => {
        { 
            (($pc & 0x2F_FFFF) << Feature::id_offset()) | (($arg1 ^ $arg2).count_ones() as usize)
        }
    };
}

impl CodeCoverageSensor {
    /// Handles a `trace_cmp` hook from Sanitizer Coverage, by recording it
    /// as a `Feature` of kind `instruction`.
    #[inline]
    fn handle_trace_cmp_u8(&mut self, pc: PC, arg1: u8, arg2: u8) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.features.set(f);
    }
    #[inline]
    fn handle_trace_cmp_u16(&mut self, pc: PC, arg1: u16, arg2: u16) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.features.set(f);
    }
    #[inline]
    fn handle_trace_cmp_u32(&mut self, pc: PC, arg1: u32, arg2: u32) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.features.set(f);
    }
    #[inline]
    fn handle_trace_cmp_u64(&mut self, pc: PC, arg1: u64, arg2: u64) {
        let f = make_instr_feature_without_tag!(pc, arg1, arg2);
        self.features.set(f);
    }
    /// Handles a `trace_indir` hook from Sanitizer Coverage, by recording it
    /// as a `Feature` of kind `indirect`.
    // #[inline]
    // fn handle_trace_indir(&mut self, caller: PC, callee: PC) {
    //     let f = Feature::indir(caller ^ callee).0 as usize; // TODO: not correct!
    //     self.features.set(f);
    // }

    /// Runs the closure on all recorded features.
    pub(crate) fn iterate_over_collected_features<F>(&mut self, mut handle: F)
    where
        F: FnMut(Feature) -> (),
    {
        const CHUNK_SIZE: usize = 32;
        let length_chunks = self.eight_bit_counters.len() / CHUNK_SIZE;
        let zero: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];

        for i in 0..length_chunks {
            let start = i * CHUNK_SIZE;
            let end = start + CHUNK_SIZE;

            let slice = <&[u8; CHUNK_SIZE]>::try_from(&self.eight_bit_counters[start..end]).unwrap();
            if slice == &zero {
                continue;
            }
            for (j, x) in slice.iter().enumerate() {
                if *x == 0 {
                    continue;
                }
                let f = Feature::edge(start + j, u16::from(*x));
                handle(f);
            }
        }

        let start_remainder = length_chunks * CHUNK_SIZE;
        let remainder = &self.eight_bit_counters[start_remainder..];
        for (j, x) in remainder.iter().enumerate() {
            let i = start_remainder + j;
            if *x == 0 {
                continue;
            }
            let f = Feature::edge(i, u16::from(*x));
            handle(f);
        }

        self.features.drain(|f| {
            handle(Feature::from_instr(InstrFeatureWithoutTag(f)));
        });
    }

    pub fn clear(&mut self) {
        for x in self.eight_bit_counters.iter_mut() {
            *x = 0;
        }
        self.features.drain(|_| {});
    }
}
