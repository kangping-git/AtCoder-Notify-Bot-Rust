use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Clone)]
#[allow(non_snake_case)]
pub struct StandingsJson {
    pub Fixed: bool,
    pub AdditionalColumns: Option<String>,
    pub TaskInfo: Vec<TaskInfo>,
    pub StandingsData: Vec<StandingsData>,
    pub Translation: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct TaskInfo {
    pub Assignment: String,
    pub TaskName: String,
    pub TaskScreenName: String,
}

#[derive(Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct StandingsData {
    pub Rank: i32,
    pub Additional: Option<i32>,
    pub UserName: String,
    pub UserScreenName: String,
    pub UserIsDeleted: bool,
    pub Affiliation: String,
    pub Country: String,
    pub Rating: i32,
    pub OldRating: i32,
    pub IsRated: bool,
    pub IsTeam: bool,
    pub Competitions: i32,
    pub AtCoderRank: i32,
    pub TaskResults: BTreeMap<String, TaskResults>,
    pub TotalResult: TotalResult,
}

#[derive(Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct TotalResult {
    pub Count: i32,
    pub Accepted: i32,
    pub Penalty: i32,
    pub Score: i64,
    pub Elapsed: i64,
    pub Frozen: bool,
    pub Additional: Option<i32>,
}

#[derive(Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct TaskResults {
    pub Count: i32,
    pub Failure: i32,
    pub Penalty: i32,
    pub Score: i64,
    pub Elapsed: i64,
    pub Status: i32,
    pub Pending: bool,
    pub Frozen: bool,
    pub SubmissionID: i64,
    pub Additional: Option<i32>,
}
