#![allow(dead_code)]
#![allow(unused_variables)]

// Dependencies
mod vjoyinterface;
extern crate std;
extern crate libc;

// Control constants - what magic numbers do we need to play with each virtual device input?
// USB HID joystick constants. We assume our emulator is configured to map virtual joystick to N64 joystick
const HID_JOYSTICK_X: libc::c_uint = 0x30;
const HID_JOYSTICK_Y: libc::c_uint = 0x31;

// Number of digital inputs on a standard N64 controller
pub const NUM_N64_BUTTONS: u8 = 14;

// Enumeration of digital inputs on an N64 controller
#[derive(Debug, Clone, Copy)]
pub enum ButtonName {
    A,
    B,
    Z,
    L,
    R,
    Start,
    Cup,
    Cdown,
    Cleft,
    Cright,
    Dup,
    Ddown,
    Dleft,
    Dright
}

impl ButtonName {
    // Emulator button bindings - what emulator button each virtual joystick button is connected to
    // Note that vJoy button indices are one-based
    fn get_vjoy_button_index(&self) -> libc::c_uchar {
        match *self {
            ButtonName::A => 0x01,
            ButtonName::B => 0x02,
            ButtonName::Z => 0x03,
            ButtonName::L => 0x04,
            ButtonName::R => 0x05,
            ButtonName::Start => 0x06,
            ButtonName::Cup => 0x07,
            ButtonName::Cdown => 0x08,
            ButtonName::Cleft => 0x09,
            ButtonName::Cright => 0x10,
            ButtonName::Dup => 0x11,
            ButtonName::Ddown => 0x12,
            ButtonName::Dleft => 0x13,
            ButtonName::Dright => 0x14
        }
    }
}

// Properties (non-input-state characteristics) about a virtual N64 controller
// Post-initialization, this should all be read-only
#[derive(Default)]
struct Props {
    vjoy_device_number: libc::c_uint,
    
    x_min: libc::c_long,
    x_max: libc::c_long,
    y_min: libc::c_long,
    y_max: libc::c_long,
}

// Comprehensive status of a virtual N64 controller's inputs
#[derive(Default)]
struct State {
    x: libc::c_long,
    y: libc::c_long,
    a: libc::c_int,
    b: libc::c_int,
    z: libc::c_int,
    l: libc::c_int,
    r: libc::c_int,
    start: libc::c_int,
    cup: libc::c_int,
    cdown: libc::c_int,
    cleft: libc::c_int,
    cright: libc::c_int,
    dup: libc::c_int,
    ddown: libc::c_int,
    dleft: libc::c_int,
    dright: libc::c_int,
}

#[derive(Default)]
pub struct Controller {
    props: Props,
    state: State
}

pub enum InputCommand {
    // direction is in degrees - strength is a number between 0 and 1
    Joystick { direction: u16, strength: f32 },
    Button { name: ButtonName, value: bool }
}

impl InputCommand {
    fn is_valid(&self) -> bool {
        match *self {
            InputCommand::Joystick { direction, strength } => {
                if direction > 359 {
                    return false;
                } else if strength < 0.0 || strength > 1.0 {
                    return false;
                }
                return true
            },
            InputCommand::Button { name: _, value: _ } => { return true; }
        }
    }
}

impl Controller {
    // Verify that a vJoy device can act as a virtual N64 controller, and if so, return a virtual
    // N64 controller
    pub fn new(vjoy_device_number: u8) -> Result<Controller, String> {
        let vjoy_device_number_native = vjoy_device_number as libc::c_uint;

        match get_vjoy_is_enabled() {
            Ok(val) => {
                if val == false {
                    return Err(format!("vJoy isn't enabled. Have you installed vJoy?"));
                }
            },
            Err(err) => return Err(format!("Unable to check if vJoy is enabled. Have you installed vJoy?"))
        }

        match verify_vjoystick_as_n64(vjoy_device_number_native) {
            Ok(()) => (),
            Err(err) => return Err(format!("Virtual joystick {} can't act as an N64 controller: {}.\
                                            You may need to reconfigure your vJoy device",
                                           vjoy_device_number, err))
        }

        match claim_vjoystick(vjoy_device_number_native) {
            Err(msg) => return Err(format!("{}", msg)),
            _ => ()
        }

        match reset_vjoystick(vjoy_device_number_native) {
            Err(msg) => return Err(format!("{}", msg)),
            _ => ()
        }

        match Controller::from_vjoy_device_number(vjoy_device_number_native) {
            Err(msg) => Err(format!("{}", msg)),
            Ok(controller) => Ok(controller)
        }
    }

