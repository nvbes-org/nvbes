use std::sync::{Arc, OnceLock};

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub fn install() -> Arc<PrometheusHandle> {
    static HANDLE: OnceLock<Arc<PrometheusHandle>> = OnceLock::new();
    HANDLE
        .get_or_init(|| {
            Arc::new(
                PrometheusBuilder::new()
                    .install_recorder()
                    .expect("trust/risk Prometheus recorder must install"),
            )
        })
        .clone()
}
