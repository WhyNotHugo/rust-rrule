//! # Manual RRuleSet
//!
//! Create an `RRuleSet` object manually.

use chrono::{Datelike, TimeZone};
use chrono_tz::UTC;
use rrule::{DateFilter, Frequency, NWeekday, RRuleProperties, RRuleSet, Weekday};

/// ## Construct `RRuleSet` from one `rrule` and `exrule`
/// The rrule will occur weekly on Tuesday and Wednesday and the exrule
/// will occur weekly on Wednesday, and therefore the end result will contain
/// weekly recurrences on Wednesday only.
fn main() {
    // Build properties for rrule that occurs weekly on Tuesday and Wednesday
    let rrule_properties = RRuleProperties::default()
        .count(4)
        .freq(Frequency::Weekly)
        .by_weekday(vec![
            NWeekday::Every(Weekday::Tue),
            NWeekday::Every(Weekday::Wed),
        ]);

    // Construct `RRule` from properties
    let rrule = rrule_properties
        .build(UTC.ymd(2020, 1, 1).and_hms(9, 0, 0))
        .expect("RRule invalid");

    // Build properties for exrule that occurs weekly on Wednesday
    let exrule_properties = RRuleProperties::default()
        .count(4)
        .freq(Frequency::Weekly)
        .by_weekday(vec![NWeekday::Every(Weekday::Wed)]);

    // Construct `RRule` from properties
    let exrule = exrule_properties
        .build(UTC.ymd(2020, 1, 1).and_hms(9, 0, 0))
        .expect("RRule invalid");

    // Now create the RRuleSet and add rrule and exrule
    let mut rrule_set = RRuleSet::default();
    rrule_set.rrule(rrule);
    rrule_set.exrule(exrule);

    let recurrences = rrule_set.all(100).unwrap();

    // Check that all the recurrences are on a Tuesday
    for occurrence in &recurrences {
        assert_eq!(occurrence.weekday(), Weekday::Tue);
    }

    assert_eq!(recurrences.len(), 2);
    println!("Done, everything worked.");
}
