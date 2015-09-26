use std;
use std::sync::Mutex;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use time;
use time::{Timespec, Duration, get_time};

use regex::Regex;

pub mod virtc;
use demc::virtc::AcceptsInputs;


const MAX_JOYSTICK_COMMAND_DURATION: u32 = 5000;
const MAX_BUTTON_COMMAND_DURATION: u32 = 5000;
const MAX_B_BUTTON_COMMAND_DURATION: u32 = 30000;
const MAX_DURATION_PER_LINE: u32 = 30000;
const MILLISECONDS_PER_FRAME: u32 = 34;


fn get_button_guard_index_n64(name: &str) -> usize {
    // Zero-based indexing of enum values
    //@todo this really shouldn't be necessary
    match name {
        "a" => 0,
        "b" => 1,
        "z" => 2,
        "l" => 3,
        "r" => 4,
        "start" => 5,
        "cup" => 6,
        "cdown" => 7,
        "cleft" => 8,
        "cright" => 9,
        "dup" => 10,
        "ddown" => 11,
        "dleft" => 12,
        "dright" => 13,
        _ => panic!("uhhh")
    }
}

fn get_button_guard_index_gcn(name: &str) -> usize {
    // Zero-based indexing of enum values
    //@todo this really shouldn't be necessary
    match name {
        "a" => 0,
        "b" => 1,
        "x" => 2,
        "y" => 3,
        "z" => 4,
        "l" => 5,
        "r" => 6,
        "start" => 7,
        "dup" => 8,
        "ddown" => 9,
        "dleft" => 10,
        "dright" => 11,
        _ => panic!("uhhh")
    }
}

#[derive(Clone)]
pub struct TimedInput {
    pub start_time: Timespec,
    pub duration: Duration,
    pub command: virtc::Input
}

trait CommandedAsynchronously {
    fn get_tx_command(&self) -> &mpsc::Sender<TimedInput>;
    fn get_command_listener(&self) -> &thread::JoinHandle<()>;
}

pub trait ChatInterfaced: CommandedAsynchronously {
    fn get_regex(&self) -> &Regex;

    fn handle_commands(&self, commands: &String) -> Result<(), ()> {
        match self.parse_string_as_commands(commands) {
            Some(commands) => {
                for command in commands.iter() {
                    self.add_command(command);
                }
                Ok(())
            },
            None => Err(())
        }
    }

