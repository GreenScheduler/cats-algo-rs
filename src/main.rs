mod forecast;
use forecast::CarbonIntensityAverageEstimate;
use chrono::Utc;


struct AverageEstimate {
    now: CarbonIntensityAverageEstimate,
    min: CarbonIntensityAverageEstimate 
}

fn get_average_estimate(data: Vec<forecast::CarbonIntensityPointEstimate>, duration: i64) -> AverageEstimate {

    let wf = forecast::WindowedForecast::new(data, duration, Utc::now());

    AverageEstimate {
        now: wf.index(0),
        min: wf.min().unwrap(),
    }

}


fn main() {

}


#[cfg(test)]
mod test {

    use core::f64;

    use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

    use super::*;


    // struct this better
    fn make_test_data () -> Vec<forecast::CarbonIntensityPointEstimate> {
        
       let naive_date = NaiveDate::from_ymd(2023, 1, 1);
        let naive_time = NaiveTime::from_hms(0, 0, 0);
        let naive_datetime = naive_date.and_time(naive_time);
        
        let d: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
        let ndata = 200;
        let step = f64::consts::PI / (ndata as f64);

        (0..ndata).map(|i| {
            forecast::CarbonIntensityPointEstimate {
                datetime: d + chrono::Duration::minutes(i),
                value: -1.0 * f64::sin(i as f64 * step),
            }
        }).collect::<Vec<forecast::CarbonIntensityPointEstimate>>()

    }

    #[test]
    #[ignore]
    fn test_has_right_length() {
       let data = make_test_data();
        let window_size = 160;

        let start = data.first().unwrap().datetime;
        let wf = forecast::WindowedForecast::new(data, window_size, start);
        println!("len: {:?}", wf);

        // assert_eq!(wf, 200 - 160 + 1);
    }

    #[test]
    fn test_values() {

        let data = make_test_data();
        let window_size = 160;

        let start = data.first().unwrap().datetime;
        let wf = forecast::WindowedForecast::new(data, window_size, start);

        let ndata = 200;
        let step = f64::consts::PI / (ndata as f64);

        let expected: Vec<f64> = (0..ndata - window_size + 1).map(|i| {
            ((i + window_size) as f64 * step).cos() - (i as f64 * step).cos()
        }).collect();

        // average estimate

        let expected: Vec<f64> = expected.iter().map(|e| e / (window_size as f64 * step)).collect();

        for (i, e) in expected.iter().enumerate() {
            let a = wf.index(i);
            eprint!("{}", a.value - *e);
        }



    }


}
