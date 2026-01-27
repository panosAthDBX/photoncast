//! Calendar methods for [`LauncherWindow`].

use super::*;

impl LauncherWindow {
    /// Fetches the next upcoming meeting from the calendar.
    pub(super) fn fetch_next_meeting(&mut self, cx: &mut ViewContext<Self>) {
        tracing::debug!("fetch_next_meeting: starting");
        cx.spawn(|this, mut cx| async move {
            // Run calendar fetch in background
            let result = cx
                .background_executor()
                .spawn(async move {
                    let calendar = photoncast_calendar::CalendarCommand::with_default_config();
                    // Fetch events for the next 24 hours
                    calendar.fetch_upcoming_events(1)
                })
                .await;

            let _ = this.update(&mut cx, |this, cx| {
                match result {
                    Ok(events) => {
                        tracing::debug!("fetch_next_meeting: got {} events", events.len());
                        // Find the next event that hasn't ended yet
                        let now = photoncast_calendar::chrono::Local::now();
                        this.meeting.next_meeting = events.into_iter().find(|e| e.end > now);
                        if this.meeting.next_meeting.is_some() {
                            tracing::debug!(
                                "Next meeting found: {:?}",
                                this.meeting.next_meeting.as_ref().map(|m| &m.title)
                            );
                            // Select meeting by default when query is empty
                            if this.search.query.is_empty() {
                                this.meeting.selected = true;
                            }
                        } else {
                            tracing::debug!("fetch_next_meeting: no upcoming meeting found");
                            this.meeting.selected = false;
                        }
                    },
                    Err(e) => {
                        tracing::debug!("Could not fetch next meeting: {}", e);
                        this.meeting.next_meeting = None;
                        this.meeting.selected = false;
                    },
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn show_calendar(
        &mut self,
        title: String,
        events: Vec<photoncast_calendar::CalendarEvent>,
        cx: &mut ViewContext<Self>,
    ) {
        tracing::info!("Entering Calendar Mode with {} events", events.len());
        // Store all events for filtering
        self.meeting.all_events = events.clone();
        self.search.mode = SearchMode::Calendar {
            title,
            events,
            error: None,
        };
        self.reset_query();
        self.search.results.clear();
        self.search.base_results.clear();
        self.search.core_results.clear();
        self.search.selected_index = 0;
        self.file_search.loading = false;
        self.file_search.pending_query = None;
        self.calculator.result = None;
        self.calculator.generation = self.calculator.generation.saturating_add(1);
        cx.notify();
    }

    pub(crate) fn show_calendar_error(
        &mut self,
        title: String,
        error: String,
        cx: &mut ViewContext<Self>,
    ) {
        tracing::info!("Entering Calendar Mode with error");
        self.search.mode = SearchMode::Calendar {
            title,
            events: Vec::new(),
            error: Some(error),
        };
        self.reset_query();
        self.search.results.clear();
        self.search.base_results.clear();
        self.search.core_results.clear();
        self.search.selected_index = 0;
        self.file_search.loading = false;
        self.file_search.pending_query = None;
        self.calculator.result = None;
        self.calculator.generation = self.calculator.generation.saturating_add(1);
        cx.notify();
    }

    pub(super) fn exit_calendar_mode(&mut self, cx: &mut ViewContext<Self>) {
        tracing::info!("Exiting Calendar Mode");
        self.search.mode = SearchMode::Normal;
        self.reset_query();
        self.search.results.clear();
        self.search.base_results.clear();
        self.search.core_results.clear();
        self.meeting.all_events.clear();
        self.search.selected_index = 0;
        self.file_search.loading = false;
        self.file_search.pending_query = None;
        self.file_search.generation += 1;
        self.calculator.result = None;
        self.calculator.generation = self.calculator.generation.saturating_add(1);
        // Reload suggestions for empty state
        self.load_suggestions(cx);
        cx.notify();
    }
}
