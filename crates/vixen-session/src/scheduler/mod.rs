//! The scheduler (`VixenModules.App.SuperScheduler`): its data in `SystemData/ModuleStore.xml`, the rules for when a schedule starts,
//! the texts the Schedules and Status windows show, and (in `run`) the executor that plays shows.

pub mod actions;
mod native_launch;
pub mod run;
pub mod sequences;

use uuid::Uuid;
use vixen_io::datetime::LocalDateTime;
use vixen_io::dcs::{format_bool, format_guid, parse_bool, parse_guid};
use vixen_io::{ModuleEntry, ModuleStore};
use vixen_xml::XElement;

/// `SuperSchedulerDescriptor`'s type id.
pub const MODULE_TYPE: Uuid = uuid::uuid!("985315e8-2b83-4d1a-89d1-58bfea7835a0");
const DATA_MODEL_TYPE: &str = "VixenModules.App.SuperScheduler.SuperSchedulerData, SuperScheduler";
const ASSEMBLY_NAME: &str = "Module.App.SuperScheduler";
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.SuperScheduler";
const DRAWING_NS: &str = "http://schemas.datacontract.org/2004/07/System.Drawing";

pub const HELP_URL: &str = "https://www.vixenlights.com/docs/usage/scheduler/";
/// The Schedules window's text, typo included.
pub const SHOWS_BLURB: &str = "Shows are sets of sequences, events and actions. You setup a show first and then schedule it to happen at a certain time below. You can setup multipe shows that happen at different times or on different days.";
pub const DELETE_TITLE: &str = "Delete Schedule";
pub const DELETE_MESSAGE: &str = "Are you sure you want to delete the selected schedule?";
pub const NO_SHOW_TITLE: &str = "Edit a Show";
pub const NO_SHOW_MESSAGE: &str = "The selected schedule does not have a show associated with it.";

// Calendar arithmetic on `LocalDateTime`, which is wall-clock time with no zone.

/// Days since 1970-01-01 in the proleptic Gregorian calendar.
fn day_number(year: i32, month: u8, day: u8) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    (year, month, day)
}

/// A calendar day.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// `default(DateTime)`, what a new schedule holds.
    pub const UNSET: Date = Date { year: 1, month: 1, day: 1 };

    pub fn of(t: &LocalDateTime) -> Date {
        Date { year: t.year, month: t.month, day: t.day }
    }

    pub fn midnight(&self) -> LocalDateTime {
        self.at(TimeOfDay::MIDNIGHT)
    }

    pub fn at(&self, time: TimeOfDay) -> LocalDateTime {
        LocalDateTime { year: self.year, month: self.month, day: self.day, hour: time.hour, minute: time.minute, second: time.second, millisecond: 0 }
    }

    pub fn plus_days(&self, days: i64) -> Date {
        let (year, month, day) = civil(day_number(self.year, self.month, self.day) + days);
        Date { year, month, day }
    }

    /// 0 is Sunday, as `DayOfWeek`.
    pub fn weekday(&self) -> usize {
        (day_number(self.year, self.month, self.day) + 4).rem_euclid(7) as usize
    }

    /// `ToShortDateString()`, en-US.
    pub fn short_text(&self) -> String {
        format!("{}/{}/{:04}", self.month, self.day, self.year)
    }

    /// What the date pickers show: a date before 2000, which a new schedule has, shows today.
    pub fn shown(&self, today: Date) -> Date {
        if self.year < 2000 { today } else { *self }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TimeOfDay {
    pub const MIDNIGHT: TimeOfDay = TimeOfDay { hour: 0, minute: 0, second: 0 };

    pub fn of(t: &LocalDateTime) -> TimeOfDay {
        TimeOfDay { hour: t.hour, minute: t.minute, second: t.second }
    }

    pub fn seconds(&self) -> i64 {
        i64::from(self.hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second)
    }

    /// `ToShortTimeString()`, en-US.
    pub fn short_text(&self) -> String {
        let hour = match self.hour % 12 {
            0 => 12,
            h => h,
        };
        format!("{}:{:02} {}", hour, self.minute, if self.hour < 12 { "AM" } else { "PM" })
    }
}

pub fn plus_days(t: LocalDateTime, days: i64) -> LocalDateTime {
    let date = Date::of(&t).plus_days(days);
    LocalDateTime { year: date.year, month: date.month, day: date.day, ..t }
}

pub fn plus_seconds(t: LocalDateTime, seconds: i64) -> LocalDateTime {
    let total = TimeOfDay::of(&t).seconds() + seconds;
    let date = Date::of(&t).plus_days(total.div_euclid(86_400));
    let rest = total.rem_euclid(86_400);
    LocalDateTime {
        year: date.year,
        month: date.month,
        day: date.day,
        hour: (rest / 3600) as u8,
        minute: (rest / 60 % 60) as u8,
        second: (rest % 60) as u8,
        millisecond: t.millisecond,
    }
}

/// Milliseconds from `earlier` to `later`.
pub fn millis_between(earlier: &LocalDateTime, later: &LocalDateTime) -> i64 {
    let stamp = |t: &LocalDateTime| (day_number(t.year, t.month, t.day) * 86_400 + TimeOfDay::of(t).seconds()) * 1000 + i64::from(t.millisecond);
    stamp(later) - stamp(earlier)
}

/// `ScheduleItem`'s stored members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleItem {
    /// Nil until a show is picked.
    pub show_id: Uuid,
    /// Sunday first, as `DayOfWeek`.
    pub days: [bool; 7],
    pub enabled: bool,
    pub start_date: Date,
    pub end_date: Date,
    /// Only the time of day is ever used; the file's date part is the day it was last written.
    pub start_time: TimeOfDay,
    pub end_time: TimeOfDay,
}

