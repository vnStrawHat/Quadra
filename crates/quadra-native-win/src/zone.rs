//! The offset of the user's time zone at a moment (specs/15, "Time where the user is"), by
//! the zone's rules for then: summer time included.

use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::System::Time::{
    FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime,
};

/// Seconds from 1601-01-01, where Windows counts from, to 1970-01-01; ticks of 100 ns a second.
const TO_1970: i64 = 11_644_473_600;
const TICKS: i64 = 10_000_000;

fn filetime(ticks: i64) -> FILETIME {
    FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 }
}

fn ticks(time: FILETIME) -> i64 {
    ((time.dwHighDateTime as i64) << 32) | time.dwLowDateTime as i64
}

/// The local clock less UTC at `unix_seconds`, in seconds; 0 where Windows cannot say.
pub(crate) fn utc_offset(unix_seconds: i64) -> i32 {
    let at = (unix_seconds + TO_1970) * TICKS;
    let (mut utc, mut local, mut back) = (SYSTEMTIME::default(), SYSTEMTIME::default(), FILETIME::default());
    // SAFETY: each pointer is to a local that lives through the call.
    let converted = unsafe {
        FileTimeToSystemTime(&filetime(at), &mut utc)
            .and_then(|()| SystemTimeToTzSpecificLocalTime(None, &utc, &mut local))
            .and_then(|()| SystemTimeToFileTime(&local, &mut back))
    };
    match converted {
        Ok(()) => ((ticks(back) - at) / TICKS) as i32,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_offset_is_one_a_time_zone_can_have() {
        let offset = super::utc_offset(1_794_000_000);
        assert!(offset.abs() <= 14 * 3600 && offset % 900 == 0, "{offset}");
    }
}
