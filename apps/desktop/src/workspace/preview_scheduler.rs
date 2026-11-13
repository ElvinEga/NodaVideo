//! Single-flight, coalescing scheduler for preview frame renders.
//!
//! Playhead changes (scrubbing, playback, edits) can arrive far faster than one
//! FFmpeg frame extraction. The old code stored a `Task` in `preview_frame_task`
//! and replaced it on every refresh, which cancelled in-flight work and let an
//! older result overwrite a newer one. Instead:
//!
//! * at most one render is in flight;
//! * while busy, only the most recent request is kept (older pending ones are dropped);
//! * results are applied monotonically, so a late result can never overwrite a newer frame;
//! * an in-flight render is never cancelled — it completes, then the newest request starts.
//!
//! This module has no GPUI or media-engine dependency: it is a pure state machine
//! over a payload `T` (the requested timeline frame in production).
//!
//! ## Ownership contract
//! * Behavior owner: `WorkspaceView`, which owns the scheduler and the task handle.
//! * Presentation owner: `PreviewMonitorPanel`, which only displays the latest completed frame.
//! * Regression test: `PreviewScheduler` unit tests below (coalescing, monotonic apply).

/// Outcome of requesting a render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule<T> {
    /// Nothing is in flight: begin rendering this request immediately.
    Start(u64, T),
    /// A render is already running; this request replaced any older pending one.
    Coalesced,
}

/// Outcome of finishing a render.
#[derive(Debug, PartialEq, Eq)]
pub struct Completion<T> {
    /// The result is newer than the last one applied, so it is safe to display.
    pub apply: bool,
    /// The result corresponds to the newest request made so far.
    pub is_latest: bool,
    /// The next request to render, if requests arrived while this one was running.
    pub next: Option<(u64, T)>,
}

/// Pure single-flight scheduler.
#[derive(Debug)]
pub struct PreviewScheduler<T> {
    next_id: u64,
    latest: u64,
    last_applied: u64,
    in_flight: Option<u64>,
    pending: Option<(u64, T)>,
}

impl<T> Default for PreviewScheduler<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PreviewScheduler<T> {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            latest: 0,
            last_applied: 0,
            in_flight: None,
            pending: None,
        }
    }

    /// Register a render request. Only the newest pending request survives.
    pub fn request(&mut self, payload: T) -> Schedule<T> {
        self.next_id += 1;
        let id = self.next_id;
        self.latest = id;

        if self.in_flight.is_none() {
            self.in_flight = Some(id);
            Schedule::Start(id, payload)
        } else {
            self.pending = Some((id, payload));
            Schedule::Coalesced
        }
    }

    /// Report that request `id` finished rendering.
    ///
    /// `apply` is monotonic: a result older than the last displayed one is rejected.
    /// `next` carries the newest coalesced request, and marks it as in flight.
    pub fn complete(&mut self, id: u64) -> Completion<T> {
        let apply = id > self.last_applied;
        if apply {
            self.last_applied = id;
        }
        let is_latest = id == self.latest;

        if self.in_flight == Some(id) {
            self.in_flight = None;
        }

        let next = self.pending.take().map(|(next_id, payload)| {
            self.in_flight = Some(next_id);
            (next_id, payload)
        });

        Completion {
            apply,
            is_latest,
            next,
        }
    }

    /// The request currently being rendered, if any.
    #[cfg(test)]
    pub fn in_flight(&self) -> Option<u64> {
        self.in_flight
    }

    /// The most recently requested id.
    #[cfg(test)]
    pub fn latest(&self) -> u64 {
        self.latest
    }

    /// Whether `id` is still the newest request.
    #[cfg(test)]
    pub fn is_current(&self, id: u64) -> bool {
        id == self.latest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[core::prelude::v1::test]
    fn first_request_starts_immediately() {
        let mut scheduler = PreviewScheduler::new();
        assert_eq!(scheduler.request(10), Schedule::Start(1, 10));
        assert_eq!(scheduler.in_flight(), Some(1));
    }

    #[core::prelude::v1::test]
    fn requests_while_busy_are_coalesced_to_the_newest() {
        let mut scheduler = PreviewScheduler::new();
        assert_eq!(scheduler.request(10), Schedule::Start(1, 10));
        assert_eq!(scheduler.request(11), Schedule::Coalesced);
        assert_eq!(scheduler.request(12), Schedule::Coalesced);
        assert_eq!(scheduler.in_flight(), Some(1));
        assert_eq!(scheduler.latest(), 3);

        let completion = scheduler.complete(1);
        // Request 1 is older than the newest request, but still newer than the
        // last displayed frame, so it may be shown while the latest renders.
        assert!(completion.apply);
        assert!(!completion.is_latest);
        assert_eq!(completion.next, Some((3, 12)));
        assert_eq!(scheduler.in_flight(), Some(3));
    }

    #[core::prelude::v1::test]
    fn a_late_result_never_overwrites_a_newer_frame() {
        let mut scheduler = PreviewScheduler::new();
        assert_eq!(scheduler.request(10), Schedule::Start(1, 10));
        assert_eq!(scheduler.request(11), Schedule::Coalesced);

        let first = scheduler.complete(1);
        assert!(first.apply);
        let next = first.next.unwrap();
        assert_eq!(next.0, 2);

        // An out-of-order completion for an already-applied id must be rejected.
        let stale = scheduler.complete(1);
        assert!(!stale.apply);
    }

    #[core::prelude::v1::test]
    fn latest_result_applies_and_leaves_scheduler_idle() {
        let mut scheduler = PreviewScheduler::new();
        assert_eq!(scheduler.request(10), Schedule::Start(1, 10));
        let completion = scheduler.complete(1);
        assert!(completion.apply);
        assert!(completion.is_latest);
        assert_eq!(completion.next, None);
        assert_eq!(scheduler.in_flight(), None);
    }

    #[core::prelude::v1::test]
    fn new_request_after_idle_starts_a_fresh_render() {
        let mut scheduler = PreviewScheduler::new();
        scheduler.request(10);
        scheduler.complete(1);
        assert_eq!(scheduler.request(20), Schedule::Start(2, 20));
        assert!(scheduler.is_current(2));
        assert!(!scheduler.is_current(1));
    }

    #[core::prelude::v1::test]
    fn long_scrub_burst_renders_only_first_and_last_frames() {
        let mut scheduler = PreviewScheduler::new();
        let mut rendered = Vec::new();

        let mut current = match scheduler.request(0) {
            Schedule::Start(id, frame) => (id, frame),
            Schedule::Coalesced => unreachable!(),
        };

        for frame in 1..50 {
            if let Schedule::Start(id, payload) = scheduler.request(frame) {
                current = (id, payload);
            }
        }

        loop {
            rendered.push(current.1);
            let completion = scheduler.complete(current.0);
            match completion.next {
                Some(next) => current = next,
                None => break,
            }
        }

        assert_eq!(rendered, vec![0, 49]);
        assert_eq!(scheduler.in_flight(), None);
    }
}
