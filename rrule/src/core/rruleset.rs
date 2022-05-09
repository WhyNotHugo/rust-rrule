use super::{datetime::DateTime, rrule::RRule};
use crate::{parser::build_rruleset, DateFilter, RRuleError, RRuleSetIter};
use chrono::TimeZone;
use chrono_tz::UTC;
use std::str::FromStr;
use crate::core::{Validated};

#[derive(Debug, Clone)]
pub struct RRuleSet<Stage = Validated> {
    pub rrule: Vec<RRule<Stage>>,
    pub rdate: Vec<DateTime>,
    pub exrule: Vec<RRule<Stage>>,
    pub exdate: Vec<DateTime>,
    pub dt_start: DateTime,
}

impl<S> Default for RRuleSet<S> {
    fn default() -> Self {
        Self {
            rrule: vec![],
            rdate: vec![],
            exrule: vec![],
            exdate: vec![],
            dt_start: UTC.ymd(1970, 1, 1).and_hms(0, 0, 0), // Unix Epoch
        }
    }
}

impl<S> RRuleSet<S> {
    pub fn rrule(&mut self, rrule: RRule<S>) {
        self.rrule.push(rrule);
    }

    pub fn exrule(&mut self, rrule: RRule<S>) {
        self.exrule.push(rrule);
    }

    pub fn rdate(&mut self, rdate: DateTime) {
        self.rdate.push(rdate);
    }

    pub fn exdate(&mut self, exdate: DateTime) {
        self.exdate.push(exdate);
    }
}

impl FromStr for RRuleSet {
    type Err = RRuleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        build_rruleset(s)
    }
}

impl<'a> DateFilter<'a, RRuleSetIter<'a>> for RRuleSet {}
