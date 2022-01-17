use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use reqwest::Client;
use serde::Deserialize;

const SERVICE_URL: &str = "https://kenkoooo.com/atcoder";

#[derive(Deserialize)]
struct ProblemModel {
    difficulty: Option<i64>,
    is_experimental: Option<bool>,
}

#[derive(Deserialize)]
struct Problem {
    id: String,
    contest_id: String,
    title: String,
}

impl Problem {
    fn generate_problem_url(&self) -> String {
        format!(
            "https://atcoder.jp/contests/{}/tasks/{}",
            self.contest_id, self.id
        )
    }
}

#[derive(Deserialize)]
struct Submission {
    id: i64,
    epoch_second: i64,
    problem_id: String,
    contest_id: String,
    user_id: String,
    language: String,
    result: String,
}

async fn fetch_submissions(user_id: &str, client: &Client) -> Result<Vec<Submission>> {
    let mut from_second = 0;
    let mut submissions = vec![];
    loop {
        let url = format!(
            "{}/atcoder-api/v3/user/submissions?user={}&from_second={}",
            SERVICE_URL, user_id, from_second
        );
        let part: Vec<Submission> = client.get(url).send().await?.json().await?;
        let next_second = part.iter().map(|x| x.epoch_second).max();
        submissions.extend(part);
        if let Some(next_second) = next_second {
            from_second = next_second + 1;
        } else {
            break;
        }
    }
    Ok(submissions)
}

async fn fetch_problems(client: &Client) -> Result<Vec<Problem>> {
    let problems: Vec<Problem> = client
        .get(format!("{}/resources/problems.json", SERVICE_URL))
        .send()
        .await?
        .json()
        .await?;
    Ok(problems)
}

async fn fetch_models(client: &Client) -> Result<BTreeMap<String, ProblemModel>> {
    let models: BTreeMap<String, ProblemModel> = client
        .get(format!("{}/resources/problem-models.json", SERVICE_URL))
        .send()
        .await?
        .json()
        .await?;
    Ok(models)
}

struct DataRow {
    problem: Problem,
    model: ProblemModel,
    last_solved: Option<DateTime<FixedOffset>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::ClientBuilder::new().gzip(true).build()?;

    let problems = fetch_problems(&client).await?;
    let mut models = fetch_models(&client).await?;
    let submissions = fetch_submissions("kenkoooo", &client).await?;

    let mut last_solved_map = BTreeMap::new();
    for submission in submissions {
        if submission.result != "AC" {
            continue;
        }

        if let Some(cur) = last_solved_map.get_mut(&submission.problem_id) {
            *cur = submission.epoch_second.max(*cur);
        } else {
            last_solved_map.insert(submission.problem_id, submission.epoch_second);
        }
    }

    let rows = problems
        .into_iter()
        .filter_map(|problem| {
            let model = models.remove(&problem.id)?;
            let last_solved = last_solved_map.remove(&problem.id).map(|epoch_second| {
                let dt = NaiveDateTime::from_timestamp(epoch_second, 0);
                FixedOffset::east(9 * 3600)
                    .from_local_datetime(&dt)
                    .unwrap()
            });
            Some(DataRow {
                problem,
                model,
                last_solved,
            })
        })
        .collect::<Vec<_>>();

    println!("title\turl\tdifficulty\tlast solved");
    for row in rows {
        let last_solved = row
            .last_solved
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(String::new);
        let is_experimental = row.model.is_experimental.unwrap_or(true);
        if is_experimental {
            continue;
        }

        let difficulty = match row.model.difficulty {
            Some(d) => d,
            None => continue,
        };
        if difficulty < 2000 {
            continue;
        }

        println!(
            "{title}\t{url}\t{difficulty}\t{last_solved}",
            title = row.problem.title,
            url = row.problem.generate_problem_url(),
            difficulty = difficulty,
            last_solved = last_solved
        );
    }

    Ok(())
}
