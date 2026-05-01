use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: CronSchedule,
    pub command: String,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: usize,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CronSchedule {
    Every { seconds: u64 },
    At { datetime: DateTime<Utc> },
    Recurring { cron_expression: String },
    Interval { minutes: u64 },
    Hourly,
    Daily { hour: u8, minute: u8 },
    Weekly { day_of_week: u8, hour: u8, minute: u8 },
}

impl CronJob {
    pub fn new(id: &str, name: &str, schedule: CronSchedule, command: &str) -> Self {
        let next_run = Self::calculate_next_run(&schedule);
        Self {
            id: id.to_string(),
            name: name.to_string(),
            schedule,
            command: command.to_string(),
            enabled: true,
            last_run: None,
            next_run,
            run_count: 0,
            metadata: HashMap::new(),
        }
    }

    fn calculate_next_run(schedule: &CronSchedule) -> Option<DateTime<Utc>> {
        match schedule {
            CronSchedule::Every { seconds } => {
                Some(Utc::now() + chrono::Duration::seconds(*seconds as i64))
            }
            CronSchedule::At { datetime } => Some(*datetime),
            CronSchedule::Interval { minutes } => {
                Some(Utc::now() + chrono::Duration::minutes(*minutes as i64))
            }
            CronSchedule::Hourly => {
                let now = Utc::now();
                let mins = 60 - now.minute() as i64;
                let next = now + chrono::Duration::minutes(mins);
                Some(next.with_minute(0).unwrap_or(next))
            }
            CronSchedule::Daily { hour, minute } => {
                let now = Utc::now();
                let mut next = now
                    .with_hour(*hour as u32)
                    .unwrap_or(now)
                    .with_minute(*minute as u32)
                    .unwrap_or(now)
                    .with_second(0)
                    .unwrap_or(now);
                if next <= now {
                    next += chrono::Duration::days(1);
                }
                Some(next)
            }
            CronSchedule::Weekly { day_of_week, hour, minute } => {
                let now = Utc::now();
                let current_day = now.weekday().num_days_from_monday();
                let dow = *day_of_week as u32;
                let days_until = if dow > current_day {
                    (dow - current_day) as i64
                } else {
                    (7 - current_day + dow) as i64
                };
                let mut next = now + chrono::Duration::days(days_until);
                next = next
                    .with_hour(*hour as u32)
                    .unwrap_or(next)
                    .with_minute(*minute as u32)
                    .unwrap_or(next)
                    .with_second(0)
                    .unwrap_or(next);
                if next <= now {
                    next += chrono::Duration::weeks(1);
                }
                Some(next)
            }
            CronSchedule::Recurring { cron_expression } => {
                Self::parse_cron_next(cron_expression)
            }
        }
    }

    fn parse_cron_next(expression: &str) -> Option<DateTime<Utc>> {
        let parts: Vec<&str> = expression.split_whitespace().collect();
        if parts.len() >= 2 {
            if let (Ok(minute), Ok(hour)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                let now = Utc::now();
                let mut next = now
                    .with_hour(hour)
                    .unwrap_or(now)
                    .with_minute(minute)
                    .unwrap_or(now)
                    .with_second(0)
                    .unwrap_or(now);
                if next <= now {
                    next += chrono::Duration::days(1);
                }
                return Some(next);
            }
        }
        Some(Utc::now() + chrono::Duration::hours(1))
    }

    pub fn mark_executed(&mut self) {
        self.last_run = Some(Utc::now());
        self.run_count += 1;
        self.next_run = Self::calculate_next_run(&self.schedule);
    }
}
