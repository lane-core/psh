//! Job table for psh.
//!
//! rc heritage: jobs are process groups. The table tracks background
//! and suspended jobs by their process group id. Each job records
//! the pids of all processes in the group (for pipelines) and the
//! display string shown by `jobs`.
//!
//! ksh93 heritage: job numbers are 1-based and reuse slots from
//! completed jobs.

use crate::exec::Status;

/// A job's current state.
#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    /// Running in background.
    Running,
    /// Stopped by signal (SIGTSTP, SIGSTOP).
    Stopped,
    /// Completed with a status.
    Done(Status),
}

/// A single job — a process group running in the background or stopped.
#[derive(Debug, Clone)]
pub struct Job {
    /// Process group id (the first pid in the pipeline).
    pub pgid: libc::pid_t,
    /// All pids in this job (pipeline stages).
    pub pids: Vec<libc::pid_t>,
    /// The command string for display (`jobs` output).
    pub command: String,
    /// Current status.
    pub status: JobStatus,
}

/// The job table — indexed by job number (1-based).
///
/// Slots are `Option<Job>`: `None` means the slot is free for reuse.
/// This avoids shifting indices when jobs complete. The shell prints
/// "[n] Done" when a completed job is noticed, then frees the slot.
#[derive(Debug, Default)]
pub struct JobTable {
    jobs: Vec<Option<Job>>,
}

impl JobTable {
    pub fn new() -> Self {
        JobTable { jobs: Vec::new() }
    }

