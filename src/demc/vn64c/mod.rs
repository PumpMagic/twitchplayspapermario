#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoy_rust;

extern crate std;

use std::collections::HashMap;


// A map of available axes to USB HID axes and buttons to vJoy button indices
pub struct ControllerHardware {
    axes: HashMap<String, u32>,
    buttons: HashMap<String, u8>
}

impl ControllerHardware {
    fn get_axis_hid(&self, name: &String) -> Option<u32> {
        match self.axes.get(name) {
            Some(hid) => Some(*hid),
            None => None
        }
    }

    fn get_button_index(&self, name: &String) -> Option<u8> {
        match self.buttons.get(name) {
            Some(index) => Some(*index),
            None => None
        }
    }
}

pub fn get_n64_controller_hardware() -> ControllerHardware {
    let mut axes = HashMap::new();
    axes.insert(String::from("x"), 0x30); // USB HID
    axes.insert(String::from("y"), 0x31); // USB HID

    let mut buttons = HashMap::new();
    buttons.insert(String::from("a"), 0x01);
    buttons.insert(String::from("b"), 0x02);
    buttons.insert(String::from("z"), 0x03);
    buttons.insert(String::from("l"), 0x04);
    buttons.insert(String::from("r"), 0x05);
    buttons.insert(String::from("start"), 0x06);
    buttons.insert(String::from("cup"), 0x07);
    buttons.insert(String::from("cdown"), 0x08);
    buttons.insert(String::from("cleft"), 0x09);
    buttons.insert(String::from("cright"), 0x0a);
    buttons.insert(String::from("dup"), 0x0b);
    buttons.insert(String::from("ddown"), 0x0c);
    buttons.insert(String::from("dleft"), 0x0d);
    buttons.insert(String::from("dright"), 0x0e);

    ControllerHardware { axes: axes, buttons: buttons }
}

pub fn get_gcn_controller_hardware() -> ControllerHardware {
    let mut axes = HashMap::new();
    axes.insert(String::from("jx"), 0x30); // USB HID
    axes.insert(String::from("jy"), 0x31); // USB HID
    axes.insert(String::from("cx"), 0x32); // USB HID???
    axes.insert(String::from("cy"), 0x33); // USB HID???

    let mut buttons = HashMap::new();
    buttons.insert(String::from("a"), 0x01);
    buttons.insert(String::from("b"), 0x02);
    buttons.insert(String::from("x"), 0x03);
    buttons.insert(String::from("y"), 0x04);
    buttons.insert(String::from("z"), 0x05);
    buttons.insert(String::from("l"), 0x06);
    buttons.insert(String::from("r"), 0x07);
    buttons.insert(String::from("dup"), 0x08);
    buttons.insert(String::from("ddown"), 0x09);
    buttons.insert(String::from("dleft"), 0x0a);
    buttons.insert(String::from("dright"), 0x0b);

    ControllerHardware { axes: axes, buttons: buttons }
}

// Properties (non-input-state characteristics) about a virtual N64 controller
// Post-initialization, this should all be read-only
struct Props {
    vjoy_device_number: u32,

    hardware: ControllerHardware,

    axis_mins: HashMap<u32, i64>,
    axis_maxes: HashMap<u32, i64>,
}

// Comprehensive status of a virtual N64 controller's inputs
struct State {
    axes: HashMap<u32, i64>,
    buttons: HashMap<u8, i32>
}

pub struct Controller {
    props: Props,
    state: State
}

#[derive(Clone)]
pub enum Input {
    Axis(String, f32),
    Button(String, bool)
}

/*
impl Input {
    //@todo implement
    fn is_compatible_with(&self, ch: &ControllerHardware) -> bool {
        match *self {
            InputCommand::Axis{ name, percent} => {
                if percent < 0.0 || percent> 1.0 {
                    return false;
                }
                return true
            },
            InputCommand::Button { name: _, value: _ } => { return true; }
        }
    }
}
*/

