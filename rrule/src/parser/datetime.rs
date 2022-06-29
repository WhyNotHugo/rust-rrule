use std::str::FromStr;

use chrono::{NaiveDate, TimeZone, Weekday};
use chrono_tz::{Tz, UTC};

use super::{
    regex::{self, ParsedDateString, ParsedStartDatetime},
    ParseError,
};
use crate::{core::DateTime, NWeekday};

/// Attempts to convert a `str` to a `chrono_tz::Tz`.
pub(crate) fn parse_timezone(tz: &str) -> Result<Tz, ParseError> {
    Tz::from_str(tz).map_err(|_err| ParseError::InvalidTimezone(tz.into()))
}

/// Convert a datetime string and a timezone to a `chrono::DateTime<Tz>`.
/// If the string specifies a zulu timezone with `Z`, then the timezone
/// argument will be ignored.
///
/// # Usage
///
/// ```
/// use rrule_parser::datetime::datestring_to_date;
/// use chrono_tz::{UTC, US};
/// use chrono::prelude::*;
///
/// // Zulu timezone
/// let dt = datestring_to_date("19970902T090000Z", &None, "DTSTART").unwrap();
/// assert_eq!(dt, UTC.ymd(1997, 9, 2).and_hms(9, 0, 0));
///
/// // Timezone via argument
/// let dt = datestring_to_date("19970902T090000", &Some(US::Pacific), "DTSTART").unwrap();
/// assert_eq!(dt, US::Pacific.ymd(1997, 9, 2).and_hms(9, 0, 0));
/// ```
pub(crate) fn datestring_to_date(
    dt: &str,
    tz: Option<Tz>,
    field: &str,
) -> Result<DateTime, ParseError> {
    let ParsedDateString {
        year,
        month,
        day,
        time,
        flags,
    } = regex::parse_datestring(dt).map_err(|_| ParseError::InvalidDateTime {
        value: dt.into(),
        field: field.into(),
    })?;

    // Combine parts to create data time.
    let date =
        NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| ParseError::InvalidDateTime {
            value: dt.into(),
            field: field.into(),
        })?;

    // Spec defines this is a date-time OR date
    // So the time can will be set to 0:0:0 if only a date is given.
    // https://icalendar.org/iCalendar-RFC-5545/3-8-2-4-date-time-start.html
    let (hour, min, sec) = if let Some(time) = time {
        (time.hour, time.min, time.sec)
    } else {
        (0, 0, 0)
    };
    let datetime = date
        .and_hms_opt(hour, min, sec)
        .ok_or_else(|| ParseError::InvalidDateTime {
            value: dt.into(),
            field: field.into(),
        })?;

    // Apply timezone appended to the datetime before converting to UTC.
    // For more info https://icalendar.org/iCalendar-RFC-5545/3-3-5-date-time.html
    let datetime: chrono::DateTime<chrono::Utc> = if flags.zulu_timezone_set {
        // If a `Z` is present, UTC should be used.
        chrono::DateTime::<_>::from_utc(datetime, chrono::Utc)
    } else {
        // If no `Z` is present, local time should be used.
        use chrono::offset::LocalResult;
        // Get datetime in local time or machine local time.
        // So this also takes into account daylight or standard time (summer/winter).
        match tz {
            Some(tz) => {
                // Use the timezone specified in the `tz`
                match tz.from_local_datetime(&datetime) {
                    LocalResult::None => Err(ParseError::InvalidDateTimeInLocalTimezone {
                        value: dt.into(),
                        field: field.into(),
                    }),
                    LocalResult::Single(date) => Ok(date),
                    LocalResult::Ambiguous(date1, date2) => {
                        Err(ParseError::DateTimeInLocalTimezoneIsAmbiguous {
                            value: dt.into(),
                            field: field.into(),
                            date1: date1.to_rfc3339(),
                            date2: date2.to_rfc3339(),
                        })
                    }
                }?
                .with_timezone(&chrono::Utc)
            }
            None => {
                // Use current system timezone
                // TODO Add option to always use UTC when this is executed on a server.
                let local = chrono::Local;
                match local.from_local_datetime(&datetime) {
                    LocalResult::None => Err(ParseError::InvalidDateTimeInLocalTimezone {
                        value: dt.into(),
                        field: field.into(),
                    }),
                    LocalResult::Single(date) => Ok(date),
                    LocalResult::Ambiguous(date1, date2) => {
                        Err(ParseError::DateTimeInLocalTimezoneIsAmbiguous {
                            value: dt.into(),
                            field: field.into(),
                            date1: date1.to_rfc3339(),
                            date2: date2.to_rfc3339(),
                        })
                    }
                }?
                .with_timezone(&chrono::Utc)
            }
        }
    };

    // Apply timezone from `TZID=` part (if any), else set datetime as UTC
    let datetime_with_timezone = datetime.with_timezone(&tz.unwrap_or(UTC));

    Ok(datetime_with_timezone)
}

/// Attempts to parse the DTSTART value from a `&str`.
pub(crate) fn parse_dtstart(s: &str) -> Result<DateTime, ParseError> {
    let ParsedStartDatetime { timezone, datetime } =
        regex::parse_start_datetime(s).map_err(|_| ParseError::InvalidDateTime {
            value: s.into(),
            field: "DTSTART".into(),
        })?;

    let tz = timezone.map(|tz| parse_timezone(&tz)).transpose()?;

    datestring_to_date(&datetime, tz, "DTSTART")
}