    /// Insert a new job, returning its 1-based job number.
    ///
    /// Reuses the first free slot, or appends a new one.
    pub fn insert(&mut self, job: Job) -> usize {
        for (i, slot) in self.jobs.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(job);
                return i + 1;
            }
        }
        self.jobs.push(Some(job));
        self.jobs.len()
    }

    /// Get a reference to a job by 1-based number.
    pub fn get(&self, job_num: usize) -> Option<&Job> {
        self.jobs.get(job_num.wrapping_sub(1))?.as_ref()
    }

    /// Get a mutable reference to a job by 1-based number.
    pub fn get_mut(&mut self, job_num: usize) -> Option<&mut Job> {
        self.jobs.get_mut(job_num.wrapping_sub(1))?.as_mut()
    }

    /// Remove a job by number (free the slot).
    pub fn remove(&mut self, job_num: usize) {
        if let Some(slot) = self.jobs.get_mut(job_num.wrapping_sub(1)) {
            *slot = None;
        }
    }

    /// Find the job number for a given pgid.
    pub fn find_by_pgid(&self, pgid: libc::pid_t) -> Option<usize> {
        for (i, slot) in self.jobs.iter().enumerate() {
            if let Some(job) = slot {
                if job.pgid == pgid {
                    return Some(i + 1);
                }
            }
        }
        None
    }

    /// Find the job containing a given pid.
    pub fn find_by_pid(&self, pid: libc::pid_t) -> Option<usize> {
        for (i, slot) in self.jobs.iter().enumerate() {
            if let Some(job) = slot {
                if job.pids.contains(&pid) {
                    return Some(i + 1);
                }
            }
        }
        None
    }

    /// Iterate all active (non-None) jobs with their 1-based numbers.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &Job)> {
        self.jobs
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|j| (i + 1, j)))
    }

    /// Reap a child: update the job's status based on waitpid results.
    ///
    /// `pid` is the reaped child. `wstatus` is the raw waitpid status.
    /// Returns the job number if the pid was found in the table.
    pub fn reap(&mut self, pid: libc::pid_t, wstatus: i32) -> Option<usize> {
        let job_num = self.find_by_pid(pid)?;
        let job = self.get_mut(job_num)?;

        if libc::WIFSTOPPED(wstatus) {
            job.status = JobStatus::Stopped;
        } else {
            // Child exited or was killed by signal.
            // For pipelines, mark done only when all pids have exited.
            // Since we get one reap per pid, remove this pid from the list.
            job.pids.retain(|&p| p != pid);
            if job.pids.is_empty() {
                let status = if libc::WIFEXITED(wstatus) {
                    Status::from_code(libc::WEXITSTATUS(wstatus))
                } else if libc::WIFSIGNALED(wstatus) {
                    Status::err(format!("signal {}", libc::WTERMSIG(wstatus)))
                } else {
                    Status::err("unknown exit")
                };
                job.status = JobStatus::Done(status);
            }
        }
        Some(job_num)
    }

    /// Collect completed jobs, printing "[n] Done" for each, and free their slots.
    ///
    /// Returns the list of (job_number, command) pairs that were completed,
    /// so the caller can print them.
    pub fn collect_done(&mut self) -> Vec<(usize, String)> {
        let mut done = Vec::new();
        for (i, slot) in self.jobs.iter_mut().enumerate() {
            let is_done = slot
                .as_ref()
                .is_some_and(|j| matches!(j.status, JobStatus::Done(_)));
            if is_done {
                let job = slot.take().unwrap();
                done.push((i + 1, job.command));
            }
        }
        done
    }

    /// The most recent job number (highest active slot), or None.
    pub fn current_job(&self) -> Option<usize> {
        for (i, slot) in self.jobs.iter().enumerate().rev() {
            if slot.is_some() {
                return Some(i + 1);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job(pgid: libc::pid_t, pids: &[libc::pid_t], cmd: &str) -> Job {
        Job {
            pgid,
            pids: pids.to_vec(),
            command: cmd.into(),
            status: JobStatus::Running,
        }
    }

    #[test]
    fn job_table_tracks_background() {
        let mut table = JobTable::new();
        let num = table.insert(make_job(100, &[100], "sleep 10 &"));
        assert_eq!(num, 1);
        assert_eq!(table.get(1).unwrap().pgid, 100);
        assert!(matches!(table.get(1).unwrap().status, JobStatus::Running));
    }

    #[test]
    fn job_table_reap_updates_status() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100], "sleep 10 &"));

        // Simulate waitpid returning: child exited with code 0
        // WIFEXITED(wstatus) is true when wstatus & 0x7f == 0
        // WEXITSTATUS extracts bits 8-15
        let wstatus = 0; // exit code 0, normal termination
        let result = table.reap(100, wstatus);
        assert_eq!(result, Some(1));
        let job = table.get(1).unwrap();
        assert!(matches!(job.status, JobStatus::Done(ref s) if s.is_success()));
    }

    #[test]
    fn job_table_reap_pipeline() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100, 101, 102], "cat | grep | sort"));

        // Reap first two — job should still be running (pids remain)
        table.reap(101, 0);
        assert!(matches!(table.get(1).unwrap().status, JobStatus::Running));
        table.reap(100, 0);
        assert!(matches!(table.get(1).unwrap().status, JobStatus::Running));

        // Reap last — job is now done
        table.reap(102, 0);
        assert!(matches!(table.get(1).unwrap().status, JobStatus::Done(_)));
    }

    #[test]
    fn collect_done_frees_slots() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100], "cmd1 &"));
        table.insert(make_job(200, &[200], "cmd2 &"));

        // Mark first as done
        table.reap(100, 0);
        let done = table.collect_done();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].0, 1);

        // Slot 1 is now free
        assert!(table.get(1).is_none());
        // Slot 2 still active
        assert!(table.get(2).is_some());
    }

    #[test]
    fn slot_reuse() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100], "first &"));
        table.insert(make_job(200, &[200], "second &"));

        // Complete and collect first
        table.reap(100, 0);
        table.collect_done();

        // New job should reuse slot 1
        let num = table.insert(make_job(300, &[300], "third &"));
        assert_eq!(num, 1);
        assert_eq!(table.get(1).unwrap().pgid, 300);
    }

    #[test]
    fn find_by_pgid() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100, 101], "pipeline"));
        assert_eq!(table.find_by_pgid(100), Some(1));
        assert_eq!(table.find_by_pgid(999), None);
    }

    #[test]
    fn find_by_pid() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100, 101], "pipeline"));
        assert_eq!(table.find_by_pid(101), Some(1));
        assert_eq!(table.find_by_pid(999), None);
    }

    #[test]
    fn stopped_status() {
        let mut table = JobTable::new();
        table.insert(make_job(100, &[100], "vim"));

        // WIFSTOPPED: wstatus & 0xff == 0x7f and signal in bits 8-15
        // On most platforms, stopped status = (signal << 8) | 0x7f
        let wstatus = (libc::SIGTSTP << 8) | 0x7f;
        table.reap(100, wstatus);
        assert!(matches!(table.get(1).unwrap().status, JobStatus::Stopped));
    }
}
