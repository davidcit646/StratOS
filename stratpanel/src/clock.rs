use std::time::{SystemTime, UNIX_EPOCH, Instant};

pub struct Clock {
    last_tick: Instant,
    cached_string: String,
}

impl Clock {
    pub fn new() -> Self {
        Clock {
            last_tick: Instant::now(),
            cached_string: String::new(),
        }
    }

    pub fn tick(&mut self, format: &str, show_date: bool) {
        let now = Instant::now();
        if now.duration_since(self.last_tick).as_secs() < 1 {
            return;
        }
        self.last_tick = now;

        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let (hour, minute, _second, year, month, day, weekday) = self.compute_time(secs);

        let time_str = if format == "12hr" {
            let hour_12 = if hour % 12 == 0 { 12 } else { hour % 12 };
            let ampm = if hour < 12 { "AM" } else { "PM" };
            format!("{:02}:{:02} {}", hour_12, minute, ampm)
        } else {
            format!("{:02}:{:02}", hour, minute)
        };

        if show_date {
            let month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
            let weekday_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
            self.cached_string = format!("{} {} {} {} {}", weekday_names[weekday], month_names[(month - 1) as usize], day, year, time_str);
        } else {
            self.cached_string = time_str;
        }
    }

    pub fn text(&self) -> &str {
        &self.cached_string
    }

    fn compute_time(&self, secs: u64) -> (u32, u32, u32, u32, u32, u32, usize) {
        const SECONDS_PER_DAY: u64 = 86400;
        const SECONDS_PER_HOUR: u64 = 3600;
        const SECONDS_PER_MINUTE: u64 = 60;

        let days = secs / SECONDS_PER_DAY;
        let remaining_secs = secs % SECONDS_PER_DAY;
        let hour = (remaining_secs / SECONDS_PER_HOUR) as u32;
        let remaining_secs = remaining_secs % SECONDS_PER_HOUR;
        let minute = (remaining_secs / SECONDS_PER_MINUTE) as u32;
        let second = (remaining_secs % SECONDS_PER_MINUTE) as u32;

        let (year, month, day, weekday) = self.compute_date(days);

        (hour, minute, second, year, month, day, weekday)
    }

    fn compute_date(&self, days: u64) -> (u32, u32, u32, usize) {
        const UNIX_EPOCH_DAY: i64 = 719528;
        let day_number = days as i64 + UNIX_EPOCH_DAY;

        let (year, month, day) = self.gregorian_date(day_number);
        let weekday = ((day_number % 7 + 6) % 7) as usize;

        (year, month, day, weekday)
    }

    fn gregorian_date(&self, day_number: i64) -> (u32, u32, u32) {
        let a = day_number + 32044;
        let b = (4 * a + 3) / 146097;
        let c = a - (146097 * b) / 4;
        let d = (4 * c + 3) / 1461;
        let e = c - (1461 * d) / 4;
        let m = (5 * e + 2) / 153;

        let day = (e - (153 * m + 2) / 5 + 1) as u32;
        let month = (m + 3 - 12 * (m / 10)) as u32;
        let year = (100 * b + d - 4800 + m / 10) as u32;

        (year, month, day)
    }
}
