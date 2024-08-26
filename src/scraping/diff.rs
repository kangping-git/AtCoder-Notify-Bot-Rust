use core::f64;
use std::collections::BTreeMap;

use std::collections::HashMap;

use super::ranking_types;

#[derive(Debug, Default, Clone)]
#[allow(unused)]
struct UserRow {
    is_rated: bool,
    rating: i32,
    prev_contests: i32,
    user_name: String,
    retreated: bool,
    task_info: BTreeMap<String, UserTaskInfo>,
    last_ac: i64,
    raw_rating: f64,
}

#[derive(Debug, Default, Clone)]
struct UserTaskInfo {
    score: f64,
    time: f64,
    elapsed: f64,
    ac: f64,
}

#[derive(Debug, Default, Clone)]
pub struct Model {
    pub slope: f64,
    pub intercept: f64,
    pub variance: f64,
    pub difficulty: f64,
    pub discrimination: f64,
}

pub fn get_diff(standing_data: ranking_types::StandingsJson, contest_is_rated: bool) -> BTreeMap<String, Model> {
    let task_ids = standing_data.TaskInfo.iter().map(|x| x.TaskScreenName.clone()).collect::<Vec<String>>();

    let mut user_results = vec![];
    let mut standings_data = standing_data.StandingsData;
    standings_data.sort_by(|a, b| a.Rank.cmp(&b.Rank));

    let mut standings = vec![];
    for result_row in standings_data {
        let total_submission = result_row.TotalResult.Count;
        let retreated = total_submission == 0;

        let is_rated = result_row.IsRated;
        let rating = result_row.OldRating;
        let prev_contests = result_row.Competitions;
        let user_name = result_row.UserScreenName;

        if !retreated && (is_rated || !contest_is_rated) {
            standings.push(user_name.clone());
        }
        let mut user_row: UserRow = UserRow {
            is_rated,
            rating,
            prev_contests,
            user_name,
            retreated,
            ..Default::default()
        };

        for task_id in task_ids.iter() {
            user_row.task_info.insert(
                task_id.clone(),
                UserTaskInfo {
                    score: 0.0,
                    time: -1.0,
                    elapsed: 10_f64.powi(100),
                    ac: 0.0,
                },
            );
        }

        let mut prev_accepted_time = vec![0];
        for i in &result_row.TaskResults {
            if i.1.Score > 0 {
                prev_accepted_time.push(i.1.Elapsed);
            }
        }

        user_row.last_ac = *prev_accepted_time.iter().max().unwrap_or(&0);
        for (task_screen_name, task_result) in &result_row.TaskResults {
            let task_id = task_screen_name.clone();
            let task_info = user_row.task_info.get_mut(&task_id).unwrap();
            task_info.score = task_result.Score as f64;
            if task_result.Score > 0 {
                let penalty = task_result.Penalty as f64 * 5.0 * 60.0 * (10.0_f64.powf(9.0));
                task_info.time = task_result.Elapsed as f64 + penalty;
                let max_accepted_time = prev_accepted_time.iter().filter(|&t| *t < task_result.Elapsed).max().cloned().unwrap_or(0);
                task_info.time = task_result.Elapsed as f64 + penalty - max_accepted_time as f64;
                task_info.elapsed = task_result.Elapsed as f64;
                task_info.ac = if task_result.Status == 1 { 1.0 } else { 0.0 };
            }
        }
        user_results.push(user_row);
    }

    let mut user_results_by_problem: HashMap<String, Vec<UserRow>> = HashMap::new();

    for task_screen_name in &task_ids {
        user_results_by_problem.entry(task_screen_name.clone()).or_default().extend(user_results.iter().cloned());
    }

    let mut models: BTreeMap<String, Model> = BTreeMap::new();
    let task_id_to_assignment: BTreeMap<String, String> = standing_data.TaskInfo.iter().map(|x| (x.TaskScreenName.clone(), x.Assignment.clone())).collect();

    for task_id in &task_ids {
        let mut model: Model = Default::default();
        let max_score = user_results_by_problem
            .get(task_id)
            .unwrap()
            .iter()
            .map(|x| x.task_info.get(task_id).unwrap().score)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        if max_score == 0.0 {
            println!("The problem {} is not solved by any competitors. skipping.", task_id);
            continue;
        }

        for task_result in &mut user_results {
            task_result.task_info.get_mut(task_id).unwrap().ac *= if task_result.task_info.get(task_id).unwrap().score == max_score {
                1.0
            } else {
                0.0
            };
        }
        let elapsed: Vec<f64> = user_results_by_problem.get(task_id).unwrap().iter().map(|x| x.task_info.get(task_id).unwrap().elapsed).collect();
        let first_ac: f64 = *elapsed.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        let mut recurring_users: Vec<UserRow> =
            user_results.iter().filter(|task_result| task_result.prev_contests > 0 && task_result.rating > 0).cloned().collect();
        for task_result in recurring_users.iter_mut() {
            task_result.raw_rating = inverse_adjust_rating(task_result.rating, task_result.prev_contests);
        }
        let time_model_sample_users: Vec<UserRow> = recurring_users
            .iter()
            .filter(|task_result| {
                let task_info = task_result.task_info.get(task_id).unwrap();
                task_info.time > first_ac / 2.0 && task_info.ac > 0.99
            })
            .cloned()
            .collect();
        if time_model_sample_users.len() < 5 {
            println!(
                "{}: insufficient data ({} users). skip estimating time model.",
                task_id,
                time_model_sample_users.len()
            );
        } else {
            let raw_ratings = time_model_sample_users.iter().map(|x| x.raw_rating).collect::<Vec<f64>>();
            let time_secs = time_model_sample_users.iter().map(|x| x.task_info.get(task_id).unwrap().time / 10.0_f64.powi(9)).collect::<Vec<f64>>();
            let time_logs = time_secs.iter().map(|x| x.log(f64::consts::E)).collect::<Vec<f64>>();
            let (slope, intercept) = single_regression(raw_ratings, time_logs);
            println!("{}: time [sec] = exp({} * raw_rating + {})", task_id, slope, intercept);
            if slope > 0.0 {
                println!("slope is positive. ignoring unreliable estimation.")
            } else {
                model.slope = slope;
                model.intercept = intercept;
                let variance_data: Vec<f64> = time_model_sample_users
                    .iter()
                    .map(|x| {
                        let task_info = x.task_info.get(task_id).unwrap();
                        let time_log = task_info.time.log(f64::consts::E);
                        model.slope * x.raw_rating + model.intercept - time_log
                    })
                    .collect();
                model.variance = variance(variance_data);
            }
        }
        println!("{:?}", model);

        let difficulty_dataset = if is_very_easy_problem(task_id) {
            recurring_users.iter().filter(|task_result| task_result.is_rated && !task_result.retreated).collect::<Vec<&UserRow>>()
        } else if is_agc_easiest_problem(task_id) {
            recurring_users.iter().collect()
        } else {
            recurring_users.iter().filter(|task_result| !task_result.retreated).collect::<Vec<&UserRow>>()
        };
        if difficulty_dataset.len() < 40 {
            println!(
                "{}: insufficient data ({} users). skip estimating difficulty model.",
                task_id,
                difficulty_dataset.len()
            );
        } else if !difficulty_dataset.iter().any(|task_result| task_result.task_info.get(task_id).unwrap().ac > 0.0) {
            println!("no contestants got AC. skip estimating difficulty model.");
        } else {
            let d_raw_ratings = difficulty_dataset.iter().map(|x| x.raw_rating).collect::<Vec<f64>>();
            let d_accepteds = difficulty_dataset.iter().map(|x| x.task_info.get(task_id).unwrap().ac).collect::<Vec<f64>>();
            let (difficulty, discrimination) = if is_agc_easiest_problem(task_id) {
                let (difficulty, discrimination, _) = fit_3plm_irt(&d_raw_ratings, &d_accepteds);
                (difficulty, discrimination)
            } else {
                fit_2plm_irt(&d_raw_ratings, &d_accepteds)
            };
            println!("difficulty: {difficulty}, discrimination: {discrimination}");
            model.difficulty = difficulty;
            model.discrimination = discrimination;
        }
        models.insert(task_id_to_assignment[task_id].clone(), model);
    }
    models
}