impl Default for ScheduleItem {
    /// `new ScheduleItem()`: every day, enabled, no show, and a window of the single instant 00:00:00, so it never starts as it is.
    fn default() -> ScheduleItem {
        ScheduleItem {
            show_id: Uuid::nil(),
            days: [true; 7],
            enabled: true,
            start_date: Date::UNSET,
            end_date: Date::UNSET,
            start_time: TimeOfDay::MIDNIGHT,
            end_time: TimeOfDay::MIDNIGHT,
        }
    }
}

impl ScheduleItem {
    /// `StartTime`'s getter: the stored time of day, today.
    pub fn start_time_today(&self, now: &LocalDateTime) -> LocalDateTime {
        Date::of(now).at(self.start_time)
    }

    /// `EndTime`'s getter: today, or tomorrow when it falls before the start.
    pub fn end_time_today(&self, now: &LocalDateTime) -> LocalDateTime {
        let today = Date::of(now);
        if self.end_time < self.start_time { today.plus_days(1).at(self.end_time) } else { today.at(self.end_time) }
    }

    /// `CheckForStartup`. KEEP: an overnight window can't be joined after midnight, and the weekday is the start day's.
    pub fn should_start(&self, now: &LocalDateTime) -> bool {
        let today = Date::of(now);
        self.enabled
            && today >= self.start_date
            && today <= self.end_date
            && *now >= self.start_time_today(now)
            && *now <= self.end_time_today(now)
            && self.days[today.weekday()]
    }

    /// `IsSchedulableInFuture`: the next ticked day counting today, at this time of day, if it lies strictly inside the dates. The end
    /// date is its own midnight, so the whole end day is outside.
    pub fn is_schedulable_in_future(&self, now: &LocalDateTime) -> Option<LocalDateTime> {
        let mut next = *now;
        let mut days = 0;
        while !self.days[Date::of(&next).weekday()] {
            next = plus_days(next, 1);
            days += 1;
            if days == 7 {
                return None;
            }
        }
        (self.start_date.midnight() < next && self.end_date.midnight() > next).then_some(next)
    }

    /// `NextStartDateTime`, for the status line only. KEEP: nothing once the time of day has passed the start time, though a later
    /// day would run.
    pub fn next_start(&self, now: &LocalDateTime) -> Option<LocalDateTime> {
        let next = self.is_schedulable_in_future(now)?;
        let time_of_day = TimeOfDay::of(&next).seconds() * 1000 + i64::from(next.millisecond);
        (self.start_time.seconds() * 1000 > time_of_day).then(|| Date::of(&next).at(self.start_time))
    }