impl Controller {
    // Verify that a vJoy device can act as a virtual N64 controller, and if so, return a virtual
    // N64 controller
    pub fn new(vjoy_device_number: u32, hardware: ControllerHardware) -> Result<Controller, String> {
        let vjoy_device_number_native = vjoy_device_number;

        match vjoy_rust::get_vjoy_is_enabled() {
            Ok(val) => {
                if val == false {
                    return Err(format!("vJoy isn't enabled. Have you installed vJoy?"));
                }
            },
            Err(err) => return Err(format!("Unable to check if vJoy is enabled. Have you installed vJoy?"))
        }

        match verify_vjoystick_hardware(vjoy_device_number_native, &hardware) {
            Ok(()) => (),
            Err(err) => return Err(format!("Virtual joystick {} can't act as an N64 controller: {}.\
                                            You may need to reconfigure your vJoy device",
                                           vjoy_device_number, err))
        }

        match vjoy_rust::claim_vjoystick(vjoy_device_number_native) {
            Err(msg) => return Err(format!("{}", msg)),
            _ => ()
        }

        match vjoy_rust::reset_vjoystick(vjoy_device_number_native) {
            Err(msg) => return Err(format!("{}", msg)),
            _ => ()
        }

        match Controller::from_hardware(vjoy_device_number_native, hardware) {
            Err(msg) => Err(format!("{}", msg)),
            Ok(controller) => Ok(controller)
        }
    }

    // Make and initialize a virtual N64 controller struct given the device number
    fn from_hardware(vjoy_device_number: u32, hardware: ControllerHardware) -> Result<Controller, &'static str> {
        let mut props = Props {
            vjoy_device_number: vjoy_device_number,

            hardware: hardware,

            axis_mins: HashMap::new(),
            axis_maxes: HashMap::new()
        };
        let mut state = State {
            axes: HashMap::new(),
            buttons: HashMap::new()
        };

        // Get and capture vjoystick min and max; set joystick values to neutral
        for (_, hid) in props.hardware.axes.iter() {
            let min = match vjoy_rust::get_vjoystick_axis_min(vjoy_device_number, *hid) {
                Ok(min) => min,
                Err(msg) => return Err(msg)
            };
            let max = match vjoy_rust::get_vjoystick_axis_max(vjoy_device_number, *hid) {
                Ok(max) => max,
                Err(msg) => return Err(msg)
            };

            props.axis_mins.insert(*hid, min);
            props.axis_maxes.insert(*hid, max);
            state.axes.insert(*hid, (max-min)/2);
        }

        for (_, index) in props.hardware.buttons.iter() {
            state.buttons.insert(*index, 0);
        }

        Ok(Controller { props: props, state: state })
    }

    pub fn change_input(&self, input: &Input) -> Result<(), &'static str> {
        /* match command.is_compatible_with(self.props.hardware) {
            true => (),
            false => { return Err("Input command is invalid. Valid directions: [0, 359]; valid strengths: [0.0, 1.0]"); }
        } */

        match input.clone() {
            Input::Axis(name, strength) => self.change_joystick(&name, strength),
            Input::Button(name, value) => self.change_button(&name, value)
        }
    }
    
    pub fn change_joystick(&self, name: &String, strength: f32) -> Result<(), &'static str> {
        let hid = self.props.hardware.get_axis_hid(name).unwrap();

        let mid: i64 = ((self.props.axis_maxes.get(&hid).unwrap() - self.props.axis_mins.get(&hid).unwrap())/2) as i64;
        let val = mid + (strength * (mid as f32)) as i64;

        match vjoy_rust::set_vjoystick_axis(self.props.vjoy_device_number, hid, val) {
            Ok(_) => Ok(()),
            Err(_) => Err("Unable to set axis")
        }
    }
    
    pub fn change_button(&self, name: &String, value: bool) -> Result<(), &'static str> {
        let index = self.props.hardware.get_button_index(name).unwrap();

        let valc = value as i32;

        match vjoy_rust::set_vjoystick_button(self.props.vjoy_device_number, index, valc) {
            Ok(_) => Ok(()),
            Err(_) => Err("Unable to set virtual joystick button")
        }
    }

    //@todo implement
    pub fn write_to_console(&self) {
    }
}


//@todo implement
fn verify_vjoystick_hardware(index: u32, hardware: &ControllerHardware) -> Result<(), String> {
    match vjoy_rust::get_vjoystick_axis_exists(index, 0x30) {
        Ok(exists) => {
            if exists == false {
                return Err(format!("No X axis"));
            }
        },
        Err(()) => return Err(format!("Unable to check for X axis"))
    }

    match vjoy_rust::get_vjoystick_axis_exists(index, 0x31) {
        Ok(exists) => {
            if exists == false {
                return Err(format!("No Y axis"));
            }
        },
        Err(()) => return Err(format!("Unable to check for Y axis"))
    }

    match vjoy_rust::get_vjoystick_button_count(index) {
        Ok(buttons) => {
            if buttons < 14 {
                return Err(format!("Less than {} buttons", 14));
            }
        },
        Err(()) => return Err(format!("Unable to get button count"))
    }

    Ok(())
}