    // Attempt to parse an IRC message into a list of controller commands
    //@todo just use a custom state machine rather than regex, this has to be insanely slow
    //@todo this function is huge
    fn parse_string_as_commands(&self, msg: &String) -> Option<Vec<TimedInput>> {
        let mut last_cap_end = None;
        let mut cumulative_delay: u32 = 0;
        let mut last_command: Option<TimedInput> = None;
        let mut cap_start_zero = false;
        let mut final_cap_end: usize = 0;
        let mut res: Vec<TimedInput> = Vec::new();

        for cap in self.get_regex().captures_iter(&msg.to_lowercase()) {
            let (cap_start, cap_end) = cap.pos(0).unwrap();

            // Raise a flag if this capture starts at the first string position - after iterating over all captures, we'll
            // make sure it was raised, and if it wasn't, we didn't parse a valid command message
            if cap_start == 0 {
                cap_start_zero = true;
            }

            // Make sure that all captures are continuous; that we're parsing commands and only commands
            // eg. we don't want "hahah" to parse as two "a" commands
            if let Some(last_cap_end) = last_cap_end {
                if last_cap_end != cap_start {
                    return None;
                }
            }

            // Store the last capture's ending position - after iterating over all captures, we'll make sure the last
            // one ended
            final_cap_end = cap_end;

            // Our regex should match on exactly one of three groups: "joystick", "button", or "delay"
            if let Some(_) = cap.name("joystick") {
                match last_command {
                    Some(command) => {
                        cumulative_delay += command.duration.num_milliseconds() as u32;
                        match command.command {
                            virtc::Input::Axis(_, _) => {
                                cumulative_delay += MILLISECONDS_PER_FRAME*2;
                            },
                            virtc::Input::Button(_, _) => {
                                ()
                            }
                        }
                    },
                    None =>  ()
                }
                // joystick command - should have one, two, or four groups:
                // "joystick_strength" (optional)
                // "joystick_direction" (mandatory)
                // "joystick_duration" (optional),
                // "joystick_duration_units" (optional; must be present if joystick_duration is)
                let mut joystick_strength: f32 = 1.0;
                let mut joystick_direction: u16 = 0;
                let mut joystick_duration: u32 = 500;
                if let Some(jscap) = cap.name("joystick_strength") {
                    match jscap.parse::<u8>() {
                        Ok(strength_u8) => { joystick_strength = strength_u8 as f32 / 100.0; },
                        _ => return None
                    }
                }
                if let Some(jdcap) = cap.name("joystick_direction") {
                    match jdcap {
                        "up" => { joystick_direction = 90; },
                        "down" => { joystick_direction = 270; },
                        "left" => { joystick_direction = 180; },
                        "right" => { joystick_direction = 0; },
                        _ => ()
                    }
                } else {
                    return None;
                }
                if let Some(jdcap) = cap.name("joystick_duration") {
                    match jdcap.parse::<u32>() {
                        Ok(duration_u32) => { joystick_duration = duration_u32; },
                        _ => return None
                    }

                    if let Some(jdcap_units) = cap.name("joystick_duration_units") {
                        if jdcap_units == "s" {
                            joystick_duration *= 1000;
                        }
                    } else {
                        return None;
                    }
                }

                // treat joystick commands with strength <0%, >100% or duration >5s as invalid
                if joystick_strength > 1.0 || joystick_strength < 0.0 || joystick_duration > MAX_JOYSTICK_COMMAND_DURATION {
                    return None;
                }

                let time_now = get_time();
                let command = TimedInput { start_time: time_now + Duration::milliseconds(cumulative_delay as i64),
                                           duration: Duration::milliseconds(joystick_duration as i64),
                                           command: virtc::Input::Joystick(String::from("control_stick"), joystick_direction, joystick_strength) };
                res.push(command.clone());

                last_command = Some(command);
            } else if let Some(_) = cap.name("button") {
                match last_command {
                    Some(command) => {
                        match command.command {
                            virtc::Input::Axis(_, _) => {
                                cumulative_delay += command.duration.num_milliseconds() as u32;
                                if command.duration.num_milliseconds() >= MILLISECONDS_PER_FRAME as i64 {
                                    cumulative_delay -= MILLISECONDS_PER_FRAME;
                                }
                            },
                            virtc::Input::Button(_, _) => {
                                if command.duration.num_milliseconds() == MILLISECONDS_PER_FRAME as i64 * 5 {
                                    cumulative_delay += 500; //massive hack
                                } else {
                                    cumulative_delay += command.duration.num_milliseconds() as u32;
                                }
                                cumulative_delay += 51;
                            }
                        }
                    },
                    None =>  ()
                }
                // button command - should have one or three groups:
                // "button_name" (mandatory)
                // "button_duration" (optional),
                // "button_duration_units" (optional; must be present if joystick_duration is)
                let button_name;
                let mut button_duration: u32 = MILLISECONDS_PER_FRAME*5;

                if let Some(bncap) = cap.name("button_name") {
                    button_name = bncap;
                } else {
                    return None;
                }

                if let Some(bdcap) = cap.name("button_duration") {
                    match bdcap.parse::<u32>() {
                        Ok(duration_u32) => { button_duration = duration_u32; },
                        _ => return None
                    }

                    if let Some(bdcap_units) = cap.name("button_duration_units") {
                        if bdcap_units == "s" {
                            button_duration *= 1000;
                        }
                    } else {
                        return None;
                    }
                }

                if cap.name("button_name").unwrap() == "b" {
                    if button_duration > MAX_B_BUTTON_COMMAND_DURATION {
                        return None;
                    }
                } else {
                    if button_duration > MAX_BUTTON_COMMAND_DURATION {
                        return None;
                    }
                }

                let time_now = get_time();
                let command = TimedInput { start_time: time_now + Duration::milliseconds(cumulative_delay as i64),
                                                  duration: Duration::milliseconds(button_duration as i64),
                                                  command: virtc::Input::Button(String::from(button_name), true)};
                res.push(command.clone());

                last_command = Some(command);
            } else if let Some(dcap) = cap.name("delay") {
                // delay command - only one argument, the delay to insert
                match dcap {
                    "+" => { cumulative_delay += 17; },
                    "!" => { cumulative_delay += 217; },
                    "." => {
                        match last_command {
                            Some(command) => { cumulative_delay += command.duration.num_milliseconds() as u32; },
                            None =>  ()
                        }
                        cumulative_delay += 250;
                    },
                    _ => { return None; }
                }
                last_command = None
            }

            last_cap_end = Some(cap_end);
        }

        if cumulative_delay > MAX_DURATION_PER_LINE {
                return None;
        }

        if final_cap_end != msg.len() {
            return None;
        }

        if cap_start_zero != true {
            return None;
        }

        return Some(res);
    }