    /// The `Schedule` column (F49).
    pub fn schedule_string(&self) -> String {
        const NAMES: [(usize, &str); 7] = [(1, "Mon"), (2, "Tue"), (3, "Wed"), (4, "Thu"), (5, "Fri"), (6, "Sat"), (0, "Sun")];
        let weekdays = self.days[1..6].iter().all(|d| *d);
        let any_weekday = self.days[1..6].iter().any(|d| *d);
        let days = if self.days.iter().all(|d| *d) {
            "Everyday".to_string()
        } else if weekdays && !self.days[0] && !self.days[6] {
            "Weekdays".to_string()
        } else if self.days[0] && self.days[6] && !any_weekday {
            "Weekends".to_string()
        } else if self.days.iter().any(|d| *d) {
            NAMES.iter().filter(|(day, _)| self.days[*day]).map(|(_, name)| *name).collect::<Vec<_>>().join(", ")
        } else {
            "Never".to_string()
        };
        format!(
            "{} to {} @ {} - {}, {}",
            self.start_date.short_text(),
            self.end_date.short_text(),
            self.start_time.short_text(),
            self.end_time.short_text(),
            days
        )
    }

    pub fn status_string(&self) -> &'static str {
        if self.enabled { "Enabled" } else { "Disabled" }
    }
}

/// The line under the time pickers (F53).
pub fn duration_text(start: TimeOfDay, end: TimeOfDay) -> String {
    let mut seconds = end.seconds() - start.seconds();
    if seconds < 0 {
        seconds += 86_400;
    }
    format!("(the show will last {} hours and {} minutes)", seconds / 3600, seconds / 60 % 60)
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScheduleEditError {
    #[error("The end date of a show must fall after the start date.")]
    Dates,
    #[error("You must select a Show to run.")]
    NoShow,
}

impl ScheduleEditError {
    pub fn title(&self) -> &'static str {
        match self {
            ScheduleEditError::Dates => "Date Error",
            ScheduleEditError::NoShow => "Schedule Error",
        }
    }
}

/// OK in the Schedule a Show dialog. FIX: checked before anything is written, so Cancel after an alert leaves the schedule alone.
pub fn validate_schedule_edit(start: Date, end: Date, show: Option<Uuid>) -> Result<(), ScheduleEditError> {
    if start > end {
        return Err(ScheduleEditError::Dates);
    }
    match show {
        Some(id) if !id.is_nil() => Ok(()),
        _ => Err(ScheduleEditError::NoShow),
    }
}

/// `SuperSchedulerData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerData {
    /// No UI for it; at least 1 when used.
    pub check_interval_seconds: i32,
    pub is_enabled: bool,
    pub items: Vec<ScheduleItem>,
    /// The Status window's top-left, in Windows' screen coordinates.
    pub status_position: (i32, i32),
}

impl Default for SchedulerData {
    fn default() -> SchedulerData {
        SchedulerData { check_interval_seconds: 1, is_enabled: false, items: Vec::new(), status_position: (0, 0) }
    }
}

impl SchedulerData {
    pub fn check_interval(&self) -> i64 {
        i64::from(self.check_interval_seconds.max(1))
    }
}

// The data contract.

fn bool_member(e: &XElement, name: &str, default: bool) -> bool {
    e.child_text(name).and_then(|t| parse_bool(&t)).unwrap_or(default)
}

fn time_member(e: &XElement, name: &str) -> Option<LocalDateTime> {
    e.child_text(name).and_then(|t| LocalDateTime::parse(&t))
}

