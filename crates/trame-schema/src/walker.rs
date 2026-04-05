//! Schema-driven document walker.
//!
//! [`SchemaWalker`] is a state machine that walks a sequence of X12 segments
//! against a [`TransactionSetDef`], emitting [`WalkEvent`]s as it recognizes
//! loops and segments.

use crate::types::{LoopDef, QualifierMatch, SegmentRef, TransactionSetDef};

/// Event emitted by the schema walker as it processes segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkEvent<'a> {
    /// Entered a new loop.
    LoopStart {
        loop_id: &'static str,
        iteration: u32,
    },
    /// A segment matched at this position in the schema.
    SegmentMatch {
        segment_id: &'a [u8],
        loop_id: Option<&'static str>,
        schema_ref: &'static SegmentRef,
    },
    /// A segment did not match the expected schema position.
    SegmentUnexpected {
        segment_id: &'a [u8],
        expected: Vec<&'static str>,
    },
    /// Exited a loop.
    LoopEnd {
        loop_id: &'static str,
    },
}

/// Phase of the transaction set we are currently processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalkPhase {
    Header,
    HeaderLoops,
    Detail,
    Summary,
    Trailer,
    Done,
}

/// Tracks position within a single loop.
#[derive(Debug, Clone)]
struct LoopState {
    loop_def: &'static LoopDef,
    /// Index into `loop_def.segments`.
    segment_idx: usize,
    /// Index into `loop_def.children`.
    child_idx: usize,
    /// Current iteration count (1-based).
    iteration: u32,
}

/// Schema walker -- walks segments against a transaction set definition.
///
/// Feed segments one at a time via [`SchemaWalker::feed`] and receive walk
/// events that describe how the segment maps to the schema.
///
/// # Example
///
/// ```
/// use trame_schema::{Registry, SchemaWalker};
///
/// let reg = Registry::new();
/// let schema = reg.lookup("997", "005010").unwrap();
/// let mut walker = SchemaWalker::new(schema);
///
/// let event = walker.feed(b"ST", None);
/// // event is SegmentMatch for ST in header
/// ```
pub struct SchemaWalker {
    schema: &'static TransactionSetDef,
    /// Stack of active loops (outermost first).
    loop_stack: Vec<LoopState>,
    phase: WalkPhase,
    /// Index within the current flat segment list.
    seg_cursor: usize,
    /// Index within the current loop list.
    loop_cursor: usize,
}

impl SchemaWalker {
    /// Create a new walker for the given transaction set schema.
    pub fn new(schema: &'static TransactionSetDef) -> Self {
        Self {
            schema,
            loop_stack: Vec::new(),
            phase: WalkPhase::Header,
            seg_cursor: 0,
            loop_cursor: 0,
        }
    }

    /// Has the walker reached the end of the transaction set?
    pub fn is_complete(&self) -> bool {
        self.phase == WalkPhase::Done
    }

