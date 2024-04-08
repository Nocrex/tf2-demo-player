pub fn ticks_to_sec(ticks: u32, tickrate: f32) -> f32 {
    return ticks as f32 / tickrate;
}

pub fn sec_to_timestamp(sec: f32) -> String {
    let secs = (sec % 60.0) as u32;
    let mins = (sec / 60.0).trunc() as u32 % 60;
    let hrs = (sec / 3600.0).trunc() as u32;
    if hrs > 0 {
        format!("{:0>2}:{:0>2}:{:0>2}", hrs, mins, secs)
    } else {
        format!("{:0>2}:{:0>2}", mins, secs)
    }
}