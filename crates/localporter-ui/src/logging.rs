use std::{
    io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use localporter_core::{init_file_logger, log_info};

use crate::state::logs_dir_path;

pub fn init_logging() -> io::Result<PathBuf> {
    let logs_dir = logs_dir_path().ok_or_else(|| io::Error::other("logs path unavailable"))?;
    let path = logs_dir.join(instance_log_file_name(SystemTime::now()));
    let initialized_path = init_file_logger(path)?;
    log_info!("logging initialized: path={}", initialized_path.display());
    Ok(initialized_path)
}

fn instance_log_file_name(now: SystemTime) -> String {
    let timestamp = format_timestamp(now);
    format!("localporter-{timestamp}Z.log")
}

fn format_timestamp(now: SystemTime) -> String {
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = duration.as_secs() as i64;
    let millis = duration.subsec_millis();

    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);

    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}-{millis:03}")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn instance_log_file_name_contains_sortable_timestamp() {
        let now = UNIX_EPOCH + Duration::from_secs(1_752_293_531) + Duration::from_millis(245);
        let file_name = instance_log_file_name(now);

        assert_eq!(file_name, "localporter-20250712-041211-245Z.log");
    }
}