fn decode_item(e: &XElement) -> ScheduleItem {
    // A data-contract load doesn't run the initializers, so a missing day or `Enabled` is false.
    let day = |name: &str| bool_member(e, name, false);
    ScheduleItem {
        show_id: e.child_text("ShowID").and_then(|t| parse_guid(&t)).unwrap_or_default(),
        days: [day("Sunday"), day("Monday"), day("Tuesday"), day("Wednesday"), day("Thursday"), day("Friday"), day("Saturday")],
        enabled: bool_member(e, "Enabled", false),
        start_date: time_member(e, "StartDate").map_or(Date::UNSET, |t| Date::of(&t)),
        end_date: time_member(e, "EndDate").map_or(Date::UNSET, |t| Date::of(&t)),
        start_time: time_member(e, "StartTime").map_or(TimeOfDay::MIDNIGHT, |t| TimeOfDay::of(&t)),
        end_time: time_member(e, "EndTime").map_or(TimeOfDay::MIDNIGHT, |t| TimeOfDay::of(&t)),
    }
}

fn xsd_date_time(t: &LocalDateTime) -> String {
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", t.year, t.month, t.day, t.hour, t.minute, t.second)
}

/// `today` is the day of the save: `StartTime` and `EndTime` are written through their getters, which put the time of day on it.
fn encode_item(item: &ScheduleItem, today: &LocalDateTime) -> XElement {
    let mut e = XElement::new(NS, "ScheduleItem");
    let flag = |e: &mut XElement, name: &str, value: bool| {
        e.push(XElement::with_text(NS, name, format_bool(value)));
    };
    let time = |e: &mut XElement, name: &str, value: LocalDateTime| {
        e.push(XElement::with_text(NS, name, xsd_date_time(&value)));
    };
    flag(&mut e, "Enabled", item.enabled);
    time(&mut e, "EndDate", item.end_date.midnight());
    time(&mut e, "EndTime", item.end_time_today(today));
    flag(&mut e, "Friday", item.days[5]);
    flag(&mut e, "Monday", item.days[1]);
    flag(&mut e, "Saturday", item.days[6]);
    e.push(XElement::with_text(NS, "ShowID", format_guid(&item.show_id)));
    time(&mut e, "StartDate", item.start_date.midnight());
    time(&mut e, "StartTime", item.start_time_today(today));
    flag(&mut e, "Sunday", item.days[0]);
    flag(&mut e, "Thursday", item.days[4]);
    flag(&mut e, "Tuesday", item.days[2]);
    flag(&mut e, "Wednesday", item.days[3]);
    e
}

fn decode_data(data: &XElement) -> SchedulerData {
    let int = |e: &XElement, name: &str| e.child_text(name).and_then(|t| t.trim().parse::<i32>().ok());
    SchedulerData {
        check_interval_seconds: int(data, "CheckIntervalInSeconds").unwrap_or(0),
        is_enabled: bool_member(data, "IsEnabled", false),
        items: data
            .child("Items")
            .map(|list| list.elements().filter(|e| e.local == "ScheduleItem").map(decode_item).collect())
            .unwrap_or_default(),
        status_position: data
            .child("StatusForm_Position")
            .map(|p| (int(p, "x").unwrap_or(0), int(p, "y").unwrap_or(0)))
            .unwrap_or((0, 0)),
    }
}

fn encode_data(data: &SchedulerData, today: &LocalDateTime) -> XElement {
    let mut e = XElement::new(NS, "SuperSchedulerData");
    e.push(XElement::with_text("", "ModuleInstanceId", format_guid(&MODULE_TYPE)));
    e.push(XElement::with_text("", "ModuleTypeId", format_guid(&MODULE_TYPE)));
    e.push(XElement::with_text(NS, "CheckIntervalInSeconds", data.check_interval_seconds.to_string()));
    e.push(XElement::with_text(NS, "IsEnabled", format_bool(data.is_enabled)));
    let mut items = XElement::new(NS, "Items");
    for item in &data.items {
        items.push(encode_item(item, today));
    }
    e.push(items);
    let mut position = XElement::new(NS, "StatusForm_Position");
    position.push(XElement::with_text(DRAWING_NS, "x", data.status_position.0.to_string()));
    position.push(XElement::with_text(DRAWING_NS, "y", data.status_position.1.to_string()));
    e.push(position);
    e
}

/// The stored data, or the defaults when the module has never been saved.
pub fn load_scheduler(store: &ModuleStore) -> SchedulerData {
    store.type_data(MODULE_TYPE).and_then(ModuleEntry::data).map(decode_data).unwrap_or_default()
}

