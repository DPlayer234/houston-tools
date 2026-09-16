use log::Record;
use log4rs::config::Deserialize;
use log4rs::filter::{Filter, Response};

/// A filter that allows filtering an appender by targets.
///
/// Also allows forcefully accepting targets, overriding later filters.
/// This is useful to always log certain targets even when an appender is
/// filtered via a level [`threshold`].
///
/// [`threshold`]: log4rs::filter::threshold
#[derive(Debug)]
pub struct TargetFilter {
    config: TargetFilterConfig,
}

fn is_target_match(target: &str, filter: &str) -> bool {
    target
        .strip_prefix(filter)
        .is_some_and(|s| s.is_empty() || s.starts_with("::"))
}

impl Filter for TargetFilter {
    fn filter(&self, record: &Record<'_>) -> Response {
        let is_match = is_target_match(record.target(), &self.config.target);

        let is_match = u8::from(is_match) * MATCH;
        let mode = self.config.mode as u8;

        // this relies on the bit-pattern assigned to `TargetMode`.
        // the matched-on values are equal to the mismatch variants' values and we toggle the
        // `MATCH` bit in `mode`, so
        // - on a match: toggles the `MATCH` bit; so if `mode` is a match variant, this unsets the
        //   `MATCH` bit, resulting in `ACCEPT`/`REJECT`. otherwise, this results in an unchecked
        //   value and the fallback branch is taken.
        // - on a mismatch: the value isn't changed; so if `mode` is a mismatch variant, it already
        //   is `ACCEPT`/`REJECT`. otherwise, the value leads to the fallback branch.
        match mode ^ is_match {
            ACCEPT => Response::Accept,
            REJECT => Response::Reject,
            _ => Response::Neutral,
        }
    }
}

// `ACCEPT` as 0 and `REJECT` as 2 are chosen specifically to match the `Response` enum
// this isn't relevant for the logic to work, it just optimizes _slightly_ nicer
const REJECT: u8 = 2;
const ACCEPT: u8 = 0;
const MATCH: u8 = 1;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
enum TargetMode {
    RejectMismatch = REJECT,
    AcceptMismatch = ACCEPT,
    RejectMatch = REJECT | MATCH,
    AcceptMatch = ACCEPT | MATCH,
}

#[derive(Debug, serde::Deserialize)]
pub struct TargetFilterConfig {
    target: String,
    mode: TargetMode,
}

pub struct TargetFilterDeserializer;

impl Deserialize for TargetFilterDeserializer {
    type Trait = dyn Filter;
    type Config = TargetFilterConfig;

    fn deserialize(
        &self,
        config: Self::Config,
        _deserializers: &log4rs::config::Deserializers,
    ) -> anyhow::Result<Box<Self::Trait>> {
        Ok(Box::new(TargetFilter { config }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(target: &str) -> Record<'_> {
        Record::builder().target(target).build()
    }

    fn check_case(mode: TargetMode, on_match: Response, on_mismatch: Response) {
        let case = (mode, &on_match, &on_mismatch);
        let f = TargetFilter {
            config: TargetFilterConfig {
                target: "my_app".to_owned(),
                mode,
            },
        };

        assert_eq!(
            f.filter(&record("my_app")),
            on_match,
            "`my_app` in {case:?}"
        );
        assert_eq!(
            f.filter(&record("my_app::util")),
            on_match,
            "`my_app::util` in {case:?}"
        );
        assert_eq!(
            f.filter(&record("my_app::")),
            on_match,
            "`my_app::` in {case:?}"
        );
        assert_eq!(
            f.filter(&record("my_app_nope")),
            on_mismatch,
            "`my_app_nope` in {case:?}"
        );
        assert_eq!(
            f.filter(&record("some_lib")),
            on_mismatch,
            "`some_lib` in {case:?}"
        );
        assert_eq!(
            f.filter(&record("some_lib::")),
            on_mismatch,
            "`some_lib::` in {case:?}"
        );
        assert_eq!(
            f.filter(&record("some_lib::util")),
            on_mismatch,
            "`some_lib::util` in {case:?}"
        );
    }

    #[test]
    fn check_match_responses() {
        check_case(
            TargetMode::RejectMismatch,
            Response::Neutral,
            Response::Reject,
        );
        check_case(
            TargetMode::AcceptMismatch,
            Response::Neutral,
            Response::Accept,
        );
        check_case(TargetMode::RejectMatch, Response::Reject, Response::Neutral);
        check_case(TargetMode::AcceptMatch, Response::Accept, Response::Neutral);
    }
}