/// Attempts to convert a `str` to a `Weekday`.
pub(crate) fn str_to_weekday(d: &str) -> Result<Weekday, ParseError> {
    let day = match &d.to_uppercase()[..] {
        "MO" => Weekday::Mon,
        "TU" => Weekday::Tue,
        "WE" => Weekday::Wed,
        "TH" => Weekday::Thu,
        "FR" => Weekday::Fri,
        "SA" => Weekday::Sat,
        "SU" => Weekday::Sun,
        _ => return Err(ParseError::InvalidWeekday(d.to_string())),
    };
    Ok(day)
}

/// Parse the "BYWEEKDAY" and "BYDAY" values
/// Example: `SU,MO,TU,WE,TH,FR` or `4MO` or `-1WE`
/// > For example, within a MONTHLY rule, +1MO (or simply 1MO) represents the first Monday
/// > within the month, whereas -1MO represents the last Monday of the month.
pub(crate) fn parse_weekdays(val: &str) -> Result<Vec<NWeekday>, ParseError> {
    let mut wdays = vec![];
    // Separate all days
    for day in val.split(',') {
        let wday = day.parse::<NWeekday>()?;
        wdays.push(wday);
    }
    Ok(wdays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{America::New_York, US, UTC};

    #[test]
    fn parses_valid_nweekdays() {
        let tests = [
            ("SU", vec![NWeekday::Every(Weekday::Sun)]),
            ("-12TU", vec![NWeekday::Nth(-12, Weekday::Tue)]),
            (
                "MO,WE",
                vec![NWeekday::Every(Weekday::Mon), NWeekday::Every(Weekday::Wed)],
            ),
            (
                "MO,WE,3TU,-4SA",
                vec![
                    NWeekday::Every(Weekday::Mon),
                    NWeekday::Every(Weekday::Wed),
                    NWeekday::Nth(3, Weekday::Tue),
                    NWeekday::Nth(-4, Weekday::Sat),
                ],
            ),
        ];

        for (input, expected_output) in tests {
            let output = parse_weekdays(input);
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_nweekdays() {
        let tests = ["", "    ", "fjoasfjapsjop", "MONDAY", "MONDAY, TUESDAY"];

        for input in tests {
            let res = parse_weekdays(input);
            assert!(res.is_err());
        }
    }

    #[test]
    fn parses_valid_dtstart_lines() {
        let tests = [
            (
                "DTSTART;TZID=America/New_York:19970902T090000\nRRULE:FREQ=DAILY;",
                New_York.ymd(1997, 9, 2).and_hms(9, 0, 0),
            ),
            (
                "DTSTART:19970902T090000Z;",
                UTC.ymd(1997, 9, 2).and_hms(9, 0, 0),
            ),
        ];

        for (input, expected_output) in tests {
            let output = parse_dtstart(input);
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_dtstart_lines() {
        let tests = [
            "",
            "TZID=America/New_York:19970902T090000",
            "19970902T090000Z",
            "DTSTAR;TZID=America/New_York:19970902T09",
        ];

        for input in tests {
            let res = parse_weekdays(input);
            assert!(res.is_err());
        }
    }

    #[test]
    fn parses_valid_weekdays() {
        let tests = [
            ("MO", Weekday::Mon),
            ("TU", Weekday::Tue),
            ("WE", Weekday::Wed),
            ("TH", Weekday::Thu),
            ("FR", Weekday::Fri),
            ("SA", Weekday::Sat),
            ("SU", Weekday::Sun),
        ];

        for (input, expected_output) in tests {
            let output = str_to_weekday(input);
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_weekdays() {
        let tests = ["", "    ", "fjoasfjapsjop", "MONDAY", "MONDAY, TUESDAY"];

        for input in tests {
            let res = str_to_weekday(input);
            assert!(res.is_err());
        }
    }

    #[test]
    fn parses_valid_datestime_str() {
        let tests = [
            (
                "19970902T090000Z",
                None,
                UTC.ymd(1997, 9, 2).and_hms(9, 0, 0),
            ),
            (
                "19970902T090000",
                Some(UTC),
                UTC.ymd(1997, 9, 2).and_hms(9, 0, 0),
            ),
            (
                "19970902T090000",
                Some(US::Pacific),
                US::Pacific.ymd(1997, 9, 2).and_hms(9, 0, 0),
            ),
            (
                "19970902T090000Z",
                Some(US::Pacific),
                // Timezone is overwritten by the zulu specified in the datetime string
                UTC.ymd(1997, 9, 2).and_hms(9, 0, 0),
            ),
        ];

        for (datetime_str, timezone, expected_output) in tests {
            let output = datestring_to_date(datetime_str, timezone, "DTSTART");
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_datetime_str() {
        let tests = [
            ("", None),
            ("TZID=America/New_York:19970902T090000", None),
            ("19970902T09", None),
            ("19970902T09", Some(US::Pacific)),
        ];

        for (datetime_str, timezone) in tests {
            let res = datestring_to_date(datetime_str, timezone, "DTSTART");
            assert!(res.is_err());
        }
    }
}