    /// Feed a segment to the walker and receive a walk event.
    ///
    /// - `segment_id`: the segment identifier bytes (e.g., `b"ST"`, `b"HL"`).
    /// - `qualifier`: optional qualifier element value for disambiguating
    ///   loops that share the same trigger segment. For `HL` segments this
    ///   is typically element 03 (the hierarchical level code).
    pub fn feed<'a>(
        &mut self,
        segment_id: &'a [u8],
        qualifier: Option<&[u8]>,
    ) -> WalkEvent<'a> {
        let seg_str = std::str::from_utf8(segment_id).unwrap_or("");

        match self.phase {
            WalkPhase::Header => self.match_flat_segments(
                segment_id,
                seg_str,
                qualifier,
                self.schema.header_segments,
            ),
            WalkPhase::HeaderLoops => {
                self.match_loop_phase(segment_id, seg_str, qualifier, self.schema.header_loops)
            }
            WalkPhase::Detail => {
                self.match_loop_phase(segment_id, seg_str, qualifier, self.schema.detail_loops)
            }
            WalkPhase::Summary => {
                self.match_loop_phase(segment_id, seg_str, qualifier, self.schema.summary_loops)
            }
            WalkPhase::Trailer => self.match_flat_segments(
                segment_id,
                seg_str,
                qualifier,
                self.schema.trailer_segments,
            ),
            WalkPhase::Done => WalkEvent::SegmentUnexpected {
                segment_id,
                expected: vec![],
            },
        }
    }

    /// Try to match against a flat (non-loop) segment list.
    fn match_flat_segments<'a>(
        &mut self,
        segment_id: &'a [u8],
        seg_str: &str,
        qualifier: Option<&[u8]>,
        segments: &'static [SegmentRef],
    ) -> WalkEvent<'a> {
        // Try to match at or after current cursor position.
        for (i, seg_ref) in segments.iter().enumerate().skip(self.seg_cursor) {
            if seg_ref.segment_id == seg_str {
                self.seg_cursor = i + 1;
                // If we've consumed all flat segments, advance to the next phase.
                if self.seg_cursor >= segments.len() {
                    self.advance_phase();
                }
                return WalkEvent::SegmentMatch {
                    segment_id,
                    loop_id: None,
                    schema_ref: seg_ref,
                };
            }
        }

        // Not found in current flat list -- try advancing to the next phase and
        // re-matching there.
        self.advance_phase();
        if self.phase != WalkPhase::Done {
            return self.feed(segment_id, qualifier);
        }

        let expected: Vec<&'static str> = segments[self.seg_cursor.min(segments.len())..]
            .iter()
            .map(|s| s.segment_id)
            .collect();
        WalkEvent::SegmentUnexpected {
            segment_id,
            expected,
        }
    }

    /// Try to match against a loop-based phase.
    fn match_loop_phase<'a>(
        &mut self,
        segment_id: &'a [u8],
        seg_str: &str,
        qualifier: Option<&[u8]>,
        loops: &'static [LoopDef],
    ) -> WalkEvent<'a> {
        // First, try matching within the current loop stack.
        if let Some(event) = self.try_match_in_stack(segment_id, seg_str, qualifier) {
            return event;
        }

        // Try to start a new top-level loop in this phase.
        for (i, loop_def) in loops.iter().enumerate().skip(self.loop_cursor) {
            if loop_def.trigger_segment == seg_str
                && qualifier_matches(loop_def.qualifier.as_ref(), qualifier)
            {
                // Close any open loops from the stack.
                self.close_all_loops();
                self.loop_cursor = i;
                self.loop_stack.push(LoopState {
                    loop_def,
                    segment_idx: 1, // trigger segment consumed
                    child_idx: 0,
                    iteration: 1,
                });
                return WalkEvent::LoopStart {
                    loop_id: loop_def.id,
                    iteration: 1,
                };
            }
        }

        // Check if we can re-enter a loop we've already seen (e.g., a repeating
        // HL loop at the same level).
        for (i, loop_def) in loops.iter().enumerate().take(self.loop_cursor) {
            if loop_def.trigger_segment == seg_str
                && qualifier_matches(loop_def.qualifier.as_ref(), qualifier)
            {
                self.close_all_loops();
                self.loop_cursor = i;
                self.loop_stack.push(LoopState {
                    loop_def,
                    segment_idx: 1,
                    child_idx: 0,
                    iteration: 1,
                });
                return WalkEvent::LoopStart {
                    loop_id: loop_def.id,
                    iteration: 1,
                };
            }
        }

        // Not found in any loop -- advance to next phase.
        self.close_all_loops();
        self.advance_phase();
        if self.phase != WalkPhase::Done {
            return self.feed(segment_id, qualifier);
        }

        let expected: Vec<&'static str> = loops.iter().map(|l| l.trigger_segment).collect();
        WalkEvent::SegmentUnexpected {
            segment_id,
            expected,
        }
    }

    /// Try to match the segment within the current loop stack.
    fn try_match_in_stack<'a>(
        &mut self,
        segment_id: &'a [u8],
        seg_str: &str,
        qualifier: Option<&[u8]>,
    ) -> Option<WalkEvent<'a>> {
        if self.loop_stack.is_empty() {
            return None;
        }

        let depth = self.loop_stack.len();
        let state = &self.loop_stack[depth - 1];
        let loop_def = state.loop_def;

        // Check for a segment match in the current loop's remaining segments.
        for (i, seg_ref) in loop_def
            .segments
            .iter()
            .enumerate()
            .skip(state.segment_idx)
        {
            if seg_ref.segment_id == seg_str {
                let state = self.loop_stack.last_mut().unwrap();
                state.segment_idx = i + 1;
                return Some(WalkEvent::SegmentMatch {
                    segment_id,
                    loop_id: Some(loop_def.id),
                    schema_ref: seg_ref,
                });
            }
        }

        // Check child loops.
        let child_start = state.child_idx;
        for (i, child) in loop_def
            .children
            .iter()
            .enumerate()
            .skip(child_start)
        {
            if child.trigger_segment == seg_str
                && qualifier_matches(child.qualifier.as_ref(), qualifier)
            {
                let state = self.loop_stack.last_mut().unwrap();
                state.child_idx = i;
                self.loop_stack.push(LoopState {
                    loop_def: child,
                    segment_idx: 1,
                    child_idx: 0,
                    iteration: 1,
                });
                return Some(WalkEvent::LoopStart {
                    loop_id: child.id,
                    iteration: 1,
                });
            }
        }

        // Also check if a child loop earlier in the list can re-trigger
        // (new iteration of a repeating child loop).
        for (i, child) in loop_def.children.iter().enumerate().take(child_start) {
            if child.trigger_segment == seg_str
                && qualifier_matches(child.qualifier.as_ref(), qualifier)
            {
                let state = self.loop_stack.last_mut().unwrap();
                state.child_idx = i;
                self.loop_stack.push(LoopState {
                    loop_def: child,
                    segment_idx: 1,
                    child_idx: 0,
                    iteration: 1,
                });
                return Some(WalkEvent::LoopStart {
                    loop_id: child.id,
                    iteration: 1,
                });
            }
        }

        // Check if the trigger segment of the current loop is repeating
        // (new iteration of the same loop).
        if loop_def.trigger_segment == seg_str
            && qualifier_matches(loop_def.qualifier.as_ref(), qualifier)
            && state.iteration < loop_def.repeat_count
        {
            let new_iter = state.iteration + 1;
            let loop_id = loop_def.id;
            let state = self.loop_stack.last_mut().unwrap();
            state.segment_idx = 1;
            state.child_idx = 0;
            state.iteration = new_iter;
            return Some(WalkEvent::LoopStart {
                loop_id,
                iteration: new_iter,
            });
        }

        // Pop the current loop and try the parent.
        self.loop_stack.pop();
        if !self.loop_stack.is_empty() {
            return self.try_match_in_stack(segment_id, seg_str, qualifier);
        }

        None
    }

    /// Close all open loops on the stack.
    fn close_all_loops(&mut self) {
        self.loop_stack.clear();
    }

    /// Advance to the next walk phase.
    fn advance_phase(&mut self) {
        match self.phase {
            WalkPhase::Header => {
                self.seg_cursor = 0;
                self.loop_cursor = 0;
                if self.schema.header_loops.is_empty() {
                    self.phase = WalkPhase::Detail;
                } else {
                    self.phase = WalkPhase::HeaderLoops;
                }
            }
            WalkPhase::HeaderLoops => {
                self.loop_cursor = 0;
                self.phase = WalkPhase::Detail;
            }
            WalkPhase::Detail => {
                self.loop_cursor = 0;
                if self.schema.summary_loops.is_empty() {
                    self.seg_cursor = 0;
                    self.phase = WalkPhase::Trailer;
                } else {
                    self.phase = WalkPhase::Summary;
                }
            }
            WalkPhase::Summary => {
                self.seg_cursor = 0;
                self.phase = WalkPhase::Trailer;
            }
            WalkPhase::Trailer => {
                self.phase = WalkPhase::Done;
            }
            WalkPhase::Done => {}
        }
    }
}

