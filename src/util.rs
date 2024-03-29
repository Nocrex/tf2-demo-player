pub fn ticks_to_sec(ticks: u32, tickrate: f32) -> f32 {
    return ticks as f32 / tickrate;
}

pub fn sec_to_ticks(sec: f32, tickrate: f32) -> u32 {
    return (sec * tickrate) as u32;
}

pub fn sec_to_timestamp(sec: f32) -> String {
    let secs = sec % 60.0;
    let mins = (sec / 60.0).trunc() as u32 % 60;
    let hrs = (sec / 3600.0).trunc() as u32;
    if hrs > 0 {
        format!("{:0>2}:{:0>2}:{:0>5.2}", hrs, mins, secs)
    } else {
        format!("{:0>2}:{:0>5.2}", mins, secs)
    }
}

pub fn timestamp_to_sec(ts: String) -> f32 {
    let mut val: f32 = 0.0;
    for part in ts.split(":") {
        val *= 60.0;
        val += part.parse::<f32>().unwrap() as f32;
    }
    val
}