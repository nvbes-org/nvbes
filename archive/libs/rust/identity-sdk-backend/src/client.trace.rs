use reqwest::RequestBuilder;

const TRACEPARENT_HEADER: &str = "traceparent";

pub(crate) fn with_fresh_trace_headers(request: RequestBuilder) -> RequestBuilder {
    let trace_id = random_hex::<16>();
    let span_id = random_hex::<8>();

    request.header(
        TRACEPARENT_HEADER,
        format!("00-{}-{}-01", trace_id, span_id),
    )
}

fn random_hex<const N: usize>() -> String {
    let bytes = rand::random::<[u8; N]>();
    let mut output = String::with_capacity(N * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
