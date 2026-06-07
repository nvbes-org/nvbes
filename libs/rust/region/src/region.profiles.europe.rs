use crate::RegionProfile;

#[path = "region.profiles.europe.part_1.rs"]
mod part_1;
#[path = "region.profiles.europe.part_2.rs"]
mod part_2;
#[path = "region.profiles.europe.part_3.rs"]
mod part_3;

pub const EUROPE_PROFILES_SLICES: &[&[RegionProfile]] = &[
    part_1::EUROPE_PROFILES_PART_1,
    part_2::EUROPE_PROFILES_PART_2,
    part_3::EUROPE_PROFILES_PART_3,
];