    // Make and initialize a virtual N64 controller struct given the device number
    fn from_vjoy_device_number(vjoy_device_number: libc::c_uint) -> Result<Controller, &'static str> {
        let mut controller = Controller { ..Default::default() };

        controller.props.vjoy_device_number = vjoy_device_number;

        // Get and capture vjoystick min and max
        match get_vjoystick_axis_min(vjoy_device_number, HID_JOYSTICK_X) {
            Ok(min) => controller.props.x_min = min,
            Err(msg) => return Err(msg)
        }
        match get_vjoystick_axis_max(vjoy_device_number, HID_JOYSTICK_X) {
            Ok(max) => controller.props.x_max = max,
            Err(msg) => return Err(msg)
        }
        match get_vjoystick_axis_min(vjoy_device_number, HID_JOYSTICK_Y) {
            Ok(min) => controller.props.y_min = min,
            Err(msg) => return Err(msg)
        }
        match get_vjoystick_axis_max(vjoy_device_number, HID_JOYSTICK_Y) {
            Ok(max) => controller.props.y_max = max,
            Err(msg) => return Err(msg)
        }

        // Set joystick values to neutral
        controller.state.x = (controller.props.x_max - controller.props.x_min) / 2;
        controller.state.y = (controller.props.y_max - controller.props.y_min) / 2;

        Ok(controller)
    }
    
    pub fn change_input(&self, command: &InputCommand) -> Result<(), &'static str> {
        match command.is_valid() {
            true => (),
            false => { return Err("Input command is invalid. Valid directions: [0, 359]; valid strengths: [0.0, 1.0]"); }
        }
        
        match *command {
            InputCommand::Joystick { direction, strength } => {
                self.change_joystick(direction, strength)
            },
            InputCommand::Button { name, value } => {
                self.change_button(name, value)
            }
        }
    }
    
    fn change_joystick(&self, direction: u16, strength: f32) -> Result<(), &'static str> {
        // Convert direction from degrees to radians
        let direction_rad: f32 = (direction as f32) * std::f32::consts::PI / 180.0;

        let x_strength = direction_rad.cos() * strength;
        let y_strength = direction_rad.sin() * strength;

        let x_mid: libc::c_long = ((self.props.x_max - self.props.x_min)/2) as libc::c_long;
        let y_mid: libc::c_long = ((self.props.y_max - self.props.y_min)/2) as libc::c_long;

        let x = x_mid + (x_strength * (x_mid as f32)) as libc::c_long;
        let y = y_mid + (y_strength * (y_mid as f32)) as libc::c_long;

        match set_vjoystick_axis(self.props.vjoy_device_number, HID_JOYSTICK_X, x) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set X axis")
        }
        //self.state.x = x;

        match set_vjoystick_axis(self.props.vjoy_device_number, HID_JOYSTICK_Y, y) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set Y axis")
        }
        //self.state.y = y;
        
        Ok(())
    }
    
    fn change_button(&self, name: ButtonName, value: bool) -> Result<(), &'static str> {
        let valc = value as libc::c_int;

        match set_vjoystick_button(self.props.vjoy_device_number, name.get_vjoy_button_index(), valc) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set virtual joystick button")
        }

        /*
        match button {
            ButtonName::A => self.state.a = valc,
            ButtonName::B => self.state.b = valc,
            ButtonName::Z => self.state.z = valc,
            ButtonName::L => self.state.l = valc,
            ButtonName::R => self.state.r = valc,
            ButtonName::Start => self.state.start = valc,
            ButtonName::Cup => self.state.cup = valc,
            ButtonName::Cdown => self.state.cdown = valc,
            ButtonName::Cleft => self.state.cleft = valc,
            ButtonName::Cright => self.state.cright = valc,
            ButtonName::Dup => self.state.dup = valc,
            ButtonName::Ddown => self.state.ddown = valc,
            ButtonName::Dleft => self.state.dleft = valc,
            ButtonName::Dright => self.state.dright = valc
        }
        */
        
        Ok(())
    }
        
    pub fn write_to_console(&self) {
        println!("X: {} Y: {} A: {} B: {} Z: {} L: {} R: {} S: {}", self.state.x, self.state.y, self.state.a, self.state.b, self.state.z, self.state.l, self.state.r, self.state.start);
        //println!("CU: {} CD: {} CL: {} CR: {}", self.state.cup, self.state.cdown, self.state.cleft, self.state.cright);
        //println!("DU: {} DD: {} DL: {} DR: {}", self.state.dup, self.state.ddown, self.state.dleft, self.state.dright);
    }
}

