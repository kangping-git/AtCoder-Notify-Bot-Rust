#[derive(Default, Debug)]
pub enum ContestType {
    #[default]
    Algorithm = 0,
    Heuristic = 1,
}
#[derive(Default, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum ContestRatingType {
    ABC,
    ARC,
    AGC,
    #[default]
    None,
}

#[derive(Default, Debug)]
pub struct Contest {
    pub contest_name: String,
    pub start_time: String,
    pub contest_duration: i32,
    pub contest_type: ContestType,
    pub url: String,
    pub contest_rating_type: ContestRatingType,
    pub rating_ragnge: (i32, i32),
    pub rating_range_raw: String,
    pub contest_id: String,
}
