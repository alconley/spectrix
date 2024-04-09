use super::compass_data::CompassData;

#[derive(Debug)]
pub struct EventBuilder {
    coincidence_window: f64,
    event: Vec<CompassData>,
    ready_event: Vec<CompassData>,
    is_event_ready: bool,
}

impl EventBuilder {
    pub fn new(window: &f64) -> EventBuilder {
        EventBuilder {
            coincidence_window: *window,
            event: vec![],
            ready_event: vec![],
            is_event_ready: false,
        }
    }

    pub fn is_event_ready(&self) -> bool {
        self.is_event_ready
    }

    pub fn push_hit(&mut self, hit: &CompassData) {
        if self.event.is_empty() {
            self.event.push(hit.clone());
            return;
        }

        if hit.timestamp - self.event[0].timestamp < self.coincidence_window {
            self.event.push(hit.clone());
        } else {
            self.ready_event = self.event.clone();
            self.is_event_ready = true;
            self.event.clear();
            self.event.push(hit.clone());
        }
    }

    pub fn get_ready_event(&mut self) -> Vec<CompassData> {
        self.is_event_ready = false;
        self.ready_event.clone()
    }
}
