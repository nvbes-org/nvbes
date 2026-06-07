use sentry::protocol::Stacktrace;

pub(crate) fn scrub_stacktrace(stacktrace: &mut Stacktrace) {
    stacktrace.registers.clear();
    for frame in &mut stacktrace.frames {
        frame.abs_path = None;
        frame.context_line = None;
        frame.pre_context.clear();
        frame.post_context.clear();
        frame.vars.clear();
    }
}
