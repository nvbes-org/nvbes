use proptest::prelude::*;

use super::{TraceParent, child_traceparent, parse_traceparent};

proptest! {
    #[test]
    fn arbitrary_input_never_panics(raw in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let input = String::from_utf8_lossy(&raw);
        let _ = parse_traceparent(&input);
    }

    #[test]
    fn round_trip_valid_traceparent(
        trace_id in "[0-9a-f]{31}[1-9a-f]",
        span_id in "[0-9a-f]{15}[1-9a-f]",
        sampled in any::<bool>(),
    ) {
        let parent = TraceParent {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            sampled,
        };
        let header = parent.to_header_value();
        let parsed = parse_traceparent(&header);

        prop_assert_eq!(parsed, Some(parent));
    }

    #[test]
    fn child_preserves_trace_id_and_changes_span_id(
        trace_id in "[0-9a-f]{31}[1-9a-f]",
        span_id in "[0-9a-f]{15}[1-9a-f]",
        sampled in any::<bool>(),
    ) {
        let parent = TraceParent {
            trace_id,
            span_id,
            sampled,
        };
        let child = child_traceparent(&parent);

        prop_assert_eq!(&child.trace_id, &parent.trace_id);
        prop_assert_eq!(child.sampled, parent.sampled);
        prop_assert_ne!(&child.span_id, &parent.span_id);
        prop_assert_eq!(child.span_id.len(), 16);
    }
}
