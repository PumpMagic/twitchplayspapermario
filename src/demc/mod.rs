use std;
use std::ops::Deref;
use std::sync::Mutex;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use time;
use time::{Timespec, Duration, get_time};

use regex::Regex;

pub mod virtc;
pub mod vgcnc;
pub mod vn64c;

use demc::virtc::{AcceptsInputs, HasJoysticks, HasButtons};


const MILLISECONDS_PER_SECOND: u32 = 1000;
const FRAMES_PER_SECOND: u32 = 60; //@todo accept at runtime
const MILLISECONDS_PER_FRAME: u32 = 17; //@todo calculate this at runtime

const DEFAULT_JOYSTICK_COMMAND_DURATION: u32 = MILLISECONDS_PER_SECOND/4;
const DEFAULT_BUTTON_COMMAND_DURATION: u32 = MILLISECONDS_PER_SECOND/2;

const MAX_JOYSTICK_COMMAND_DURATION: u32 = 5000;
const MAX_BUTTON_COMMAND_DURATION: u32 = 5000;
const MAX_B_BUTTON_COMMAND_DURATION: u32 = 30000;
const MAX_X_BUTTON_COMMAND_DURATION: u32 = 30000;
const MAX_R_BUTTON_COMMAND_DURATION: u32 = 10000;
const MAX_START_BUTTON_COMMAND_DURATION: u32 = 500;
const MAX_DURATION_PER_LINE: u32 = 30000;
const MILLISECONDS_PER_DOT: u32 = 250;

const JOYSTICK_TO_JOYSTICK_DELAY: u32 = MILLISECONDS_PER_FRAME*2;
const BUTTON_TO_JOYSTICK_DELAY: u32 = 0;
const BUTTON_TO_BUTTON_DELAY: u32 = MILLISECONDS_PER_FRAME*3;
const JOYSTICK_TO_BUTTON_UNDELAY: u32 = MILLISECONDS_PER_FRAME;
const SIMULTANEOUS_COMMAND_DELAY: u32 = MILLISECONDS_PER_FRAME;


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
    fn parse_string_as_commands(&self, msg: &String) -> Option<Vec<TimedInput>>;

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

    fn add_command(&self, command: &TimedInput) {
        self.get_tx_command().send(command.clone());
    }
}


pub struct ControllerConstraints {
    pub illegal_combinations: Vec<(String, Vec<String>)>,
    //max_durations: Vec<
}


// A democratized virtual controller
pub struct DemC<T> {
    controller: Arc<T>,
    re: Regex,
    tx_command: mpsc::Sender<TimedInput>,
    command_listener: thread::JoinHandle<()>
}

impl<T> CommandedAsynchronously for DemC<T> {
    fn get_tx_command(&self) -> &mpsc::Sender<TimedInput> {
        return &self.tx_command;
    }

    fn get_command_listener(&self) -> &thread::JoinHandle<()> {
        return &self.command_listener;
    }
}


