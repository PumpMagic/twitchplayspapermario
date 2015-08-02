use std::sync::Mutex;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::f32::consts::PI;

use time;

use libvn64c::{VirtualN64Controller, VirtualN64ControllerButton};


//@todo this module's method of handling delayed votes and queueing is seriously gross
//@todo refactor this


// A democratized virtual N64 controller
pub struct DemC {
    controller: Arc<VirtualN64Controller>,
    
    tx_joystick_vote: mpsc::Sender<(time::Timespec, u16, f32, time::Duration)>,
    tx_button_vote: mpsc::Sender<VirtualN64ControllerButton>,
    tx_delayed_joystick_vote: mpsc::Sender<(time::Timespec, u16, f32, time::Duration, time::Duration)>,
    tx_delayed_button_vote: mpsc::Sender<(time::Timespec, VirtualN64ControllerButton, time::Duration)>,
    
    joystick_vote_listener: thread::JoinHandle<()>,
    button_vote_listener: thread::JoinHandle<()>
}

impl DemC {
    pub fn new(controller: VirtualN64Controller) -> DemC {
        let arc_controller = Arc::new(controller);
        
        let (tx_joystick_vote, rx_joystick_vote) = mpsc::channel();
        let (tx_button_vote, rx_button_vote): (mpsc::Sender<VirtualN64ControllerButton>, mpsc::Receiver<VirtualN64ControllerButton>) = mpsc::channel();
        //let (tx_button_vote, rx_button_vote) = mpsc::channel();
        let (tx_delayed_joystick_vote, rx_delayed_joystick_vote) = mpsc::channel();
        let (tx_delayed_button_vote, rx_delayed_button_vote) = mpsc::channel();
        
        // Spawn a joystick vote listener
        let arc_controller_joystick_vote_listener = arc_controller.clone();
        let joystick_vote_listener = thread::spawn(move || {
            let mut joystick_votes: Vec<(time::Timespec, u16, f32, time::Duration)> = Vec::new();
            let mut delayed_joystick_votes: Vec<(time::Timespec, u16, f32, time::Duration, time::Duration)> = Vec::new();
            
            loop {
                match rx_joystick_vote.try_recv() {
                    Ok((vote_time, direction, strength, duration)) => {
                        joystick_votes.push((vote_time, direction, strength, duration));
                    },
                    _ => ()
                }
                
                match rx_delayed_joystick_vote.try_recv() {
                    Ok((vote_time, direction, strength, duration, delay)) => {
                        delayed_joystick_votes.push((vote_time, direction, strength, duration, delay));
                    },
                    _ => ()
                }
                
                let time_now = time::get_time();
                
                // Transfer delayed votes to current votes if it's time
                let mut delayed_joystick_votes_fresh: Vec<(time::Timespec, u16, f32, time::Duration, time::Duration)> = Vec::new();
                for &(vote_time, direction, strength, duration, delay) in delayed_joystick_votes.iter() {
                    if time_now - delay > vote_time {
                        joystick_votes.push((time_now, direction, strength, duration));
                    } else {
                        delayed_joystick_votes_fresh.push((vote_time, direction, strength, duration, delay));
                    }
                }
                delayed_joystick_votes = delayed_joystick_votes_fresh;
                
                // Prune old votes
                let mut joystick_votes_fresh: Vec<(time::Timespec, u16, f32, time::Duration)> = Vec::new();
                for &(vote_time, direction, strength, duration) in joystick_votes.iter() {
                    if time_now - vote_time < duration {
                        joystick_votes_fresh.push((vote_time, direction, strength, duration));
                    }
                }
                joystick_votes = joystick_votes_fresh;
                
                if !joystick_votes.is_empty() {
                    // Get the average joystick direction
                    //@todo use f64 for sums?
                    let mut x_sum: f32 = 0.0;
                    let mut y_sum: f32 = 0.0;
                    let mut num_votes: u16 = 0;
                    
                    for &(_, direction, strength, _) in joystick_votes.iter() {
                        let direction_rad: f32 = (direction as f32) * PI / 180.0;
                        
                        if (direction_rad.cos() * strength).abs() > 0.0000001 {
                            x_sum += direction_rad.cos() * strength;
                        }
                        if (direction_rad.sin() * strength).abs() > 0.0000001 {
                            y_sum += direction_rad.sin() * strength;
                        }
                        
                        num_votes += 1;
                    }
                    
                    let x_avg = (x_sum / num_votes as f32) as f32;
                    let y_avg = (y_sum / num_votes as f32) as f32;
                    
                    let direction_avg_rad: f32 = (y_avg/x_avg).atan();
                    let mut direction_avg_deg: i16 = (direction_avg_rad * 180.0 / PI) as i16;
                    if x_avg < 0.0 {
                        direction_avg_deg += 180;
                    }
                    if direction_avg_deg < 0 {
                        direction_avg_deg += 360;
                    }
                    let strength_avg = (x_avg*x_avg + y_avg*y_avg).sqrt();
                    
                    arc_controller_joystick_vote_listener.set_joystick(direction_avg_deg as u16, strength_avg);
                } else {
                    arc_controller_joystick_vote_listener.set_joystick(0, 0.0);
                }
                
                
                thread::sleep_ms(1);
            }
        });
        
        // Spawn a button vote listener
        let arc_controller_button_vote_listener = arc_controller.clone();
        let button_vote_listener = thread::spawn(move || {
            //@todo these mutexes owning nothing is indicative of unrustic code
            let button_guards = [Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(()),
                                 Mutex::new(())];
            
            let mut button_votes: Vec<VirtualN64ControllerButton> = Vec::new();
            let mut delayed_button_votes: Vec<(time::Timespec, VirtualN64ControllerButton, time::Duration)> = Vec::new();
            
            loop {
                match rx_button_vote.try_recv() {
                    Ok(button) => {
                        button_votes.push(button);
                    },
                    _ => ()
                }
                
                match rx_delayed_button_vote.try_recv() {
                    Ok((vote_time, button, delay)) => {
                        delayed_button_votes.push((vote_time, button, delay));
                    },
                    _ => ()
                }
            
                // Transfer delayed votes to current votes if it's time
                let time_now = time::get_time();
                let mut delayed_button_votes_fresh: Vec<(time::Timespec, VirtualN64ControllerButton, time::Duration)> = Vec::new();
                for &(vote_time, button, delay) in delayed_button_votes.iter() {
                    if time_now - delay > vote_time {
                        button_votes.push(button);
                    } else {
                        delayed_button_votes_fresh.push((vote_time, button, delay));
                    }
                }
                delayed_button_votes = delayed_button_votes_fresh;
            
                for &button in button_votes.iter() {
                    // Is a button in a press-release cycle? If so, ignore vote
                    // Otherwise, hold the button for 0.1667 seconds and then release it indefinitely but
                    // for at least 0.0833 seconds (5 frames, at 60fps)
                    let button_guard_index = button.get_raw_index();
                    
                    match button_guards[button_guard_index].try_lock() { //@todo button -> array index map
                        Ok(_) => {
                            let closure_controller = arc_controller_button_vote_listener.clone();
                            let closure_button = button.clone();
                            thread::spawn(move || {
                                closure_controller.set_button(&closure_button, true);
                                thread::sleep_ms(167);
                                closure_controller.set_button(&closure_button, false);
                                thread::sleep_ms(83);
                            });
                        },
                        _ => ()
                    }
                }
                
                button_votes.clear();
            }
        });
        
        DemC { controller: arc_controller,
               tx_joystick_vote: tx_joystick_vote,
               tx_button_vote: tx_button_vote,
               tx_delayed_joystick_vote: tx_delayed_joystick_vote,
               tx_delayed_button_vote: tx_delayed_button_vote,
               joystick_vote_listener: joystick_vote_listener,
               button_vote_listener: button_vote_listener }
    }
    
    
    // Vote to move the joystick in a certain direction
    pub fn cast_joystick_vote(&mut self, direction: u16, strength: f32, duration: u32, delay: u32) {
        if delay > 0 {
            self.tx_delayed_joystick_vote.send((time::get_time(), direction, strength,
                                                time::Duration::milliseconds(duration as i64),
                                                time::Duration::milliseconds(delay as i64)));
        } else {
            self.tx_joystick_vote.send((time::get_time(), direction, strength,
                                        time::Duration::milliseconds(duration as i64)));
        }
    }


    // Vote to press a button
    pub fn cast_button_vote(&self, button: VirtualN64ControllerButton, delay: u32) {
        if delay > 0 {
            self.tx_delayed_button_vote.send((time::get_time(), button, time::Duration::milliseconds(delay as i64)));
        } else {
            self.tx_button_vote.send(button);
        }
    }
}