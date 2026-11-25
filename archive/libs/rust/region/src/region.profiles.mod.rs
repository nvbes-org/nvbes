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
    let slices = [
        europe::EUROPE_PROFILES_SLICES,
        americas::AMERICAS_PROFILES_SLICES,
        asia_pacific::ASIA_PACIFIC_PROFILES_SLICES,
        africa_me::AFRICA_ME_PROFILES_SLICES,
    ];
    let capacity = slices
        .iter()
        .flat_map(|group| group.iter().copied())
        .map(|slice| slice.len())
        .sum();
    let mut v = Vec::with_capacity(capacity);
    for slice in slices.iter().flat_map(|group| group.iter().copied()) {
        v.extend_from_slice(slice);
    }
    v
});
