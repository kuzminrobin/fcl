// The code_commons crate is to be reused for various code-handling projects, such as
// * (dynamic handling) code profiling, code coverage;
// * (static handling) translation from language to language.
mod call_graph;
pub use call_graph::CallGraph;

/// Function call repeat count data type.
type RepeatCountType = usize;
/// The maximum value (saturation value) for the function call repeat count data type.
/// The function can be called in a loop endlessly, such that the function call repeat count can potentially overflow.
/// The algorithm increments that count to the maximum (saturation) value and then stops incrementing.
const REPEAT_COUNT_MAX: RepeatCountType = RepeatCountType::MAX;

pub enum RepeatCountCategory {
    Exact(RepeatCountType),   // Repeats 6 time(s). // None is REPEAT_COUNT_MAX.
    AtLeast(RepeatCountType), // Repeats 6+ time(s). // The `overall` is REPEAT_COUNT_MAX.
    Unknown,                  // Repeats ? time(s). // Both are REPEAT_COUNT_MAX.
}
impl RepeatCountCategory {
    pub fn to_string(&self) -> String {
        match self {
            RepeatCountCategory::Exact(exact) => exact.to_string(),
            RepeatCountCategory::AtLeast(at_least) => at_least.to_string() + "+",
            RepeatCountCategory::Unknown => "?".to_string(),
        }
    }
}

#[derive(Clone)]
pub enum ItemKind {
    /// Item is a function or a closure.
    Call {
        name: String,
        param_vals: Option<String>,
    },
    /// Item is a loop body.
    Loopbody,
}
impl ItemKind {
    pub fn is_call(&self) -> bool {
        if let Self::Call { .. } = self {
            true
        } else {
            false
        }
    }
    pub fn is_loopbody(&self) -> bool {
        if let Self::Loopbody { .. } = self {
            true
        } else {
            false
        }
    }
}

/// Trait to be implemented by the instances that need to be notified about the code run events
/// (such as function or closure calls, returns, etc.).
pub trait CoderunNotifiable {
    /// Non-cached call happened.
    fn notify_call(&mut self, _call_depth: usize, _name: &str, _param_vals: &Option<String>) {}
    /// Non-cached return happened.
    fn notify_return(
        &mut self,
        _call_depth: usize,
        _name: &str,
        _has_nested_calls: bool,
        _ret_val: &Option<String>,
    ) {
    }
    /// Repeat count has stopped being cached.
    fn notify_repeat_count(
        &mut self,
        _call_depth: usize,
        _kind: &ItemKind,
        _count: RepeatCountCategory,
    ) {
    }

    /// Flush needed (any output cached by this trait implementor needs to be flushed).
    fn notify_flush(&mut self) {}

    /// Loop body has stopped being cached.
    fn notify_loopbody_start(&mut self, _call_depth: usize);

    /// Loop body has ended (but not necessarily the whole loop).
    fn notify_loopbody_end(&mut self, _call_depth: usize) {}
}

/// The function call repeat count. Consists of the two parts.
/// * Actual repeat count. Stops incrementing upon reaching `REPEAT_COUNT_MAX` (saturates).
/// * The flushed part of the actual repeat count. Value less than or equal to the actual repeat count.
#[derive(Clone, Copy)]
pub struct RepeatCount {
    overall: RepeatCountType,
    flushed: RepeatCountType, // flushed <= overall
}
impl RepeatCount {
    pub fn new() -> Self {
        Self {
            overall: 0,
            flushed: 0,
        }
    }
    pub fn non_flushed(&self) -> RepeatCountCategory {
        if self.overall < REPEAT_COUNT_MAX {
            return RepeatCountCategory::Exact(self.overall - self.flushed);
        } else if self.flushed < REPEAT_COUNT_MAX {
            return RepeatCountCategory::AtLeast(self.overall - self.flushed);
        }
        RepeatCountCategory::Unknown
    }
    pub fn non_flushed_is_empty(&self) -> bool {
        // Equal but not both are saturated:
        self.overall == self.flushed && self.flushed < REPEAT_COUNT_MAX
    }
    pub fn inc(&mut self) {
        if self.overall < REPEAT_COUNT_MAX {
            self.overall += 1
        }
    }
    pub fn mark_flushed(&mut self) {
        self.flushed = self.overall
    }
}
impl core::cmp::PartialEq for RepeatCount {
    fn eq(&self, other: &Self) -> bool {
        self.overall.eq(&other.overall)
    }
}
