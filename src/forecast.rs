use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Index;

use chrono::DateTime;
use chrono::Utc;


#[derive(Debug, Copy, Clone)]
pub struct CarbonIntensityPointEstimate {
    pub value: f64,
    pub datetime: DateTime<Utc>,
}

impl CarbonIntensityPointEstimate {
    pub fn new(value: f64, datetime: DateTime<Utc>) -> CarbonIntensityPointEstimate {
        CarbonIntensityPointEstimate { value, datetime }
    }
}

impl Display for CarbonIntensityPointEstimate {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}\t{}", self.datetime, self.value)
    }
}

impl PartialOrd for CarbonIntensityPointEstimate {
    fn partial_cmp(&self, other: &CarbonIntensityPointEstimate) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}



impl PartialEq for CarbonIntensityPointEstimate {
    fn eq(&self, other: &CarbonIntensityPointEstimate) -> bool {
        self.value == other.value
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CarbonIntensityAverageEstimate {
    pub value: f64,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}


impl PartialOrd for CarbonIntensityAverageEstimate {
    fn partial_cmp(&self, other: &CarbonIntensityAverageEstimate) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl Ord for CarbonIntensityAverageEstimate {
    fn cmp(&self, other: &CarbonIntensityAverageEstimate) -> std::cmp::Ordering {
        self.value.partial_cmp(&other.value).expect("Could not compare values")
    }
}


impl PartialEq for CarbonIntensityAverageEstimate {
    fn eq(&self, other: &CarbonIntensityAverageEstimate) -> bool {
        self.value == other.value
    }
}

impl Eq for CarbonIntensityAverageEstimate {}

#[derive(Debug)]
pub struct WindowedForecast {
    data: Vec<CarbonIntensityPointEstimate>,
    duration: i64, // in minutes
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    ndata: usize,
    data_stepsize: i64,
    pub index: usize,

}

impl WindowedForecast {
    pub fn new(data: Vec<CarbonIntensityPointEstimate>, duration: i64, start: DateTime<Utc>) -> WindowedForecast {

        let data_stepsize = data.get(1).expect("data must have at least 2 elements").datetime.timestamp() - data.get(0).expect("data must have at least 2 elements").datetime.timestamp();

        let end = start.timestamp() + duration * 60; // add to struct



        let data: Vec<CarbonIntensityPointEstimate> = data.into_iter().filter(|d| d.datetime.timestamp() > start.timestamp() && d.datetime.timestamp() < end).collect();

        fn bisect_left(data: &[CarbonIntensityPointEstimate], t: i64, data_stepsize: &i64) -> usize {
            for (i, d) in data.iter().enumerate() {
                if d.datetime.timestamp() + data_stepsize >= t {
                    return i;
                }
            }
            panic!("t is greater than the last element in data");
        }

        let ndata = bisect_left(&data, end, &data_stepsize);


        let end = DateTime::from_timestamp(end, 0).expect("Could not convert timestamp to DateTime").with_timezone(&Utc);

        WindowedForecast {
            index: 0,
            data,
            data_stepsize,
            start,
            duration,
            end,
            ndata
        }

    }


    fn interp(p1: &CarbonIntensityPointEstimate, p2: &CarbonIntensityPointEstimate, when: DateTime<Utc>) -> CarbonIntensityPointEstimate {


        let timestep = p2.datetime.timestamp() - p1.datetime.timestamp();

        let slope = (p2.value - p1.value) / timestep as f64;

        let offset = when.timestamp() - p1.datetime.timestamp();

        CarbonIntensityPointEstimate::new(p1.value + slope * offset as f64, when)

    }


    // This is not an ideomatic implementation of Index, because Index returns a reference not a
    // value
    pub fn index(&self, index: usize) -> CarbonIntensityAverageEstimate {


        let window_start = self.start.timestamp() + index as i64 * self.data_stepsize;
        let window_end = self.end.timestamp() + index as i64 * self.data_stepsize;


        let lbound = WindowedForecast::interp(
            self.data.get(index).expect("index out of bounds"),
            self.data.get(index + 1).expect("index out of bounds"),
            DateTime::from_timestamp(window_start, 0).expect("Could not convert timestamp to DateTime").with_timezone(&Utc)
        );


        let rbound = if self.ndata == self.data.len() {
            *self.data.last().unwrap()
        } else {
            WindowedForecast::interp(
                self.data.get(index + self.ndata - 1).expect("index out of bounds"),
                self.data.get(index + self.ndata).expect("index out of bounds"),
                DateTime::from_timestamp(window_end, 0).expect("Could not convert timestamp to DateTime").with_timezone(&Utc)
            )
        };


   
        // This is kinda gross... fix it later // FIXME
        let mut window_data = vec![lbound];
        window_data.extend(self.data.iter().skip(index).take(self.ndata));
        window_data.extend(vec![rbound]);

        let acc = window_data.iter().rev().zip(window_data.iter().skip(1)).map(|(a, b)| {
            0.5 * (a.value + b.value) * (b.datetime - a.datetime).num_seconds() as f64
        }).collect::<Vec<f64>>();

       let duration = window_data.last().unwrap().datetime - window_data.first().unwrap().datetime;
    
        CarbonIntensityAverageEstimate {
            value: acc.iter().sum::<f64>() / duration.num_seconds() as f64,
            start: window_data.first().unwrap().datetime,
            end: window_data.last().unwrap().datetime,
        }

    
}

    pub fn len(&self) -> usize {
        self.data.len() - self.ndata + 1
    }

}

impl Iterator for WindowedForecast {
    type Item = CarbonIntensityAverageEstimate;

    fn next(&mut self) -> Option<Self::Item> {
   
        if self.index < self.len() {
            let result = Some(self.index(self.index));
            self.index += 1;
            result
        } else {
            None
        }


  }
}






#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_by_value_lt() {
        let a = CarbonIntensityPointEstimate::new(1.0, Utc::now());
        let b = CarbonIntensityPointEstimate::new(0.0, Utc::now());
        assert!(a > b);
    }

    #[test]
    fn test_order_by_value_gt() {
        let a = CarbonIntensityPointEstimate::new(0.0, Utc::now());
        let b = CarbonIntensityPointEstimate::new(1.0, Utc::now());
        assert!(a < b);
    }


    #[test]
    fn test_order_by_value_eq() {
        let a = CarbonIntensityPointEstimate::new(0.0, Utc::now());
        let b = CarbonIntensityPointEstimate::new(0.0, Utc::now());
        assert!(a == b);
    }
}