/// Writes the data into the store. A list of schedules in which nothing changed keeps its stored text, so its times keep the date
/// they were last written on; once one changes, every time in it is written on the day of the save, as upstream writes them at every
/// save. Nothing reads that date.
pub fn save_scheduler(store: &mut ModuleStore, data: &SchedulerData, today: &LocalDateTime) {
    let new_plain = encode_data(data, today);
    match store.type_data_mut(MODULE_TYPE) {
        Some(entry) => {
            let old_plain = encode_data(&entry.data().map(decode_data).unwrap_or_default(), today);
            entry.merge_data(&old_plain, new_plain);
        }
        None => store.set_type_data(ModuleEntry::new(DATA_MODEL_TYPE, &MODULE_TYPE, &MODULE_TYPE, ASSEMBLY_NAME, new_plain)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const REAL_STORE: &str = "/Users/developer/Documents/Vixen 3/SystemData/ModuleStore.xml";

    pub(crate) fn at(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> LocalDateTime {
        LocalDateTime { year, month, day, hour, minute, second, millisecond: 0 }
    }

    fn time(hour: u8, minute: u8) -> TimeOfDay {
        TimeOfDay { hour, minute, second: 0 }
    }

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date { year, month, day }
    }

    fn owners_item() -> ScheduleItem {
        ScheduleItem {
            show_id: uuid::uuid!("e81726f8-9708-41df-b8de-f8c07b875f22"),
            days: [true; 7],
            enabled: true,
            start_date: date(2018, 12, 25),
            end_date: date(2019, 1, 1),
            start_time: time(19, 7),
            end_time: time(19, 11),
        }
    }

    #[test]
    fn calendar_arithmetic() {
        assert_eq!(date(1970, 1, 1).weekday(), 4);
        assert_eq!(date(2018, 12, 25).weekday(), 2);
        assert_eq!(date(2024, 2, 28).plus_days(2), date(2024, 3, 1));
        assert_eq!(date(2023, 2, 28).plus_days(1), date(2023, 3, 1));
        assert_eq!(date(2018, 12, 31).plus_days(1), date(2019, 1, 1));
        assert_eq!(date(1, 1, 1).plus_days(0), date(1, 1, 1));
        assert_eq!(plus_seconds(at(2018, 12, 31, 23, 59, 59), 2), at(2019, 1, 1, 0, 0, 1));
        assert_eq!(millis_between(&at(2018, 12, 31, 23, 59, 59), &at(2019, 1, 1, 0, 0, 1)), 2000);
        assert_eq!(date(1999, 12, 31).shown(date(2026, 11, 9)), date(2026, 11, 9));
        assert_eq!(date(2000, 1, 1).shown(date(2026, 11, 9)), date(2000, 1, 1));
    }

    #[test]
    fn the_owners_schedule_loads_and_saves_back_unchanged() {
        let Ok(store) = vixen_io::load_module_store_file(Path::new(REAL_STORE)) else { return };
        let data = load_scheduler(&store);
        assert_eq!(data, SchedulerData { check_interval_seconds: 1, is_enabled: false, items: vec![owners_item()], status_position: (90, 204) });

        let original = store.type_data(MODULE_TYPE).unwrap().data().unwrap().clone();
        let mut copy = store.clone();
        save_scheduler(&mut copy, &data, &at(2026, 11, 9, 8, 0, 0));
        let saved = copy.type_data(MODULE_TYPE).unwrap().data().unwrap();
        assert_eq!(saved.first_difference(&original, String::new()), None, "untouched, the times keep the day they were written on");

        // Once a schedule changes, the times are written on the day of the save.
        let mut changed = data.clone();
        changed.items[0].end_time = time(21, 30);
        changed.is_enabled = true;
        save_scheduler(&mut copy, &changed, &at(2026, 11, 9, 8, 0, 0));
        let saved = copy.type_data(MODULE_TYPE).unwrap().data().unwrap();
        let item = saved.child("Items").unwrap().child("ScheduleItem").unwrap();
        assert_eq!(item.child_text("EndTime").as_deref(), Some("2026-11-09T21:30:00"));
        assert_eq!(item.child_text("StartTime").as_deref(), Some("2026-11-09T19:07:00"));
        assert_eq!(load_scheduler(&copy), changed);
    }

    #[test]
    fn a_new_store_gets_the_data_contract_shape() {
        let mut data = SchedulerData::default();
        let mut overnight = owners_item();
        overnight.start_time = time(22, 0);
        overnight.end_time = time(2, 0);
        overnight.days = [false, true, true, true, true, true, false];
        data.items = vec![ScheduleItem::default(), overnight];
        data.status_position = (12, -40);
        let mut store = ModuleStore::new();
        save_scheduler(&mut store, &data, &at(2026, 11, 9, 8, 0, 0));
        let saved = store.type_data(MODULE_TYPE).unwrap().data().unwrap();
        let members: Vec<&str> = saved.elements().map(|e| e.local.as_str()).collect();
        assert_eq!(members, ["ModuleInstanceId", "ModuleTypeId", "CheckIntervalInSeconds", "IsEnabled", "Items", "StatusForm_Position"]);
        let items: Vec<&XElement> = saved.child("Items").unwrap().elements().collect();
        let members: Vec<&str> = items[0].elements().map(|e| e.local.as_str()).collect();
        assert_eq!(
            members,
            ["Enabled", "EndDate", "EndTime", "Friday", "Monday", "Saturday", "ShowID", "StartDate", "StartTime", "Sunday", "Thursday", "Tuesday", "Wednesday"]
        );
        assert_eq!(items[0].child_text("StartDate").as_deref(), Some("0001-01-01T00:00:00"));
        assert_eq!(items[0].child_text("StartTime").as_deref(), Some("2026-11-09T00:00:00"));
        // An overnight end is written on the day after.
        assert_eq!(items[1].child_text("EndTime").as_deref(), Some("2026-11-10T02:00:00"));
        assert_eq!(load_scheduler(&store), data);
    }

    #[test]
    fn a_schedule_starts_inside_its_dates_times_and_days() {
        let item = owners_item();
        assert!(item.should_start(&at(2018, 12, 25, 19, 7, 0)));
        assert!(item.should_start(&at(2019, 1, 1, 19, 11, 0)));
        assert!(!item.should_start(&at(2018, 12, 25, 19, 6, 59)));
        assert!(!item.should_start(&at(2018, 12, 25, 19, 11, 1)));
        assert!(!item.should_start(&at(2018, 12, 24, 19, 8, 0)));
        assert!(!item.should_start(&at(2019, 1, 2, 19, 8, 0)));
        let mut off = item.clone();
        off.enabled = false;
        assert!(!off.should_start(&at(2018, 12, 25, 19, 8, 0)));
        // 2018-12-25 was a Tuesday.
        let mut not_tuesday = item.clone();
        not_tuesday.days[2] = false;
        assert!(!not_tuesday.should_start(&at(2018, 12, 25, 19, 8, 0)));
        assert!(not_tuesday.should_start(&at(2018, 12, 26, 19, 8, 0)));
        // A new schedule's window is one instant on a day long gone.
        assert!(!ScheduleItem::default().should_start(&at(2026, 11, 9, 0, 0, 0)));
    }

    #[test]
    fn an_overnight_window_cant_be_joined_after_midnight() {
        let mut item = owners_item();
        item.start_time = time(22, 0);
        item.end_time = time(2, 0);
        assert_eq!(item.end_time_today(&at(2018, 12, 26, 23, 0, 0)), at(2018, 12, 27, 2, 0, 0));
        assert!(item.should_start(&at(2018, 12, 26, 23, 0, 0)));
        assert!(!item.should_start(&at(2018, 12, 27, 1, 0, 0)));
    }

    #[test]
    fn the_next_start_follows_the_originals_odd_rule() {
        let item = owners_item();
        // Before the start time on a day inside the dates.
        assert_eq!(item.next_start(&at(2018, 12, 26, 9, 0, 0)), Some(at(2018, 12, 26, 19, 7, 0)));
        // Past the start time: nothing, though tomorrow would run.
        assert_eq!(item.next_start(&at(2018, 12, 26, 19, 7, 0)), None);
        assert_eq!(item.next_start(&at(2018, 12, 26, 20, 0, 0)), None);
        // The start date must already be past; its own midnight isn't.
        assert_eq!(item.next_start(&at(2018, 12, 25, 0, 0, 0)), None);
        assert_eq!(item.next_start(&at(2018, 12, 25, 0, 0, 1)), Some(at(2018, 12, 25, 19, 7, 0)));
        assert_eq!(item.next_start(&at(2018, 12, 24, 9, 0, 0)), None);
        // All through the end day.
        assert_eq!(item.next_start(&at(2019, 1, 1, 9, 0, 0)), None);
        assert_eq!(item.next_start(&at(2018, 12, 31, 9, 0, 0)), Some(at(2018, 12, 31, 19, 7, 0)));

        // Only Saturdays: from Wednesday the 26th, the 29th.
        let mut saturdays = item.clone();
        saturdays.days = [false, false, false, false, false, false, true];
        assert_eq!(saturdays.next_start(&at(2018, 12, 26, 9, 0, 0)), Some(at(2018, 12, 29, 19, 7, 0)));
        assert_eq!(saturdays.is_schedulable_in_future(&at(2018, 12, 26, 20, 0, 0)), Some(at(2018, 12, 29, 20, 0, 0)));
        assert_eq!(saturdays.next_start(&at(2018, 12, 26, 20, 0, 0)), None);
        let mut never = item.clone();
        never.days = [false; 7];
        assert_eq!(never.is_schedulable_in_future(&at(2018, 12, 26, 9, 0, 0)), None);
    }

    #[test]
    fn the_schedule_column_reads_as_the_original() {
        let mut item = owners_item();
        assert_eq!(item.schedule_string(), "12/25/2018 to 1/1/2019 @ 7:07 PM - 7:11 PM, Everyday");
        item.days = [false, true, true, true, true, true, false];
        assert!(item.schedule_string().ends_with(", Weekdays"));
        item.days = [true, false, false, false, false, false, true];
        assert!(item.schedule_string().ends_with(", Weekends"));
        item.days = [true, true, false, true, false, false, false];
        assert!(item.schedule_string().ends_with("PM, Mon, Wed, Sun"));
        item.days = [false; 7];
        assert!(item.schedule_string().ends_with(", Never"));
        assert_eq!(ScheduleItem::default().schedule_string(), "1/1/0001 to 1/1/0001 @ 12:00 AM - 12:00 AM, Everyday");
        assert_eq!(TimeOfDay { hour: 12, minute: 5, second: 0 }.short_text(), "12:05 PM");
        assert_eq!(item.status_string(), "Enabled");
    }

    #[test]
    fn the_duration_line_and_the_dialogs_checks() {
        assert_eq!(duration_text(time(19, 7), time(19, 11)), "(the show will last 0 hours and 4 minutes)");
        assert_eq!(duration_text(time(22, 0), time(2, 30)), "(the show will last 4 hours and 30 minutes)");
        assert_eq!(duration_text(time(0, 0), time(0, 0)), "(the show will last 0 hours and 0 minutes)");

        let show = Some(Uuid::from_u128(1));
        assert_eq!(validate_schedule_edit(date(2026, 12, 1), date(2026, 12, 1), show), Ok(()));
        let dates = validate_schedule_edit(date(2026, 12, 2), date(2026, 12, 1), None).unwrap_err();
        assert_eq!((dates.title(), dates.to_string().as_str()), ("Date Error", "The end date of a show must fall after the start date."));
        let no_show = validate_schedule_edit(date(2026, 12, 1), date(2026, 12, 2), None).unwrap_err();
        assert_eq!((no_show.title(), no_show.to_string().as_str()), ("Schedule Error", "You must select a Show to run."));
        assert_eq!(validate_schedule_edit(date(2026, 12, 1), date(2026, 12, 2), Some(Uuid::nil())), Err(ScheduleEditError::NoShow));
    }
}