// vJoy wrapper functions
fn get_vjoy_is_enabled() -> Result<bool, ()> {
    unsafe {
        let vjoy_enabled = vjoyinterface::vJoyEnabled();
        if vjoy_enabled == 0 {
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

fn get_vjoystick_axis_exists(index: libc::c_uint, axis: libc::c_uint) -> Result<bool, ()> {
    unsafe {
        let axis_exists = vjoyinterface::GetVJDAxisExist(index, axis);
        if axis_exists == 0 {
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

fn get_vjoystick_axis_min(index: libc::c_uint, axis: libc::c_uint) -> Result<libc::c_long, &'static str> {
    unsafe {
        let mut min: libc::c_long = 0;
        let min_raw_pointer = &mut min as *mut libc::c_long;
        let min_result = vjoyinterface::GetVJDAxisMin(index, axis, min_raw_pointer);
        if min_result == 0 {
            Err("Unable to get axis minimum")
        } else {
            Ok(min)
        }
    }
}

fn get_vjoystick_axis_max(index: libc::c_uint, axis: libc::c_uint) -> Result<libc::c_long, &'static str> {
    unsafe {
        let mut max: libc::c_long = 0;
        let max_raw_pointer = &mut max as *mut libc::c_long;
        let max_result = vjoyinterface::GetVJDAxisMax(index, axis, max_raw_pointer);
        if max_result == 0 {
            Err("Unable to get axis maximum: does the axis exist?")
        } else {
            Ok(max)
        }
    }
}

fn get_vjoystick_button_count(index: libc::c_uint) -> Result<u8, ()> {
    unsafe {
        let num_buttons = vjoyinterface::GetVJDButtonNumber(index);

        Ok(num_buttons as u8)
    }
}

fn get_vjoystick_status(index: libc::c_uint) -> vjoyinterface::Enum_VjdStat {
    unsafe {
        let joystick_status = vjoyinterface::GetVJDStatus(index);

        joystick_status
    }
}

fn claim_vjoystick(index: libc::c_uint) -> Result<(), &'static str> {
    unsafe {
        let joystick_status = get_vjoystick_status(index);
        if joystick_status == vjoyinterface::VJD_STAT_FREE {
            // Try to claim it
            let acquire_vjd_result = vjoyinterface::AcquireVJD(index);
            if acquire_vjd_result == 0 {
                return Err("Virtual joystick is available, but unable to acquire it");
            } else {
                return Ok(());
            }
        } else if joystick_status == vjoyinterface::VJD_STAT_OWN {
            // We've already claimed it
            return Ok(());
        }
    }

    Err("Virtual joystick is owned by someone else, missing, or in unknown state")
}

fn reset_vjoystick(index: libc::c_uint) -> Result<(), &'static str> {
    unsafe {
        let reset_result = vjoyinterface::ResetVJD(index);
        if reset_result == 0 {
            return Err("vJoy reset function returned failure");
        }
    }

    Ok(())
}

fn set_vjoystick_axis(index: libc::c_uint, axis: libc::c_uint, value: libc::c_long) -> Result<(), ()> {
    unsafe {
        let set_x_result = vjoyinterface::SetAxis(value, index, axis);
        if set_x_result == 0 {
            return Err(());
        }
    }

    Ok(())
}

fn set_vjoystick_button(index: libc::c_uint, button: libc::c_uchar, value: libc::c_int) -> Result<(), ()> {
    unsafe {
        let set_result = vjoyinterface::SetBtn(value, index, button);
        if set_result == 0 {
            return Err(());
        }
    }

    Ok(())
}

// Verify that a vJoy device has the controls we need to treat it like an N64 controller
// If this fails, the vJoy device should be configured manually using vJoy's supplied configuration tool
fn verify_vjoystick_as_n64(index: libc::c_uint) -> Result<(), String> {
    match get_vjoystick_axis_exists(index, HID_JOYSTICK_X) {
        Ok(exists) => {
            if exists == false {
                return Err(format!("No X axis"));
            }
        },
        Err(()) => return Err(format!("Unable to check for X axis"))
    }

    match get_vjoystick_axis_exists(index, HID_JOYSTICK_Y) {
        Ok(exists) => {
            if exists == false {
                return Err(format!("No Y axis"));
            }
        },
        Err(()) => return Err(format!("Unable to check for Y axis"))
    }

    match get_vjoystick_button_count(index) {
        Ok(buttons) => {
            if buttons < NUM_N64_BUTTONS {
                return Err(format!("Less than {} buttons", NUM_N64_BUTTONS));
            }
        },
        Err(()) => return Err(format!("Unable to get button count"))
    }

    Ok(())
}