    fn add_command(&self, command: &TimedInput) {
        self.get_tx_command().send(command.clone());
    }
}

// A democratized virtual controller
pub struct DemN64C {
    controller: Arc<virtc::VN64C>,
    re: Regex,
    tx_command: mpsc::Sender<TimedInput>,
    command_listener: thread::JoinHandle<()>
}

impl CommandedAsynchronously for DemN64C {
    fn get_tx_command(&self) -> &mpsc::Sender<TimedInput> {
        return &self.tx_command;
    }

    fn get_command_listener(&self) -> &thread::JoinHandle<()> {
        return &self.command_listener;
    }
}


impl ChatInterfaced for DemN64C {
    fn get_regex(&self) -> &Regex {
        return &self.re;
    }
}

impl DemN64C {
    pub fn new(vjoy_device_number: u32) -> Result<Self, u8> {
        let controller = match virtc::VN64C::new(vjoy_device_number) {
            Ok(controller) => controller,
            Err(_) => return Err(1)
        };

        let arc_controller = Arc::new(controller);
        
        let (tx_command, rx_command) = mpsc::channel();
        
        //@todo these mutexes owning nothing is indicative of unrustic code
        let button_guards = [Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                             Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                             Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                             Mutex::new(()), Mutex::new(())];
        
        // Spawn a command listener
        let arc_controller_command_handler = arc_controller.clone();
        let command_listener = thread::spawn(move || {
            let mut queued_commands: Vec<TimedInput> = Vec::new();
            let mut active_joystick_commands: Vec<TimedInput> = Vec::new();
            // There is no active button commands vector because closures
            
            loop {
                // Get all commands from the mpsc receiver
                loop {
                    match rx_command.try_recv() {
                        Ok(command) => {
                            queued_commands.push(command);
                        },
                        _ => { break; }
                    }
                }
                
                let time_now = time::get_time();
                
                // Move all queued joystick commands whose time it is into the active joystick command list
                // Try acting on all queued button commands whose time it is
                let mut queued_commands_fresh: Vec<TimedInput> = Vec::new();
                for command in queued_commands.iter() {
                    if command.start_time <= time_now {
                        match command.command.clone() {
                            virtc::Input::Axis(_, _) => {
                                active_joystick_commands.push(command.clone());
                            }
                            virtc::Input::Button(name, _) => {
                                // Is a button in a press-release cycle? If so, ignore vote
                                // Otherwise, hold the button for as long as the command specified,
                                // then release it indefinitely but for at least 0.0498 seconds
                                // (3 frames, at 60fps)

                                let button_guard_index = get_button_guard_index_n64(&name);
                                
                                match button_guards[button_guard_index].try_lock() {
                                    Ok(_) => {
                                        let closure_controller = arc_controller_command_handler.clone();
                                        let closure_button_name = name.clone();

                                        let myclone = command.clone();
                                        thread::spawn(move || {
                                            let command1 = virtc::Input::Button(closure_button_name.clone(), true);
                                            closure_controller.set_input(&command1);
                                            thread::sleep_ms(myclone.duration.num_milliseconds() as u32);
                                            let command2 = virtc::Input::Button(closure_button_name.clone(), false);
                                            closure_controller.set_input(&command2);
                                            thread::sleep_ms(34);
                                        });

                                    },
                                    _ => ()
                                }

                            }
                        }
                    } else {
                        queued_commands_fresh.push(command.clone());
                    }
                }
                queued_commands = queued_commands_fresh;
                
                // Prune old commands from the active list
                let mut active_joystick_commands_fresh: Vec<TimedInput> = Vec::new();
                for command in active_joystick_commands.iter() {
                    if command.start_time + command.duration > time_now {
                        active_joystick_commands_fresh.push(command.clone());
                    }
                }
                active_joystick_commands = active_joystick_commands_fresh;
                
                if !active_joystick_commands.is_empty() {
                    // Get the average joystick direction
                    //@todo use f64 for sums?
                    let mut x_sum: f32 = 0.0;
                    let mut y_sum: f32 = 0.0;
                    let mut num_x_commands: u16 = 0;
                    let mut num_y_commands: u16 = 0;
                    
                    // Loop over all commands
                    for command in active_joystick_commands.iter() {
                        match command.command.clone() {
                            virtc::Input::Axis(name, strength) => {
                                if name == "x" {
                                    x_sum += strength;
                                    num_x_commands += 1;
                                } else if name == "y" {
                                    y_sum += strength;
                                    num_y_commands += 1;
                                }

                            },
                            _ => panic!("How did something besides an axis command get here?")
                        }
                    }

                    let x_avg = (x_sum / num_x_commands as f32) as f32;
                    let y_avg = (y_sum / num_y_commands as f32) as f32;

                    let x_command = virtc::Input::Axis(String::from("x"), x_avg);
                    let y_command = virtc::Input::Axis(String::from("y"), y_avg);
                    arc_controller_command_handler.set_input(&x_command);
                    arc_controller_command_handler.set_input(&y_command);
                } else {
                    let x_command = virtc::Input::Axis(String::from("x"), 0.0);
                    let y_command = virtc::Input::Axis(String::from("y"), 0.0);
                    arc_controller_command_handler.set_input(&x_command);
                    arc_controller_command_handler.set_input(&y_command);
                }
                
                thread::sleep_ms(1);
            }
        });
        
        Ok( DemN64C { controller: arc_controller,
               re: Regex::new(r"\s*((?P<joystick>((?P<joystick_strength>[:digit:]+)%\s*)?(?P<joystick_direction>up|down|left|right)(\s*(?P<joystick_duration>[:digit:]+)(?P<joystick_duration_units>s|ms))?)|(?P<button>((?P<button_name>start|cup|cdown|cleft|cright|dup|ddown|dleft|dright|a|b|z|l|r)(\s*(?P<button_duration>[:digit:]+)(?P<button_duration_units>s|ms))?))|(?P<delay>[\+!\.]))\s*").unwrap(),
               tx_command: tx_command,
               command_listener: command_listener } )
    }
}