fn inverse_adjust_rating(rating: i32, prev_contests: i32) -> f64 {
    if rating <= 0 {
        return f64::NAN;
    }
    let mut rating = rating as f64;
    if rating <= 400.0 {
        rating = 400.0 * (1.0 - (400.0 / rating).log(f64::consts::E))
    }
    let adjustment = ((1.0 - (0.9_f64.powf(2.0 * prev_contests as f64))).sqrt() / (1.0 - 0.9_f64.powi(prev_contests)) - 1.0) / (19.0_f64.sqrt() - 1.0) * 1200.0;
    rating + adjustment
}

fn single_regression(x: Vec<f64>, y: Vec<f64>) -> (f64, f64) {
    let n = x.len();
    let x_sum: f64 = x.iter().sum();
    let y_sum: f64 = y.iter().sum();
    let xy_sum: f64 = x.iter().zip(y.iter()).map(|(&xi, &yi)| xi * yi).sum();
    let sqx_sum: f64 = x.iter().map(|&xi| xi.powi(2)).sum();
    let slope = (n as f64 * xy_sum - x_sum * y_sum) / (n as f64 * sqx_sum - x_sum.powi(2));
    let intercept = (sqx_sum * y_sum - xy_sum * x_sum) / (n as f64 * sqx_sum - x_sum.powi(2));
    (slope, intercept)
}
fn variance(data: Vec<f64>) -> f64 {
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    data.iter()
        .map(|value| {
            let diff = mean - *value;
            diff * diff
        })
        .sum::<f64>()
        / (data.len() as f64 - 1.0)
}
fn is_very_easy_problem(task_screen_name: &str) -> bool {
    task_screen_name.starts_with("abc")
        && task_screen_name.chars().last().map(|c| c == 'a' || c == 'b').unwrap_or(false)
        && task_screen_name[3..6].parse::<i32>().unwrap_or(0) >= 42
}
fn is_agc_easiest_problem(task_screen_name: &str) -> bool {
    task_screen_name.starts_with("agc") && task_screen_name.ends_with("_a")
}

