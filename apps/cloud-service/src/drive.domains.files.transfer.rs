#[path = "drive.domains.files.transfer.download_url.rs"]
mod download_url;
#[path = "drive.domains.files.transfer.range.rs"]
mod range;
#[path = "drive.domains.files.transfer.stream.rs"]
mod stream;

use super::types::DownloadUrlResponse;
pub use download_url::create_download_url;
pub use stream::download_object;

pub(super) enum ResolvedRange {
    Full,
    Partial { start: i64, end_inclusive: i64 },
    Unsatisfiable,
}
