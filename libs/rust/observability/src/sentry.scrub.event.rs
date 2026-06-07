use sentry::protocol::{Breadcrumb, Event};

use super::{
    sanitize_path_like, scrub_request, scrub_sensitive_string, scrub_stacktrace, scrub_tag_map,
    scrub_value_map,
};

pub(crate) fn scrub_event(mut event: Event<'static>) -> Option<Event<'static>> {
    event.user = None;

    if let Some(request) = event.request.as_mut() {
        scrub_request(request);
    }
    if let Some(message) = event.message.as_mut() {
        *message = scrub_sensitive_string(message);
    }
    if let Some(logentry) = event.logentry.as_mut() {
        logentry.message = scrub_sensitive_string(&logentry.message);
        for param in &mut logentry.params {
            super::scrub_json_value(param);
        }
    }
    if let Some(transaction) = event.transaction.as_mut() {
        *transaction = sanitize_path_like(transaction);
    }

    event.breadcrumbs.values = event
        .breadcrumbs
        .values
        .into_iter()
        .filter_map(scrub_breadcrumb)
        .collect();

    for exception in &mut event.exception.values {
        if let Some(value) = exception.value.as_mut() {
            *value = scrub_sensitive_string(value);
        }
        if let Some(stacktrace) = exception.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace);
        }
        if let Some(stacktrace) = exception.raw_stacktrace.as_mut() {
            scrub_stacktrace(stacktrace);
        }
    }

    if let Some(stacktrace) = event.stacktrace.as_mut() {
        scrub_stacktrace(stacktrace);
    }

    scrub_value_map(&mut event.extra);
    scrub_tag_map(&mut event.tags);

    Some(event)
}

pub(crate) fn scrub_breadcrumb(mut breadcrumb: Breadcrumb) -> Option<Breadcrumb> {
    if breadcrumb
        .category
        .as_deref()
        .is_some_and(|category| category.starts_with("console"))
    {
        return None;
    }

    if let Some(message) = breadcrumb.message.as_mut() {
        *message = scrub_sensitive_string(message);
    }
    scrub_value_map(&mut breadcrumb.data);

    Some(breadcrumb)
}
