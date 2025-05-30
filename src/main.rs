mod forecast;
use chrono::DateTime;
use chrono::Utc;
use forecast::CarbonIntensityAverageEstimate;

struct AverageEstimate {
    now: CarbonIntensityAverageEstimate,
    min: Option<CarbonIntensityAverageEstimate>,
}

fn get_average_estimate(
    data: Vec<forecast::CarbonIntensityPointEstimate>,
    duration: i64,
    start: DateTime<Utc>,
) -> AverageEstimate {
    let wf = forecast::WindowedForecast::new(data, duration, start);

    AverageEstimate {
        now: wf.index(0),
        min: wf.min(),
    }
}

fn main() {}

#[cfg(test)]
mod test {

    use core::f64;

    use chrono::prelude::*;
    use chrono::{DateTime, NaiveDate, Utc};
    use csv::ReaderBuilder;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    use super::*;

    // struct this better
    fn make_test_data() -> Vec<forecast::CarbonIntensityPointEstimate> {
        let naive_datetime = NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let d: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
        let ndata = 200;
        let step = f64::consts::PI / (ndata as f64);

        (0..ndata)
            .map(|i| forecast::CarbonIntensityPointEstimate {
                datetime: d + chrono::Duration::minutes(i),
                value: -1.0 * f64::sin(i as f64 * step),
            })
            .collect::<Vec<forecast::CarbonIntensityPointEstimate>>()
    }

    fn read_sample_data(
    ) -> Result<Vec<forecast::CarbonIntensityPointEstimate>, Box<dyn std::error::Error>> {
        let file_path = Path::new("tests/carbon_intensity_24h.csv");
        let file = File::open(file_path)?;
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(BufReader::new(file));

        let mut data = Vec::new();

        for result in rdr.records() {
            let record = result?;
            let datestr = &record[0];
            let intensity_value = &record[3];

            // Convert "2025-05-28T14:00:00Z" to RFC3339 (add +00:00 manually if needed)
            let datetime_str = format!("{}:00+00:00", datestr.trim_end_matches('Z'));
            let datetime = DateTime::parse_from_rfc3339(&datetime_str)?.with_timezone(&Utc);

            let value: f64 = intensity_value.parse()?;

            data.push(forecast::CarbonIntensityPointEstimate { value, datetime });
        }

        Ok(data)
    }

    #[test]
    fn test_sample_data_length() {
        let data = read_sample_data().unwrap();
        assert_eq!(data.len(), 96);
    }

    #[test]
    fn test_across_all_durations_non_half_hour() {
        // test across all supported durations
        let data = read_sample_data().unwrap();
        let start = Utc.with_ymd_and_hms(2023, 5, 4, 12, 50, 0).unwrap();
        for duration in 1..2821 {
            let estimate = get_average_estimate(data.clone(), duration, start);
            // Check that carbon intensity is actually lower in the minimum
            assert!(estimate.min.unwrap().value <= estimate.now.value);
        }
    }

    #[test]
    fn test_across_all_durations_at_half_hour() {
        // tests showing minimisation for an extra window as start time is at exactly half hour
        let data = read_sample_data().unwrap();
        let start = Utc.with_ymd_and_hms(2023, 5, 4, 12, 30, 0).unwrap();
        for duration in 1..2851 {
            let estimate = get_average_estimate(data.clone(), duration, start);
            // Check that carbon intensity is actually lower in the minimum
            assert!(estimate.min.unwrap().value <= estimate.now.value);
        }
    }

    #[test]
    fn test_across_durations_expect_failure_non_half_hour() {
        // Expected failures as we do not find a valid window beyond the last data point
        let data = read_sample_data().unwrap();
        let start = Utc.with_ymd_and_hms(2023, 5, 4, 12, 50, 0).unwrap();
        for duration in 2831..2851 {
            let estimate = get_average_estimate(data.clone(), duration, start);
            assert!(estimate.min.is_none());
        }
    }

    #[test]
    fn test_has_right_length() {
        let data = make_test_data();
        let window_size = 160;

        let start = data.first().unwrap().datetime;
        let wf = forecast::WindowedForecast::new(data, window_size, start);

        assert_eq!(wf.len(), 40);
    }

    #[test]
    fn test_values() {
        let data = make_test_data();
        let window_size = 160;

        let start = data.first().unwrap().datetime;
        let wf = forecast::WindowedForecast::new(data, window_size, start);

        let ndata = 200;
        let step = f64::consts::PI / (ndata as f64);

        fn compute(i: usize, window_size: i64, step: f64) -> f64 {
            ((i as i64 + window_size) as f64 * step).cos() - (i as f64 * step).cos()
        }
        let expected: Vec<f64> = (0..=40).map(|i| compute(i, window_size, step)).collect();

        let expected: Vec<f64> = expected
            .iter()
            .map(|e| e / (window_size as f64 * step))
            .collect();
        let actual: Vec<CarbonIntensityAverageEstimate> = wf.into_iter().collect();

        for (e, a) in expected.iter().zip(actual.iter()) {
            let atol = 1e-8;
            let rtel = 0.01;
            let tol = atol + rtel * a.value.abs();
            let diff = (e - a.value).abs();
            let close = diff <= tol;
            assert!(close);
        }
    }
}
