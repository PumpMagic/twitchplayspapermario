use std::sync::Mutex;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use time;
use time::{Timespec, Duration};

pub mod virtc;


fn get_button_guard_index(name: &str) -> usize {
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

#[derive(Clone)]
pub struct TimedInputCommand {
    pub start_time: Timespec,
    pub duration: Duration,
    pub command: virtc::Input
}

// A democratized virtual N64 controller
pub struct DemC {
    controller: Arc<virtc::Controller>,
    
    tx_command: mpsc::Sender<TimedInputCommand>,
    command_listener: thread::JoinHandle<()>
}

impl DemC {
    pub fn new_n64(vjoy_device_number: u32) -> Result<DemC, u8> {
        let controller_result = virtc::Controller::new(vjoy_device_number, virtc::get_n64_controller_hardware());
        let controller = match controller_result {
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
            let mut queued_commands: Vec<TimedInputCommand> = Vec::new();
            let mut active_joystick_commands: Vec<TimedInputCommand> = Vec::new();
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
                let mut queued_commands_fresh: Vec<TimedInputCommand> = Vec::new();
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

                                let button_guard_index = get_button_guard_index(&name);
                                
                                match button_guards[button_guard_index].try_lock() {
                                    Ok(_) => {
                                        let closure_controller = arc_controller_command_handler.clone();
                                        let closure_button_name = name.clone();

                                        let myclone = command.clone();
                                        thread::spawn(move || {
                                            let command1 = virtc::Input::Button(closure_button_name.clone(), true);
                                            closure_controller.change_input(&command1);
                                            thread::sleep_ms(myclone.duration.num_milliseconds() as u32);
                                            let command2 = virtc::Input::Button(closure_button_name.clone(), false);
                                            closure_controller.change_input(&command2);
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
                let mut active_joystick_commands_fresh: Vec<TimedInputCommand> = Vec::new();
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
                    arc_controller_command_handler.change_input(&x_command);
                    arc_controller_command_handler.change_input(&y_command);
                } else {
                    let x_command = virtc::Input::Axis(String::from("x"), 0.0);
                    let y_command = virtc::Input::Axis(String::from("y"), 0.0);
                    arc_controller_command_handler.change_input(&x_command);
                    arc_controller_command_handler.change_input(&y_command);
                }
                
                thread::sleep_ms(1);
            }
        });
        
        Ok( DemC { controller: arc_controller,
               tx_command: tx_command,
               command_listener: command_listener } )
    }
    
    
    pub fn add_command(&self, command: &TimedInputCommand) {
        self.tx_command.send(command.clone());
    }
}