fn fit_3plm_irt(xs: &Vec<f64>, ys: &[f64]) -> (f64, f64, f64) {
    let accepts: f64 = ys.iter().sum();
    let mut iterations: Vec<(f64, f64, f64, f64)> = Vec::new();
    for retreat_proba in frange(0.0, 0.5, 0.025) {
        let participate_proba = 1.0 - retreat_proba;
        let (difficulty, discrimination) = _fit_1plm_binary_search(xs, accepts / participate_proba);
        let mut logl = 0.0;
        for (&x, &y) in xs.iter().zip(ys.iter()) {
            let p = participate_proba * safe_sigmoid(discrimination * (x - difficulty));
            logl += safe_log(if y == 1.0 { p } else { 1.0 - p });
        }
        iterations.push((logl, difficulty, discrimination, retreat_proba));
    }
    let (logl, difficulty, discrimination, _) = iterations.into_iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap();
    (difficulty, discrimination, logl)
}

fn _fit_1plm_binary_search(xs: &Vec<f64>, positive_count: f64) -> (f64, f64) {
    let discrimination = f64::ln(6.0) / 400.0;
    let (mut lb, mut ub) = (-10000, 10000);
    let accepts = positive_count;
    while ub - lb > 1 {
        let m = (ub + lb) / 2;
        let mut expected_accepts = 0.0;
        for x in xs {
            expected_accepts += 1.0 / (1.0 + (6.0f64.powf((m as f64 - x) / 400.0)));
        }
        if expected_accepts < accepts {
            ub = m;
        } else {
            lb = m;
        }
    }
    let difficulty = lb;
    (difficulty as f64, discrimination)
}

fn frange(start: f64, end: f64, step: f64) -> impl Iterator<Item = f64> {
    std::iter::successors(Some(start), move |&v| if (start - end) * (v - end) > 0.0 { Some(v + step) } else { None })
}

fn safe_sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp().min(750.0))
}

fn safe_log(x: f64) -> f64 {
    x.max(10f64.powi(-100)).ln()
}
fn fit_2plm_irt(xs: &Vec<f64>, ys: &[f64]) -> (f64, f64) {
    _fit_1plm_binary_search(xs, ys.iter().sum())
}
