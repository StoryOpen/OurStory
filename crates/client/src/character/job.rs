use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct Job(pub u32);

#[derive(Debug, Clone, Reflect)]
pub struct JobEntry {
    pub job: Job,
    pub label: String,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct JobCatalog {
    pub entries: Vec<JobEntry>,
}

impl JobCatalog {
    pub fn label_for(&self, job: Job) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.job == job)
            .map(|entry| entry.label.as_str())
    }

    pub fn display_label(&self, job: Job) -> String {
        self.label_for(job)
            .map(str::to_string)
            .unwrap_or_else(|| {
                warn!("JobCatalog::display_label: job {} not found, using fallback label", job.0);
                format!("Job {}", job.0)
            })
    }

    pub fn next_after(&self, job: Job) -> Option<Job> {
        if self.entries.is_empty() {
            return None;
        }

        let next_idx = self
            .entries
            .iter()
            .position(|entry| entry.job == job)
            .map(|idx| (idx + 1) % self.entries.len())
            .unwrap_or_else(|| {
                warn!("JobCatalog::next_after: job {} not found in catalog, wrapping to first", job.0);
                0
            });
        Some(self.entries[next_idx].job)
    }
}

pub fn load_job_catalog(wz: &wz::WzData) -> JobCatalog {
    let class_names = match wz.list_children("Skill") {
        Ok(names) => names,
        Err(e) => {
            warn!("load_job_catalog: failed to list Skill children: {e}, returning empty");
            return JobCatalog::default();
        }
    };

    let mut entries = Vec::new();
    for class_name in class_names {
        let Some(job_key) = class_name.strip_suffix(".img") else {
            continue;
        };
        let Ok(job_id) = job_key.parse::<u32>() else {
            continue;
        };
        let Some(label) = wz.read_string(&format!("String/Skill.img/{job_key}/bookName"))
            .map(|label: String| label.trim().to_string())
            .filter(|label| !label.is_empty())
        else {
            continue;
        };

        entries.push(JobEntry {
            job: Job(job_id),
            label,
        });
    }

    entries.sort_by_key(|entry| entry.job.0);
    JobCatalog { entries }
}

impl Job {
    pub fn parent(&self) -> Option<Job> {
        Some(Job(match self.0 {
            100 => return None,
            110 | 200 | 300 | 400 | 500 => 100,
            111 | 120 | 130 => 110,
            112 => 111,
            121 => 120,
            122 => 121,
            131 => 130,
            132 => 131,
            210 | 220 | 230 => 200,
            211 => 210,
            212 => 211,
            221 => 220,
            222 => 221,
            231 => 230,
            232 => 231,
            310 | 320 => 300,
            311 => 310,
            312 => 311,
            321 => 320,
            322 => 321,
            410 | 420 => 400,
            411 => 410,
            412 => 411,
            421 => 420,
            422 => 421,
            510 | 520 => 500,
            511 => 510,
            512 => 511,
            521 => 520,
            522 => 521,
            1000 => return None,
            1100 | 1200 | 1300 | 1400 | 1500 => 1000,
            1110 => 1100,
            1111 => 1110,
            1112 => 1111,
            1210 => 1200,
            1211 => 1210,
            1212 => 1211,
            1310 => 1300,
            1311 => 1310,
            1312 => 1311,
            1410 => 1400,
            1411 => 1410,
            1412 => 1411,
            1510 => 1500,
            1511 => 1510,
            1512 => 1511,
            2000 => return None,
            2100 => 2000,
            2110 => 2100,
            2111 => 2110,
            2112 => 2111,
            800 | 900 | 910 => return None,
            _ => return None,
        }))
    }

    pub fn lineage(&self) -> Vec<Job> {
        let mut jobs = vec![*self];
        let mut current = *self;
        while let Some(parent) = current.parent() {
            jobs.push(parent);
            current = parent;
        }
        jobs.reverse();
        jobs
    }
}
