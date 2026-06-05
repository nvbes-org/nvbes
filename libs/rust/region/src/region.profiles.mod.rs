use crate::RegionProfile;
use std::sync::LazyLock;

#[path = "region.profiles.africa_me.rs"]
mod africa_me;
#[path = "region.profiles.americas.rs"]
mod americas;
#[path = "region.profiles.asia_pacific.rs"]
mod asia_pacific;
#[path = "region.profiles.europe.rs"]
mod europe;

pub static ALL_PROFILES: LazyLock<Vec<RegionProfile>> = LazyLock::new(|| {
    let mut v = Vec::with_capacity(
        europe::EUROPE_PROFILES.len()
            + americas::AMERICAS_PROFILES.len()
            + asia_pacific::ASIA_PACIFIC_PROFILES.len()
            + africa_me::AFRICA_ME_PROFILES.len(),
    );
    v.extend_from_slice(europe::EUROPE_PROFILES);
    v.extend_from_slice(americas::AMERICAS_PROFILES);
    v.extend_from_slice(asia_pacific::ASIA_PACIFIC_PROFILES);
    v.extend_from_slice(africa_me::AFRICA_ME_PROFILES);
    v
});
