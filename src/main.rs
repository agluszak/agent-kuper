use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Deserializer};
use std::env;

/// Convert timestamp (ms) to `DateTime<Utc>`
fn parse_timestamp(ms: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(ms)
        .single()
        .expect("Invalid timestamp")
}

/// Custom deserializer for timestamps
fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp: i64 = Deserialize::deserialize(deserializer)?;
    Ok(parse_timestamp(timestamp))
}

/// Struct for project details
#[derive(Debug, Deserialize)]
struct Project {
    key: String, // Project name (e.g., "BAZEL")
}

/// Enum to represent PR states
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum ReviewState {
    Opened,
    Closed,
    Deleted,
}

impl std::fmt::Display for ReviewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewState::Opened => write!(f, "Opened"),
            ReviewState::Closed => write!(f, "Closed"),
            ReviewState::Deleted => write!(f, "Deleted"),
        }
    }
}

/// Struct for the inner "review" field in the API response
#[derive(Debug, Deserialize)]
struct Review {
    number: i32, // PR ID
    title: String,
    state: ReviewState,
    project: Project,

    #[serde(deserialize_with = "deserialize_timestamp", rename = "createdAt")]
    created_at: DateTime<Utc>,
}

/// Wrapper struct matching the API response structure
#[derive(Debug, Deserialize)]
struct ReviewWrapper {
    review: Review,
}

/// Root API response
#[derive(Debug, Deserialize)]
struct ApiResponse {
    data: Vec<ReviewWrapper>,
}

/// Fetch PRs from JetBrains Space API.
fn fetch_prs_for_month(
    date: NaiveDate,
    space_domain: &str,
    user_id: &str,
    project_id: &str,
    token: &str,
) -> Result<Vec<Review>, Box<dyn std::error::Error>> {
    let month = date.month();
    let year = date.year();

    let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
        .pred_opt()
        .unwrap();

    // API request URL
    let url = format!(
        "https://{}/api/http/projects/id:{}/code-reviews?state=&author={}&from={}&to={}&$fields=data(review(title,createdAt,state,number,project))",
        space_domain, project_id, user_id, start_date, end_date
    );

    let client = Client::new();
    let response = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/json")
        .send()?;

    let response_text = response.text()?; // Get raw JSON to debug
    let api_response = serde_json::from_str::<ApiResponse>(&response_text)?;

    Ok(api_response
        .data
        .into_iter()
        .map(|wrapper| wrapper.review)
        .collect::<Vec<Review>>())
}

fn last_day_of_month(date: NaiveDate) -> NaiveDate {
    let year = date.year();
    let month = date.month();
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_month_year = if month == 12 { year + 1 } else { year };

    NaiveDate::from_ymd_opt(next_month_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

fn render_template(
    prs: Vec<Review>,
    month: NaiveDate,
    percent_creative: i32,
    space_domain: &str,
    user_name: &str,
) -> String {
    let prs = prs
        .iter()
        .enumerate()
        .filter_map(|(idx, pr)| render_pr(idx as i32 + 1, pr, space_domain, user_name))
        .collect::<Vec<String>>()
        .join("\n");

    let last_day_of_month = last_day_of_month(month).format("%d.%m.%Y").to_string();
    let month = month.format("%m.%Y").to_string();

    TEMPLATE
        .replace("{prs}", &prs)
        .replace("{month}", &month)
        .replace("{percent_creative}", &percent_creative.to_string())
        .replace("{last_day_of_month}", &last_day_of_month)
}

fn render_pr(idx: i32, pr: &Review, domain: &str, author: &str) -> Option<String> {
    let project = &pr.project.key;
    let number = pr.number;
    let url = format!("https://{domain}/p/{project}/reviews/{number}");
    let link = format!("[{}]({})", url, url);
    let status = match pr.state {
        ReviewState::Opened => "Unfinished",
        ReviewState::Closed => "Finished",
        ReviewState::Deleted => return None,
    };
    let creation_date = pr.created_at.format("%d.%m.%Y").to_string();
    Some(format!(
        "| {} | {} | {} | {} | {} | {} |",
        idx,
        pr.title.replace("|", "\\|"),
        author,
        link,
        status,
        creation_date
    ))
}

const TEMPLATE: &str = r#"
---
title: "Formularz rejestracji czasu pracy twórczej"
author: "JetBrains Poland sp. z o.o."
date: "{last_day_of_month}"
geometry: a4paper,landscape,margin=2cm
fontsize: 10pt
mainfont: DejaVu Serif
lang: pl-PL
---

Warszawa, {last_day_of_month}

# Formularz rejestracji czasu pracy twórczej i Utworów w JetBrains Poland spółka z ograniczoną odpowiedzialnością / Registration form for creative time and Works at JetBrains Poland spółka z ograniczoną odpowiedzialnością

## Dotyczy miesiąca / Concerns the month of: {month}

| Lp. | Utwór / Work | Autor / Author | Forma ustalenia / Form of the Work’s establishment | Status | Data powstania / Date of creation |
| --- | ----------------------------------------------- | ------------------- | ------------------------ | ---------------- | -------------------- |
{prs}

### Total % of actual working time spent by creative time: {percent_creative}%


## Oświadczenie Pracownika

Niniejszym potwierdzam, że według mojej najlepszej wiedzy wskazany/e wyżej utwór/utwory stanowi/ą wynik mojej działalności twórczej o indywidualnym charakterze chroniony/e przepisami ustawy z dnia 4 lutego 1994 r. O prawie autorskim i prawach pokrewnych (t.j.: Dz.U. z 2022 r., poz. 2509). Ponadto oświadczam, że w miesiącu, którego dotyczy to oświadczenie mój czas pracy kreatywnej nad tworzeniem ww. Utworu/Utworów w stosunku do całego efektywnego czasu pracy (z wyłączeniem nieobecności w pracy) wyniósł {percent_creative} procent.

## Employee’s declaration

I hereby declare that, to my best knowledge, this (these) work(s) is (are) the result of my creative activity of an individual character protected by the provisions of the Act of 4 February 1994 on Copyright and Related Rights (consolidated text: Journal of Laws of 2022, item 2509). I also declare that, in the month which this declaration concerns, I have worked creatively on creating the above Work(s) for {percent_creative} percent of the entire effective working time (i.e., excluding absences at work).
"#;

fn previous_month() -> NaiveDate {
    let now = Utc::now();
    NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let month = args.get(1).map_or_else(previous_month, |m| {
        NaiveDate::parse_from_str(&format!("{}-01", m), "%Y-%m-%d")
            .expect("Invalid date format, use YYYY-MM")
    });

    let space_domain = env::var("SPACE_DOMAIN").expect("Missing SPACE_DOMAIN in env");
    let project_id = env::var("SPACE_PROJECT_ID").expect("Missing SPACE_PROJECT_ID in env");
    let token = env::var("SPACE_TOKEN").expect("Missing SPACE_TOKEN in env");
    let user_id = env::var("SPACE_USER_ID").expect("Missing SPACE_USER_ID in env");
    let user_name = env::var("USER_NAME").expect("Missing USER_NAME in env");
    let percent_creative = env::var("PERCENT_CREATIVE")
        .expect("Missing PERCENT_CREATIVE in env")
        .parse::<i32>()
        .expect("Invalid PERCENT_CREATIVE");

    let prs = fetch_prs_for_month(month, &space_domain, &user_id, &project_id, &token)?;

    let rendered = render_template(prs, month, percent_creative, &space_domain, &user_name);
    println!("{}", rendered);

    Ok(())
}