impl<T> ChatInterfaced for DemC<T> {
    fn get_regex(&self) -> &Regex {
        return &self.re;
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
                            virtc::Input::Joystick(_, _, _) => {
                                cumulative_delay += JOYSTICK_TO_JOYSTICK_DELAY;
                            },
                            virtc::Input::Button(_, _) => {
                                cumulative_delay += BUTTON_TO_JOYSTICK_DELAY;
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
                let mut joystick_name = "";
                let mut joystick_duration: u32 = DEFAULT_JOYSTICK_COMMAND_DURATION;
                if let Some(jscap) = cap.name("joystick_strength") {
                    match jscap.parse::<u8>() {
                        Ok(strength_u8) => { joystick_strength = strength_u8 as f32 / 100.0; },
                        _ => return None
                    }
                }
                if let Some(jdcap) = cap.name("joystick_direction") {
                    match jdcap {
                        "cup" => { joystick_direction = 90; joystick_name = "c_stick"; },
                        "cdown" => { joystick_direction = 270; joystick_name = "c_stick"; },
                        "cleft" => { joystick_direction = 180; joystick_name = "c_stick"; },
                        "cright" => { joystick_direction = 0; joystick_name = "c_stick"; },
                        "up" => { joystick_direction = 90; joystick_name = "control_stick"; },
                        "down" => { joystick_direction = 270; joystick_name = "control_stick"; },
                        "left" => { joystick_direction = 180; joystick_name = "control_stick"; },
                        "right" => { joystick_direction = 0; joystick_name = "control_stick"; },
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
                            joystick_duration *= MILLISECONDS_PER_SECOND;
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
                                           command: virtc::Input::Joystick(String::from(joystick_name), joystick_direction, joystick_strength) };
                res.push(command.clone());

                last_command = Some(command);
            } else if let Some(_) = cap.name("button") {
                match last_command {
                    Some(command) => {
                        match command.command {
                            virtc::Input::Joystick(_, _, _) => {
                                cumulative_delay += command.duration.num_milliseconds() as u32;
                                if command.duration.num_milliseconds() >= JOYSTICK_TO_BUTTON_UNDELAY as i64 {
                                    cumulative_delay -= JOYSTICK_TO_BUTTON_UNDELAY;
                                }
                            },
                            virtc::Input::Button(_, _) => {
                                cumulative_delay += command.duration.num_milliseconds() as u32;
                                cumulative_delay += BUTTON_TO_BUTTON_DELAY;
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
                let mut button_duration: u32 = DEFAULT_BUTTON_COMMAND_DURATION;

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
                            button_duration *= MILLISECONDS_PER_SECOND;
                        }
                    } else {
                        return None;
                    }
                }

                //@todo support per-button max durations in the passed constraints
                let max_duration = match button_name {
                    "b" => MAX_B_BUTTON_COMMAND_DURATION,
                    "x" => MAX_X_BUTTON_COMMAND_DURATION,
                    "r" => MAX_R_BUTTON_COMMAND_DURATION,
                    "start" => MAX_START_BUTTON_COMMAND_DURATION,
                    _ => MAX_BUTTON_COMMAND_DURATION
                };
                if button_duration > max_duration {
                    return None;
                }

                let time_now = get_time();
                let command = TimedInput { start_time: time_now + Duration::milliseconds(cumulative_delay as i64),
                                                  duration: Duration::milliseconds(button_duration as i64),
                                                  command: virtc::Input::Button(String::from(button_name), true)};
                res.push(command.clone());

                last_command = Some(command);
            } else if let Some(dncap) = cap.name("delay_duration") {
                let mut delay_duration;
                match dncap.parse::<u32>() {
                    Ok(duration_u32) => { delay_duration = duration_u32; },
                    _ => return None
                }

                if let Some(dncap_units) = cap.name("delay_duration_units") {
                    if dncap_units == "s" {
                        delay_duration *= MILLISECONDS_PER_SECOND;
                    }
                } else {
                    return None;
                }
                
                match last_command {
                    Some(command) => { cumulative_delay += command.duration.num_milliseconds() as u32; },
                    None =>  ()
                }
                cumulative_delay += delay_duration;
                
                last_command = None
            } else if let Some(dcap) = cap.name("delay_hardcode") {
                // delay command - only one argument, the delay to insert
                match dcap {
                    "+" => { cumulative_delay += MILLISECONDS_PER_FRAME; },
                    "!" => { 
                        match last_command {
                            Some(command) => { cumulative_delay += command.duration.num_milliseconds() as u32; },
                            None => ()
                        }
                        cumulative_delay += MILLISECONDS_PER_FRAME;
                    },
                    "." => {
                        match last_command {
                            Some(command) => { cumulative_delay += command.duration.num_milliseconds() as u32; },
                            None => ()
                        }
                        cumulative_delay += MILLISECONDS_PER_DOT;
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
}

impl<T> DemC<T> where T: AcceptsInputs + Send + Sync + 'static {
    pub fn new(controller: T, constraints: ControllerConstraints) -> Result<DemC<T>, u8> where T: HasButtons + HasJoysticks {
        let arc_controller = Arc::new(controller);

        let (tx_command, rx_command) = mpsc::channel();

        // Spawn a command listener
        let arc_controller_command_handler = arc_controller.clone();
        let command_listener = thread::spawn(move || {
            //@todo these mutexes owning nothing is indicative of unrustic code
            //@todo size according to number of buttons
            let button_guards = Arc::new(
                                    vec![Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                                         Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                                         Mutex::new(()), Mutex::new(()), Mutex::new(()), Mutex::new(()),
                                         Mutex::new(()), Mutex::new(())]);
        
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
                            virtc::Input::Joystick(_, _, _) => {
                                active_joystick_commands.push(command.clone());
                            }
                            virtc::Input::Button(name, _) => {
                                // Is a button in a press-release cycle? If so, ignore vote
                                // Otherwise, hold the button for as long as the command specified,
                                // then release it for a frame before relinquishing control

                                
                                // Make sure that pressing this button would not complete an illegal combination
                                let mut ignore_button = false;
                                for &(ref constrained_button, ref constraining_buttons) in constraints.illegal_combinations.iter() {
                                    
                                    if constrained_button.as_ref() == name {
                                        let mut constrained_button_in_use_count = 0;
                                        for constraining_button in constraining_buttons.iter() {
                                            let index = get_button_guard_index_gcn(&constraining_button);
                                            if !{ button_guards[index].try_lock().is_ok() } {
                                                constrained_button_in_use_count = constrained_button_in_use_count+1;
                                            }
                                        }
                                        if constrained_button_in_use_count == constraining_buttons.len() {
                                            ignore_button = true;
                                        }
                                    }
                                }

                                if !ignore_button {
                                    let closure_controller = arc_controller_command_handler.clone();
                                    let closure_button_name = name.clone();
                                    
                                    let command_clone = command.clone();
                                    let button_guards_clone = button_guards.clone(); // Arc<Vec<Mutex<()>>>
                                    
                                    thread::spawn(move || {
                                        let button_guard_index = get_button_guard_index_gcn(&name);
                                        let button_guard_vec: &Vec<_> = button_guards_clone.deref();
                                        let button_guard = &button_guard_vec[button_guard_index];
                                        let lock_result = button_guard.try_lock();
                                        match lock_result {
                                            Ok(_) => {
                                                let command1 = virtc::Input::Button(closure_button_name.clone(), true);
                                                closure_controller.set_input(&command1);
                                                thread::sleep_ms(command_clone.duration.num_milliseconds() as u32);
                                                let command2 = virtc::Input::Button(closure_button_name.clone(), false);
                                                closure_controller.set_input(&command2);
                                            },
                                            _ => ()
                                        }
                                    }); 
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
                    let mut jx_sum: f32 = 0.0;
                    let mut jy_sum: f32 = 0.0;
                    let mut num_j_commands: u16 = 0;
                    let mut cx_sum: f32 = 0.0;
                    let mut cy_sum: f32 = 0.0;
                    let mut num_c_commands: u16 = 0;

                    // Loop over all commands
                    for command in active_joystick_commands.iter() {
                        match command.command {
                            virtc::Input::Joystick(ref name, direction, strength) => {
                                if name == "control_stick" {
                                    let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;
                                    jx_sum += direction_rad.cos() * strength;
                                    jy_sum += direction_rad.sin() * strength;
                                    num_j_commands += 1;
                                } else {
                                    let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;
                                    cx_sum += direction_rad.cos() * strength;
                                    cy_sum += direction_rad.sin() * strength;
                                    num_c_commands += 1;                                
                                }
                            },
                            _ => panic!("How did something besides a joystick or cstick command get here?")
                        }
                    }

                    let jx_avg = (jx_sum / num_j_commands as f32) as f32;
                    let jy_avg = (jy_sum / num_j_commands as f32) as f32;
                    let cx_avg = (cx_sum / num_c_commands as f32) as f32;
                    let cy_avg = (cy_sum / num_c_commands as f32) as f32;
                    
                    let mut j_direction_avg_rad = jy_avg.atan2(jx_avg);
                    if j_direction_avg_rad < 0.0 {
                        j_direction_avg_rad = j_direction_avg_rad + ((2.0*std::f32::consts::PI) as f32);
                    }
                    let mut c_direction_avg_rad = cy_avg.atan2(cx_avg);
                    if c_direction_avg_rad < 0.0 {
                        c_direction_avg_rad = c_direction_avg_rad + ((2.0*std::f32::consts::PI) as f32);
                    }
                    
                    let j_direction_avg = (j_direction_avg_rad * 180.0 / std::f32::consts::PI) as u16;
                    let mut j_strength_avg: f32 = jx_avg.abs() + jy_avg.abs(); //@todo lazy, but... what we want?
                    if j_strength_avg > 1.0 {
                        j_strength_avg = 1.0;
                    }
                    let c_direction_avg = (c_direction_avg_rad * 180.0 / std::f32::consts::PI) as u16;
                    let mut c_strength_avg: f32 = cx_avg.abs() + cy_avg.abs(); //@todo lazy, but... what we want?
                    if c_strength_avg > 1.0 {
                        c_strength_avg = 1.0;
                    }
                    
                    let j_command = if num_j_commands > 0 {
                        virtc::Input::Joystick(String::from("control_stick"), j_direction_avg, j_strength_avg)
                    } else {
                        virtc::Input::Joystick(String::from("control_stick"), 0, 0.0)
                    };
                    let c_command = if num_c_commands > 0 {
                        virtc::Input::Joystick(String::from("c_stick"), c_direction_avg, c_strength_avg)
                    } else {
                        virtc::Input::Joystick(String::from("c_stick"), 0, 0.0)
                    };
                    arc_controller_command_handler.set_input(&j_command);
                    arc_controller_command_handler.set_input(&c_command);
                } else {
                    let j_command = virtc::Input::Joystick(String::from("control_stick"), 0, 0.0);
                    let c_command = virtc::Input::Joystick(String::from("c_stick"), 0, 0.0);
                    arc_controller_command_handler.set_input(&j_command);
                    arc_controller_command_handler.set_input(&c_command);
                }

                thread::sleep_ms(1);
            }
        });
        
        let my_clone = arc_controller.clone();
        Ok( DemC { controller: arc_controller,
                   re: make_virtc_regex(my_clone.deref()).unwrap(),
                   tx_command: tx_command,
                   command_listener: command_listener } )
    }
}

fn make_virtc_joystick_regex<T>(controller: &T) -> Result<String, u8> where T: HasJoysticks {
    let mut regex_string = String::new();
    
    regex_string.push_str( r"(?P<joystick>" );
        // Optional: strength modifier
        regex_string.push_str( r"(" );
            regex_string.push_str( r"(?P<joystick_strength>" );
                regex_string.push_str( r"[:digit:]+" );
            regex_string.push_str( r")%\s*" );
        regex_string.push_str( r")?" );
            
        // Mandatory: joystick & direction
        regex_string.push_str( r"(?P<joystick_direction>" );
            for (joystick_name, _) in controller.get_joystick_map() {
                if joystick_name == "control_stick" {
                    regex_string.push_str( r"up|down|left|right|" );
                } else if joystick_name == "c_stick" {
                    regex_string.push_str( r"cup|cdown|cleft|cright|" );
                }
            }
            // remove last pipe
            let new_len = regex_string.len() - 1;
            regex_string.truncate(new_len);
        regex_string.push_str( r")" );
        
        // Optional: duration modifier
        regex_string.push_str( r"(\s*" );
            regex_string.push_str( r"(?P<joystick_duration>" );
                regex_string.push_str( r"[:digit:]+" );
            regex_string.push_str( r")" );
            regex_string.push_str( r"(?P<joystick_duration_units>" );
                regex_string.push_str( r"s|ms");
            regex_string.push_str( r")");
        regex_string.push_str( r")?" );
    regex_string.push_str( r")" );
    
    Ok(regex_string)
}

fn make_virtc_button_regex<T>(controller: &T) -> Result<String, u8> where T: HasButtons {
    let mut regex_string = String::new();
    
    regex_string.push_str( r"(?P<button>" );
        regex_string.push_str( r"(" );
            regex_string.push_str( r"(?P<button_name>" );
                for (button_name, _) in controller.get_button_map() {
                    regex_string.push_str(button_name);
                    regex_string.push_str(r"|");
                }
                // remove last pipe
                let new_len = regex_string.len() - 1;
                regex_string.truncate(new_len);
            regex_string.push_str( r")" );
            
            regex_string.push_str( r"(" );
                regex_string.push_str( r"\s*(?P<button_duration>" );
                    regex_string.push_str( r"[:digit:]+" );
                regex_string.push_str( r")" );
                
                regex_string.push_str( r"(?P<button_duration_units>" );
                    regex_string.push_str( r"s|ms" );
                regex_string.push_str( r")" );
            regex_string.push_str( r")?" );
        regex_string.push_str( r")" );
    regex_string.push_str( r")" );
    
    Ok(regex_string)
}

fn make_virtc_delay_regex() -> Result<String, u8> {
    let mut regex_string = String::new();

    regex_string.push_str( r"(?P<delay>" );
        regex_string.push_str( r"(" );
            regex_string.push_str( r"\(" );
            
            regex_string.push_str( r"(?P<delay_duration>" );
                regex_string.push_str( r"[:digit:]+" );
            regex_string.push_str( r")" );
            
            regex_string.push_str( r"(?P<delay_duration_units>" );
                regex_string.push_str( r"s|ms" );
            regex_string.push_str( r")" );
                
            regex_string.push_str( r"\)" );
        regex_string.push_str( r")" );
    
        regex_string.push_str( r"|" );
        
        regex_string.push_str( r"(?P<delay_hardcode>" );
            regex_string.push_str( r"[\+!\.]" );
        regex_string.push_str( r")" );
    regex_string.push_str( r")" );
    
    Ok(regex_string)
}

fn make_virtc_regex<T>(controller: &T) -> Result<Regex, u8> where T: HasButtons + HasJoysticks {
    // Dynamically generate regex that will match all of the virtual controller's inputs. LOL
    let mut regex_string = String::new();
    
    regex_string.push_str( r"\s*" );
    regex_string.push_str( r"(" );
        regex_string.push_str(make_virtc_joystick_regex(controller).unwrap().as_ref());
        regex_string.push_str( r"|" );
        regex_string.push_str(make_virtc_button_regex(controller).unwrap().as_ref());
        regex_string.push_str( r"|" );
        regex_string.push_str(make_virtc_delay_regex().unwrap().as_ref());
    regex_string.push_str( r")" );
    regex_string.push_str( r"\s*" );
    
    match Regex::new(&regex_string) {
        Ok(re) => Ok(re),
        Err(_) => Err(1)
    }
}