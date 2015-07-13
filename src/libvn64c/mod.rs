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
const NUM_N64_BUTTONS: u8 = 14;

#[derive(Debug)]
pub enum VirtualN64ControllerButton {
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

// Default emulator button bindings. vJoy button indices are one-based
impl VirtualN64ControllerButton {
    fn get_vjoy_button_index(&self) -> libc::c_uchar {
        match *self {
            VirtualN64ControllerButton::A => 0x01,
            VirtualN64ControllerButton::B => 0x02,
            VirtualN64ControllerButton::Z => 0x03,
            VirtualN64ControllerButton::L => 0x04,
            VirtualN64ControllerButton::R => 0x05,
            VirtualN64ControllerButton::Start => 0x06,
            VirtualN64ControllerButton::Cup => 0x07,
            VirtualN64ControllerButton::Cdown => 0x08,
            VirtualN64ControllerButton::Cleft => 0x09,
            VirtualN64ControllerButton::Cright => 0x10,
            VirtualN64ControllerButton::Dup => 0x11,
            VirtualN64ControllerButton::Ddown => 0x12,
            VirtualN64ControllerButton::Dleft => 0x13,
            VirtualN64ControllerButton::Dright => 0x14
        }
    }
}

// Info about our virtual joystick
// Post-initialization, this should all be read-only
struct VirtualN64ControllerProps {
    vjoy_device_number: libc::c_uint,
    
    x_min: libc::c_long,
    x_max: libc::c_long,
    y_min: libc::c_long,
    y_max: libc::c_long,
}

// Last-set values
struct VirtualN64ControllerState {
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

pub struct VirtualN64Controller {
    props: VirtualN64ControllerProps,
    state: VirtualN64ControllerState,
}

impl Default for VirtualN64Controller {
    fn default() -> VirtualN64Controller {
        VirtualN64Controller {
            props: VirtualN64ControllerProps {
                vjoy_device_number: 0,
                x_min: 0,
                x_max: 0,
                y_min: 0,
                y_max: 0
            },
            state: VirtualN64ControllerState {
                x: 0,
                y: 0,
                a: 0,
                b: 0,
                z: 0,
                l: 0,
                r: 0,
                start: 0,
                cup: 0,
                cdown: 0,
                cleft: 0,
                cright: 0,
                dup: 0,
                ddown: 0,
                dleft: 0,
                dright: 0
            }
        }
    }
}

impl VirtualN64Controller {
    // Make a virtual N64 controller given a vJoy device number
    fn from_vjoy_device_number(vjoy_device_number: libc::c_uint) -> Result<(VirtualN64Controller), &'static str> {
        let mut controller = VirtualN64Controller { ..Default::default() };

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

    // Verify that a vJoy device can act as a virtual N64 controller, and if so, return a virtual N64 controller
    pub fn new(vjoy_device_number: u8) -> Result<VirtualN64Controller, String> {
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

        match VirtualN64Controller::from_vjoy_device_number(vjoy_device_number_native) {
            Err(msg) => Err(format!("{}", msg)),
            Ok(controller) => Ok(controller)
        }
    }

    // Set the virtual joystick, assuming that direction is in degrees and strength is a number 0-1 inclusive
    pub fn set_joystick(&mut self, direction: u16, strength: f32) -> Result<(), &'static str> {
        if direction > 359 {
            return Err("Direction must be between 0 and 359");
        }
        if strength < 0.0 || strength > 1.0 {
            return Err("Strength must be between 0 and 1");
        }

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
        self.state.x = x;

        match set_vjoystick_axis(self.props.vjoy_device_number, HID_JOYSTICK_Y, y) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set Y axis")
        }
        self.state.y = y;

        self.write_to_console();
        
        Ok(())
    }

    pub fn set_button(&mut self, button: VirtualN64ControllerButton, value: bool) -> Result<(), &'static str> {
        let valc = value as libc::c_int;

        match set_vjoystick_button(self.props.vjoy_device_number, button.get_vjoy_button_index(), valc) {
            Ok(_) => (),
            Err(_) => return Err("Unable to set virtual joystick button")
        }

        match button {
            VirtualN64ControllerButton::A => self.state.a = valc,
            VirtualN64ControllerButton::B => self.state.b = valc,
            VirtualN64ControllerButton::Z => self.state.z = valc,
            VirtualN64ControllerButton::L => self.state.l = valc,
            VirtualN64ControllerButton::R => self.state.r = valc,
            VirtualN64ControllerButton::Start => self.state.start = valc,
            VirtualN64ControllerButton::Cup => self.state.cup = valc,
            VirtualN64ControllerButton::Cdown => self.state.cdown = valc,
            VirtualN64ControllerButton::Cleft => self.state.cleft = valc,
            VirtualN64ControllerButton::Cright => self.state.cright = valc,
            VirtualN64ControllerButton::Dup => self.state.dup = valc,
            VirtualN64ControllerButton::Ddown => self.state.ddown = valc,
            VirtualN64ControllerButton::Dleft => self.state.dleft = valc,
            VirtualN64ControllerButton::Dright => self.state.dright = valc
        }

        self.write_to_console();
        
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