/// Check if a qualifier value matches the loop's qualifier requirement.
fn qualifier_matches(requirement: Option<&QualifierMatch>, actual: Option<&[u8]>) -> bool {
    match requirement {
        None => true,
        Some(qm) => match actual {
            None => false,
            Some(val) => {
                let val_str = std::str::from_utf8(val).unwrap_or("");
                qm.values.contains(&val_str)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Registry;

    fn get_997() -> &'static TransactionSetDef {
        let reg = Registry::new();
        reg.lookup("997", "005010").unwrap()
    }

    fn get_270() -> &'static TransactionSetDef {
        let reg = Registry::new();
        reg.lookup("270", "005010").unwrap()
    }

    fn get_837p() -> &'static TransactionSetDef {
        let reg = Registry::new();
        reg.lookup("837", "005010").unwrap()
    }

    #[test]
    fn walk_997_simple() {
        let schema = get_997();
        let mut walker = SchemaWalker::new(schema);

        // ST -- header segment
        let ev = walker.feed(b"ST", None);
        assert!(matches!(ev, WalkEvent::SegmentMatch { loop_id: None, .. }));

        // AK1 -- header segment
        let ev = walker.feed(b"AK1", None);
        assert!(matches!(ev, WalkEvent::SegmentMatch { loop_id: None, .. }));

        // AK2 -- starts the AK2 loop
        let ev = walker.feed(b"AK2", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "AK2", iteration: 1 }
        ));

        // AK5 -- segment inside AK2 loop
        let ev = walker.feed(b"AK5", None);
        assert!(matches!(
            ev,
            WalkEvent::SegmentMatch { loop_id: Some("AK2"), .. }
        ));

        // AK9 -- trailer segment (closes AK2 loop)
        let ev = walker.feed(b"AK9", None);
        assert!(matches!(ev, WalkEvent::SegmentMatch { loop_id: None, .. }));

        // SE -- trailer segment
        let ev = walker.feed(b"SE", None);
        assert!(matches!(ev, WalkEvent::SegmentMatch { loop_id: None, .. }));

        assert!(walker.is_complete());
    }

    #[test]
    fn walk_997_with_ak3_ak4() {
        let schema = get_997();
        let mut walker = SchemaWalker::new(schema);

        walker.feed(b"ST", None);
        walker.feed(b"AK1", None);

        // AK2 loop
        let ev = walker.feed(b"AK2", None);
        assert!(matches!(ev, WalkEvent::LoopStart { loop_id: "AK2", .. }));

        // AK3 child loop
        let ev = walker.feed(b"AK3", None);
        assert!(matches!(ev, WalkEvent::LoopStart { loop_id: "AK3", .. }));

        // AK4 inside AK3 loop
        let ev = walker.feed(b"AK4", None);
        assert!(matches!(
            ev,
            WalkEvent::SegmentMatch { loop_id: Some("AK3"), .. }
        ));

        // AK5 -- back in AK2 loop (AK3 loop closes)
        let ev = walker.feed(b"AK5", None);
        assert!(matches!(
            ev,
            WalkEvent::SegmentMatch { loop_id: Some("AK2"), .. }
        ));

        walker.feed(b"AK9", None);
        walker.feed(b"SE", None);
        assert!(walker.is_complete());
    }

    #[test]
    fn walk_997_multiple_ak2_iterations() {
        let schema = get_997();
        let mut walker = SchemaWalker::new(schema);

        walker.feed(b"ST", None);
        walker.feed(b"AK1", None);

        // First AK2 iteration
        let ev = walker.feed(b"AK2", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "AK2", iteration: 1 }
        ));
        walker.feed(b"AK5", None);

        // Second AK2 iteration (re-trigger)
        let ev = walker.feed(b"AK2", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "AK2", iteration: 2 }
        ));
        walker.feed(b"AK5", None);

        walker.feed(b"AK9", None);
        walker.feed(b"SE", None);
        assert!(walker.is_complete());
    }

    #[test]
    fn walk_270_hl_hierarchy() {
        let schema = get_270();
        let mut walker = SchemaWalker::new(schema);

        // Header
        walker.feed(b"ST", None);
        walker.feed(b"BHT", None);

        // 2000A -- HL with qualifier "20"
        let ev = walker.feed(b"HL", Some(b"20"));
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2000A", .. }
        ));

        // 2100A -- NM1 inside 2000A
        let ev = walker.feed(b"NM1", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2100A", .. }
        ));

        // 2000B -- HL with qualifier "21"
        let ev = walker.feed(b"HL", Some(b"21"));
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2000B", .. }
        ));

        // 2100B -- NM1 inside 2000B
        let ev = walker.feed(b"NM1", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2100B", .. }
        ));

        // 2000C -- HL with qualifier "22"
        let ev = walker.feed(b"HL", Some(b"22"));
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2000C", .. }
        ));

        // TRN in 2000C
        let ev = walker.feed(b"TRN", None);
        assert!(matches!(
            ev,
            WalkEvent::SegmentMatch { loop_id: Some("2000C"), .. }
        ));

        // 2100C -- NM1 inside 2000C
        let ev = walker.feed(b"NM1", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2100C", .. }
        ));

        // 2110C -- EQ
        let ev = walker.feed(b"EQ", None);
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2110C", .. }
        ));

        // DTP inside 2110C
        let ev = walker.feed(b"DTP", None);
        assert!(matches!(
            ev,
            WalkEvent::SegmentMatch { loop_id: Some("2110C"), .. }
        ));

        // Trailer
        walker.feed(b"SE", None);
        assert!(walker.is_complete());
    }

    #[test]
    fn walk_270_qualifier_mismatch() {
        let schema = get_270();
        let mut walker = SchemaWalker::new(schema);

        walker.feed(b"ST", None);
        walker.feed(b"BHT", None);

        // HL with wrong qualifier -- should be unexpected
        let ev = walker.feed(b"HL", Some(b"99"));
        assert!(matches!(ev, WalkEvent::SegmentUnexpected { .. }));
    }

    #[test]
    fn unexpected_segment_detection() {
        let schema = get_997();
        let mut walker = SchemaWalker::new(schema);

        walker.feed(b"ST", None);
        walker.feed(b"AK1", None);

        // XYZ is not in the schema at all
        let ev = walker.feed(b"XYZ", None);
        assert!(matches!(ev, WalkEvent::SegmentUnexpected { .. }));
    }

    #[test]
    fn walker_is_complete_initially_false() {
        let schema = get_997();
        let walker = SchemaWalker::new(schema);
        assert!(!walker.is_complete());
    }

    #[test]
    fn walk_837p_header_loops() {
        let schema = get_837p();
        let mut walker = SchemaWalker::new(schema);

        // Header segments
        walker.feed(b"ST", None);
        walker.feed(b"BHT", None);

        // 1000A -- NM1 with qualifier "41"
        let ev = walker.feed(b"NM1", Some(b"41"));
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "1000A", .. }
        ));

        // PER in 1000A
        let ev = walker.feed(b"PER", None);
        assert!(matches!(
            ev,
            WalkEvent::SegmentMatch { loop_id: Some("1000A"), .. }
        ));

        // 1000B -- NM1 with qualifier "40"
        let ev = walker.feed(b"NM1", Some(b"40"));
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "1000B", .. }
        ));

        // Now into detail phase: 2000A
        let ev = walker.feed(b"HL", Some(b"20"));
        assert!(matches!(
            ev,
            WalkEvent::LoopStart { loop_id: "2000A", .. }
        ));
    }

    #[test]
    fn walk_270_multiple_subscribers() {
        let schema = get_270();
        let mut walker = SchemaWalker::new(schema);

        walker.feed(b"ST", None);
        walker.feed(b"BHT", None);

        // First 2000A
        let ev = walker.feed(b"HL", Some(b"20"));
        assert!(matches!(ev, WalkEvent::LoopStart { loop_id: "2000A", .. }));
        walker.feed(b"NM1", None); // 2100A

        // 2000B
        let ev = walker.feed(b"HL", Some(b"21"));
        assert!(matches!(ev, WalkEvent::LoopStart { loop_id: "2000B", .. }));
        walker.feed(b"NM1", None); // 2100B

        // First 2000C
        let ev = walker.feed(b"HL", Some(b"22"));
        assert!(matches!(ev, WalkEvent::LoopStart { loop_id: "2000C", iteration: 1 }));
        walker.feed(b"NM1", None); // 2100C

        // Second 2000C (new subscriber)
        let ev = walker.feed(b"HL", Some(b"22"));
        assert!(matches!(ev, WalkEvent::LoopStart { loop_id: "2000C", .. }));
        walker.feed(b"NM1", None); // 2100C

        walker.feed(b"SE", None);
        assert!(walker.is_complete());
    }
}
