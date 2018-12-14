
use std::time::{SystemTime, UNIX_EPOCH, Duration};

pub fn d_println(s: String) {
    if cfg!(debug_assertions) {
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("{} {}", to_ms(t), s);
    }
}

fn to_ms(d: Duration) -> u64 {
    d.as_secs() * 1000 + d.subsec_nanos() as u64 / 1_000_000
}