// A democratized virtual controller
pub struct DemGcnC {
    controller: Arc<virtc::VGcnC>,
    re: Regex,
    tx_command: mpsc::Sender<TimedInput>,
    command_listener: thread::JoinHandle<()>
}

impl CommandedAsynchronously for DemGcnC {
    fn get_tx_command(&self) -> &mpsc::Sender<TimedInput> {
        return &self.tx_command;
    }

    fn get_command_listener(&self) -> &thread::JoinHandle<()> {
        return &self.command_listener;
    }
}


impl ChatInterfaced for DemGcnC {
    fn get_regex(&self) -> &Regex {
        return &self.re;
    }
}

impl DemGcnC {
    pub fn new(vjoy_device_number: u32) -> Result<Self, u8> {
        let controller = match virtc::VGcnC::new(vjoy_device_number) {
            Ok(controller) => controller,
            Err(_) => return Err(1)
        };

        let arc_controller = Arc::new(controller);

        let (tx_command, rx_command) = mpsc::channel();

        //@todo these mutexes owning nothing is indicative of unrustic code
        let button_guards = [Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                             Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                             Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(())];

        // Spawn a command listener
        let arc_controller_command_handler = arc_controller.clone();
        let command_listener = thread::spawn(move || {
            let mut queued_commands: Vec<TimedInput> = Vec::new();
            let mut active_joystick_commands: Vec<TimedInput> = Vec::new();
            // There is no active button commands vector because closures

            loop {
                // Get all commands from the mpsc receiver
                loop {
                    match rx_command.try_recv() {
                        Ok(command) => {
                            queued_commands.push(command);
                        },
                        _ => { break; }
                    }
                }

                let time_now = time::get_time();

                // Move all queued joystick commands whose time it is into the active joystick command list
                // Try acting on all queued button commands whose time it is
                let mut queued_commands_fresh: Vec<TimedInput> = Vec::new();
                for command in queued_commands.iter() {
                    if command.start_time <= time_now {
                        match command.command.clone() {
                            virtc::Input::Axis(_, _) => {
                                active_joystick_commands.push(command.clone());
                            }
                            virtc::Input::Button(name, _) => {
                                // Is a button in a press-release cycle? If so, ignore vote
                                // Otherwise, hold the button for as long as the command specified,
                                // then release it indefinitely but for at least 0.0498 seconds
                                // (3 frames, at 60fps)

                                let button_guard_index = get_button_guard_index_gcn(&name);

                                match button_guards[button_guard_index].try_lock() {
                                    Ok(_) => {
                                        let closure_controller = arc_controller_command_handler.clone();
                                        let closure_button_name = name.clone();

                                        let myclone = command.clone();
                                        thread::spawn(move || {
                                            let command1 = virtc::Input::Button(closure_button_name.clone(), true);
                                            closure_controller.set_input(&command1);
                                            thread::sleep_ms(myclone.duration.num_milliseconds() as u32);
                                            let command2 = virtc::Input::Button(closure_button_name.clone(), false);
                                            closure_controller.set_input(&command2);
                                            thread::sleep_ms(34);
                                        });

                                    },
                                    _ => ()
                                }

                            }
                        }
                    } else {
                        queued_commands_fresh.push(command.clone());
                    }
                }
                queued_commands = queued_commands_fresh;

                // Prune old commands from the active list
                let mut active_joystick_commands_fresh: Vec<TimedInput> = Vec::new();
                for command in active_joystick_commands.iter() {
                    if command.start_time + command.duration > time_now {
                        active_joystick_commands_fresh.push(command.clone());
                    }
                }
                active_joystick_commands = active_joystick_commands_fresh;

                if !active_joystick_commands.is_empty() {
                    // Get the average joystick direction
                    //@todo use f64 for sums?
                    let mut x_sum: f32 = 0.0;
                    let mut y_sum: f32 = 0.0;
                    let mut num_x_commands: u16 = 0;
                    let mut num_y_commands: u16 = 0;

                    // Loop over all commands
                    for command in active_joystick_commands.iter() {
                        match command.command.clone() {
                            virtc::Input::Axis(name, strength) => {
                                if name == "jx" {
                                    x_sum += strength;
                                    num_x_commands += 1;
                                } else if name == "jy" {
                                    y_sum += strength;
                                    num_y_commands += 1;
                                }

                            },
                            _ => panic!("How did something besides an axis command get here?")
                        }
                    }

                    let x_avg = (x_sum / num_x_commands as f32) as f32;
                    let y_avg = (y_sum / num_y_commands as f32) as f32;
                    
                    let x_command = virtc::Input::Axis(String::from("jx"), x_avg);
                    let y_command = virtc::Input::Axis(String::from("jy"), y_avg);
                    arc_controller_command_handler.set_input(&x_command);
                    arc_controller_command_handler.set_input(&y_command);
                } else {
                    let x_command = virtc::Input::Axis(String::from("jx"), 0.0);
                    let y_command = virtc::Input::Axis(String::from("jy"), 0.0);
                    arc_controller_command_handler.set_input(&x_command);
                    arc_controller_command_handler.set_input(&y_command);
                }

                thread::sleep_ms(1);
            }
        });

        Ok( DemGcnC { controller: arc_controller,
               re: Regex::new(r"\s*((?P<joystick>((?P<joystick_strength>[:digit:]+)%\s*)?(?P<joystick_direction>up|down|left|right)(\s*(?P<joystick_duration>[:digit:]+)(?P<joystick_duration_units>s|ms))?)|(?P<button>((?P<button_name>start|cup|cdown|cleft|cright|dup|ddown|dleft|dright|a|b|x|y|z|l|r)(\s*(?P<button_duration>[:digit:]+)(?P<button_duration_units>s|ms))?))|(?P<delay>[\+!\.]))\s*").unwrap(),
               tx_command: tx_command,
               command_listener: command_listener } )
    }
}