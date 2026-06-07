use crate::RegionProfile;

#[path = "region.profiles.asia_pacific.part_1.rs"]
mod part_1;
#[path = "region.profiles.asia_pacific.part_2.rs"]
mod part_2;
#[path = "region.profiles.asia_pacific.part_3.rs"]
mod part_3;

pub const ASIA_PACIFIC_PROFILES_SLICES: &[&[RegionProfile]] = &[
    part_1::ASIA_PACIFIC_PROFILES_PART_1,
    part_2::ASIA_PACIFIC_PROFILES_PART_2,
    part_3::ASIA_PACIFIC_PROFILES_PART_3,
];
