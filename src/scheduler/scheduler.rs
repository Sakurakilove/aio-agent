use super::job::{CronJob, CronSchedule};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CronScheduler {
    jobs: HashMap<String, CronJob>,
    run_count: AtomicUsize,
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            run_count: AtomicUsize::new(0),
        }
    }

    pub fn add_job(&mut self, job: CronJob) {
        self.jobs.insert(job.id.clone(), job);
    }

    pub fn remove_job(&mut self, job_id: &str) -> Option<CronJob> {
        self.jobs.remove(job_id)
    }

    pub fn enable_job(&mut self, job_id: &str) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.enabled = true;
        }
    }

    pub fn disable_job(&mut self, job_id: &str) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.enabled = false;
        }
    }

    pub fn get_due_jobs(&self) -> Vec<CronJob> {
        let now = Utc::now();
        self.jobs
            .values()
            .filter(|job| {
                job.enabled
                    && job.next_run.map(|next| next <= now).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub fn mark_job_executed(&mut self, job_id: &str) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.mark_executed();
            self.run_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn list_jobs(&self) -> Vec<&CronJob> {
        self.jobs.values().collect()
    }

    pub fn get_job(&self, job_id: &str) -> Option<&CronJob> {
        self.jobs.get(job_id)
    }

    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    pub fn enabled_job_count(&self) -> usize {
        self.jobs.values().filter(|j| j.enabled).count()
    }

    pub fn total_runs(&self) -> usize {
        self.run_count.load(Ordering::SeqCst)
    }

    pub fn get_next_run(&self) -> Option<DateTime<Utc>> {
        self.jobs
            .values()
            .filter(|j| j.enabled)
            .filter_map(|j| j.next_run)
            .min()
    }
}
