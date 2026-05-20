use std::{thread::sleep, time::Duration};

use user_idle3::UserIdle;

fn main() {
    loop {
        let user_idle = UserIdle::get_time().unwrap();
        println!("User idle duration: {:?}", user_idle.duration());
        sleep(Duration::from_secs(1));
    }
}
