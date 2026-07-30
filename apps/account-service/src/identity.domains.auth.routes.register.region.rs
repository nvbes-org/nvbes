use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use nvbes_region::{is_country_allowed, supported_data_regions, supported_profiles};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use utoipa::ToSchema;

const SUPPORTED_REGIONS_CACHE_CONTROL: &str =
    "public, max-age=86400, stale-while-revalidate=604800";

static SUPPORTED_REGIONS_CACHE: OnceLock<SupportedRegionsCache> = OnceLock::new();

#[derive(Serialize, ToSchema)]
pub(crate) struct RegionResponse {
    region: Option<String>,
}

#[utoipa::path(
    get,
    path = "/auth/region",
    tag = "auth",
    responses(
        (status = 200, description = "Detected region from request headers", body = RegionResponse),
    ),
)]
pub(crate) async fn region(headers: HeaderMap) -> Json<RegionResponse> {
    Json(RegionResponse {
        region: crate::http::request::region_from_headers(&headers),
    })
}

#[derive(Clone, Serialize, ToSchema)]
pub(crate) struct SupportedRegionResponse {
    country_code: &'static str,
    data_region: &'static str,
    legal_jurisdiction: &'static str,
    primary_timezone: &'static str,
    timezones: Vec<&'static str>,
    sub_region: Option<&'static str>,
    display_name: Option<&'static str>,
    hosting_strategy: &'static str,
    is_european_exclusive: bool,
}

struct SupportedRegionsCache {
    etag: String,
    body: Vec<SupportedRegionResponse>,
}

fn supported_region_catalog() -> Vec<SupportedRegionResponse> {
    let allowed_data_regions = supported_data_regions();
    let mut regions: Vec<_> = supported_profiles()
        .iter()
        .filter(|profile| {
            allowed_data_regions.contains(&profile.data_region)
                && is_country_allowed(profile.country_code)
        })
        .map(|profile| SupportedRegionResponse {
            country_code: profile.country_code,
            data_region: profile.data_region.as_str(),
            legal_jurisdiction: profile.legal_jurisdiction.as_str(),
            primary_timezone: profile.primary_timezone.as_str(),
            timezones: profile.timezones.iter().map(|tz| tz.as_str()).collect(),
            sub_region: profile.sub_region,
            display_name: profile.display_name,
            hosting_strategy: profile.data_region.hosting_strategy(),
            is_european_exclusive: profile.data_region.is_european_exclusive(),
        })
        .collect();

    regions.sort_by(|left, right| left.country_code.cmp(right.country_code));
    regions
}

fn supported_regions_cache() -> &'static SupportedRegionsCache {
    SUPPORTED_REGIONS_CACHE.get_or_init(|| {
        let body = supported_region_catalog();
        let json = serde_json::to_vec(&body).expect("supported regions catalog must serialize");
        let hash = Sha256::digest(&json);

        SupportedRegionsCache {
            etag: format!("\"regions-{}\"", hex::encode(hash)),
            body,
        }
    })
}

fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .map(str::trim)
                .any(|candidate| candidate == "*" || candidate == etag)
        })
}

fn insert_supported_regions_cache_headers(headers: &mut HeaderMap, etag: &str) {
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(SUPPORTED_REGIONS_CACHE_CONTROL),
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(etag).expect("supported regions ETag must be a valid header"),
    );
}

#[utoipa::path(
    get,
    path = "/auth/regions",
    tag = "auth",
    responses(
        (status = 200, description = "Supported region catalog", body = [SupportedRegionResponse]),
        (status = 304, description = "Supported region catalog not modified"),
    ),
)]
pub(crate) async fn supported_regions(headers: HeaderMap) -> Response {
    let cache = supported_regions_cache();

    if if_none_match_matches(&headers, &cache.etag) {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        insert_supported_regions_cache_headers(response.headers_mut(), &cache.etag);
        return response;
    }

    let mut response = Json(cache.body.clone()).into_response();
    insert_supported_regions_cache_headers(response.headers_mut(), &cache.etag);
    response
}

#[cfg(test)]
mod tests {
    use super::{if_none_match_matches, supported_region_catalog, supported_regions_cache};
    use axum::http::{HeaderMap, HeaderValue, header};

    #[test]
    fn supported_regions_cache_uses_strong_etag() {
        let cache = supported_regions_cache();

        assert!(cache.etag.starts_with("\"regions-"));
        assert!(cache.etag.ends_with('"'));
        assert!(!cache.body.is_empty());
    }

    #[test]
    fn supported_regions_catalog_is_sorted_for_stable_hashing() {
        let catalog = supported_region_catalog();
        let mut sorted_codes: Vec<_> = catalog.iter().map(|region| region.country_code).collect();
        sorted_codes.sort_unstable();

        assert_eq!(
            catalog
                .iter()
                .map(|region| region.country_code)
                .collect::<Vec<_>>(),
            sorted_codes
        );
    }

    #[test]
    fn if_none_match_accepts_matching_etag_and_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"other\", \"regions-test\""),
        );

        assert!(if_none_match_matches(&headers, "\"regions-test\""));

        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));

        assert!(if_none_match_matches(&headers, "\"regions-test\""));
    }
}
